//! Campaign supervisor — the orchestrator driven tick-by-tick, in-process.
//!
//! Submodule of the `control_api` bin (lives under `src/bin/control_api/` so Cargo
//! does not auto-discover it as a separate binary target).
//!
//! Architecture decision AD-1: the control plane OWNS a `ResearchOrchestrator`
//! and calls `tick()` in a supervised loop, rather than shelling out to
//! `research_platform`. That buys what a spawned CLI cannot give:
//!
//!   * **pause** between ticks (no signals, no orphaned children)
//!   * **step** exactly one tick — a debugger for science
//!   * structured status after every tick instead of scraped stdout
//!   * cancellation as a flag check, with state persisted by the campaign itself
//!
//! The Runtime is blocking and CPU-bound, so the loop lives on a dedicated
//! `std::thread` and communicates with the async HTTP layer through a channel
//! plus a mutex-guarded snapshot. Only ONE campaign runs at a time: the campaign
//! directory is an append-only store and two writers would corrupt it.
//!
//! Nothing here fabricates progress. If a tick fails, the error is recorded and
//! the loop stops; the state machine never reports work it did not do.

use ising_engine::engine_v2::ai_scientist::{
    CampaignConfig, OrchestratorConfig, ResearchOrchestrator, RuntimeExecutor,
};
use ising_engine::engine_v2::evolution::Evolver;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::registry::OperatorRegistry;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};

/// Lifecycle of a supervised campaign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    /// Nothing has been started.
    Idle,
    Running,
    Paused,
    /// Budget reached, or a stop was requested — terminal.
    Finished,
    /// A tick returned an error — terminal, and the error is kept.
    Failed,
}

/// Commands the HTTP layer sends to the loop thread.
enum Cmd {
    Pause,
    Resume,
    /// Run exactly one tick, then return to Paused.
    Step,
    Stop,
}

/// What a caller sees. Every field is measured, never estimated.
#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub phase: Phase,
    pub dir: String,
    /// Ticks actually completed.
    pub ticks_done: usize,
    /// Requested cap (0 = until stopped).
    pub max_ticks: usize,
    pub instances: Vec<String>,
    /// Instance the next tick will study (round-robin unless planner-driven).
    pub next_instance: Option<String>,
    pub planner_driven: bool,
    pub executive_driven: bool,
    pub started_unix: Option<u64>,
    pub last_error: Option<String>,
    /// Newest-first tick log, bounded.
    pub events: Vec<TickEvent>,
}

/// One completed tick, flattened for transport.
#[derive(Debug, Clone, Serialize)]
pub struct TickEvent {
    pub tick: usize,
    pub instance: String,
    pub experiments_before: usize,
    pub experiments_after: usize,
    pub best_score: f64,
    pub best_schedule: Vec<String>,
    pub baseline: f64,
    pub theories_published: usize,
    pub concepts: Vec<String>,
    pub health: String,
    pub recall: String,
    pub directives: Vec<String>,
    pub wall_ms: f64,
}

/// Parameters accepted when starting a campaign.
#[derive(Debug, Clone, Deserialize)]
pub struct StartReq {
    /// Instance file names (resolved under the instances dir) — e.g. ["G1","G22"].
    pub instances: Vec<String>,
    /// 0 = run until stopped.
    #[serde(default)]
    pub max_ticks: usize,
    #[serde(default)]
    pub generations: Option<usize>,
    #[serde(default)]
    pub planner_driven: bool,
    #[serde(default)]
    pub executive_driven: bool,
    #[serde(default)]
    pub shared_knowledge: bool,
    #[serde(default)]
    pub curiosity_lambda: Option<f64>,
    #[serde(default)]
    pub seed: Option<u64>,
    /// Start paused, so the caller can step tick-by-tick.
    #[serde(default)]
    pub start_paused: bool,
}

const MAX_EVENTS: usize = 200;

/// Shared handle held by the HTTP layer.
pub struct Supervisor {
    dir: PathBuf,
    instances_dir: PathBuf,
    inner: Mutex<Option<Running>>,
    status: Arc<Mutex<Status>>,
}

struct Running {
    tx: Sender<Cmd>,
    handle: std::thread::JoinHandle<()>,
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Supervisor {
    pub fn new(dir: PathBuf, instances_dir: PathBuf) -> Self {
        let status = Status {
            phase: Phase::Idle,
            dir: dir.display().to_string(),
            ticks_done: 0,
            max_ticks: 0,
            instances: Vec::new(),
            next_instance: None,
            planner_driven: false,
            executive_driven: false,
            started_unix: None,
            last_error: None,
            events: Vec::new(),
        };
        Self {
            dir,
            instances_dir,
            inner: Mutex::new(None),
            status: Arc::new(Mutex::new(status)),
        }
    }

    pub fn status(&self) -> Status {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|e| e.into_inner().clone())
    }

    fn is_active(&self) -> bool {
        matches!(self.status().phase, Phase::Running | Phase::Paused)
    }

    /// Load the requested instances into IRs up front, so a bad path fails the
    /// start request instead of dying three ticks in.
    fn load_instances(&self, names: &[String]) -> Result<Vec<(String, ProblemIR)>, String> {
        if names.is_empty() {
            return Err("no instances given".into());
        }
        let mut out = Vec::with_capacity(names.len());
        for n in names {
            if n.contains('/') || n.contains("..") {
                return Err(format!("invalid instance name: {n}"));
            }
            let p = self.instances_dir.join(n);
            let text = std::fs::read_to_string(&p)
                .map_err(|e| format!("cannot read {}: {e}", p.display()))?;
            let ir = rudy_maxcut_ir(&text).map_err(|e| format!("{n}: {e}"))?;
            out.push((n.clone(), ir));
        }
        Ok(out)
    }

    pub fn start(&self, req: StartReq) -> Result<Status, String> {
        if self.is_active() {
            return Err(
                "a campaign is already running — stop it first (one writer per dir)".into(),
            );
        }
        let instances = self.load_instances(&req.instances)?;
        let dir = self.dir.clone();

        // Fail the request (not a background thread) if the dir cannot be opened.
        let mut orch = ResearchOrchestrator::open(&dir, 500)
            .map_err(|e| format!("cannot open platform dir {}: {e}", dir.display()))?;

        let cfg = OrchestratorConfig {
            campaign: CampaignConfig {
                instance_id: String::new(), // set per tick
                generations: req.generations.unwrap_or(1),
                shared_knowledge: req.shared_knowledge,
                curiosity_lambda: req.curiosity_lambda.unwrap_or(0.0),
                base_seed: req.seed.unwrap_or(1),
                ..Default::default()
            },
            planner_driven: req.planner_driven,
            executive_driven: req.executive_driven,
            ..Default::default()
        };

        {
            let mut s = self.status.lock().map_err(|e| e.to_string())?;
            *s = Status {
                phase: if req.start_paused {
                    Phase::Paused
                } else {
                    Phase::Running
                },
                dir: dir.display().to_string(),
                ticks_done: 0,
                max_ticks: req.max_ticks,
                instances: req.instances.clone(),
                next_instance: req.instances.first().cloned(),
                planner_driven: req.planner_driven,
                executive_driven: req.executive_driven,
                started_unix: Some(now_unix()),
                last_error: None,
                events: Vec::new(),
            };
        }

        let (tx, rx) = mpsc::channel::<Cmd>();
        let status = Arc::clone(&self.status);
        let max_ticks = req.max_ticks;
        let handle = std::thread::Builder::new()
            .name("campaign-supervisor".into())
            .spawn(move || {
                run_loop(&mut orch, instances, cfg, max_ticks, rx, status);
            })
            .map_err(|e| format!("cannot spawn supervisor thread: {e}"))?;

        *self.inner.lock().map_err(|e| e.to_string())? = Some(Running { tx, handle });
        Ok(self.status())
    }

    fn send(&self, cmd: Cmd, verb: &str) -> Result<Status, String> {
        let guard = self.inner.lock().map_err(|e| e.to_string())?;
        match guard.as_ref() {
            Some(r) => {
                r.tx.send(cmd)
                    .map_err(|_| "supervisor thread has exited".to_string())?
            }
            None => return Err(format!("no campaign to {verb}")),
        }
        Ok(self.status())
    }

    pub fn pause(&self) -> Result<Status, String> {
        self.send(Cmd::Pause, "pause")
    }
    pub fn resume(&self) -> Result<Status, String> {
        self.send(Cmd::Resume, "resume")
    }
    pub fn step(&self) -> Result<Status, String> {
        self.send(Cmd::Step, "step")
    }

    /// Request a stop and join the thread, so the campaign's state is fully
    /// persisted before the caller is told it stopped.
    pub fn stop(&self) -> Result<Status, String> {
        let running = self.inner.lock().map_err(|e| e.to_string())?.take();
        match running {
            Some(r) => {
                let _ = r.tx.send(Cmd::Stop);
                let _ = r.handle.join();
                Ok(self.status())
            }
            None => Err("no campaign to stop".into()),
        }
    }
}

/// The supervised loop. Runs on its own thread; the Runtime blocks inside `tick`.
fn run_loop(
    orch: &mut ResearchOrchestrator,
    instances: Vec<(String, ProblemIR)>,
    mut cfg: OrchestratorConfig,
    max_ticks: usize,
    rx: Receiver<Cmd>,
    status: Arc<Mutex<Status>>,
) {
    let registry = OperatorRegistry::standard();
    let evolver = Evolver::new(Default::default());
    let executor = RuntimeExecutor::auto();

    let set = |f: &mut dyn FnMut(&mut Status)| {
        if let Ok(mut s) = status.lock() {
            f(&mut s);
        }
    };

    let mut idx = 0usize;
    let mut ticks = 0usize;
    // One pending step when paused.
    let mut step_once = false;

    loop {
        // ── drain commands ─────────────────────────────────────────────
        let paused_now = status
            .lock()
            .map(|s| s.phase == Phase::Paused)
            .unwrap_or(false);
        loop {
            match rx.try_recv() {
                Ok(Cmd::Pause) => set(&mut |s| {
                    if s.phase == Phase::Running {
                        s.phase = Phase::Paused;
                    }
                }),
                Ok(Cmd::Resume) => set(&mut |s| {
                    if s.phase == Phase::Paused {
                        s.phase = Phase::Running;
                    }
                }),
                Ok(Cmd::Step) => step_once = true,
                Ok(Cmd::Stop) => {
                    set(&mut |s| s.phase = Phase::Finished);
                    return;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    set(&mut |s| s.phase = Phase::Finished);
                    return;
                }
            }
        }

        let phase = status.lock().map(|s| s.phase).unwrap_or(Phase::Finished);
        match phase {
            Phase::Paused if !step_once => {
                // Idle politely; a command will wake the next iteration.
                std::thread::sleep(std::time::Duration::from_millis(120));
                continue;
            }
            Phase::Running | Phase::Paused => {}
            _ => return,
        }
        let _ = paused_now;

        if max_ticks > 0 && ticks >= max_ticks {
            set(&mut |s| s.phase = Phase::Finished);
            return;
        }

        // ── one tick ───────────────────────────────────────────────────
        let (name, ir) = &instances[idx % instances.len()];
        cfg.campaign.instance_id = name.clone();
        let started = std::time::Instant::now();
        let result = orch.tick(ir, &registry, &evolver, &executor, &cfg);
        let wall_ms = started.elapsed().as_secs_f64() * 1000.0;

        match result {
            Ok(rep) => {
                ticks += 1;
                idx += 1;
                let ev = TickEvent {
                    tick: rep.tick,
                    instance: rep.instance.clone(),
                    experiments_before: rep.campaign.experiments_before,
                    experiments_after: rep.campaign.experiments_after,
                    best_score: rep.campaign.best_score,
                    best_schedule: rep.campaign.best_schedule.clone(),
                    baseline: rep.campaign.baseline,
                    theories_published: rep.theories_published,
                    concepts: rep.concepts.clone(),
                    health: rep.health.clone(),
                    recall: rep.recall.chars().take(280).collect(),
                    directives: rep.directives.clone(),
                    wall_ms,
                };
                let next = instances[idx % instances.len()].0.clone();
                set(&mut |s| {
                    s.ticks_done = ticks;
                    s.next_instance = Some(next.clone());
                    s.events.insert(0, ev.clone());
                    if s.events.len() > MAX_EVENTS {
                        s.events.truncate(MAX_EVENTS);
                    }
                    // A step returns control to the caller.
                    if step_once {
                        s.phase = Phase::Paused;
                    }
                });
                step_once = false;
            }
            Err(e) => {
                let msg = e.to_string();
                set(&mut |s| {
                    s.phase = Phase::Failed;
                    s.last_error = Some(msg.clone());
                });
                return;
            }
        }
    }
}

/// A `Supervisor` is shared across async handlers.
pub type SharedSupervisor = Arc<Supervisor>;
