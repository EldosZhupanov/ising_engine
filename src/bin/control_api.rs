//! Control plane (read + replay) — the bridge the browser laboratory needs.
//!
//! The website is a static Observatory: it renders a build-time snapshot of the
//! append-only record and cannot reach anything the browser can't fetch. This
//! binary closes three specific gaps, and nothing more:
//!
//!   1. **Liveness.** `/api/db/stats` and `/api/artifacts` read the artifacts as
//!      they are *now*, so the interface stops being a photograph.
//!   2. **System telemetry.** `/api/system` exposes CPU/RAM (and GPU when
//!      `nvidia-smi` exists) — the browser cannot shell out.
//!   3. **Replay.** `/api/replay` re-executes a recorded experiment from its
//!      seed and reports whether the score comes back *identical*. This is only
//!      possible because of ADR-0004 determinism, and it is the capability that
//!      turns 109k rows from a log into a verifiable corpus.
//!
//!   4. **Campaign lifecycle.** `/api/campaign/*` owns a `ResearchOrchestrator`
//!      and drives `tick()` on a supervised thread (architecture AD-1), which is
//!      what makes pause / resume / **step** possible at all — a spawned CLI
//!      could only be killed. One campaign at a time: the campaign directory is
//!      an append-only store and two writers would corrupt it.
//!
//! Campaign ticks DO append to the record (that is their purpose); everything
//! else here is read-only and runs the read-only Runtime.
//!
//! Run:  cargo run --release --bin control_api -- [--dir experiments/platform_gset] [--port 7878]
//! Binds 127.0.0.1 only. There is no authentication because there is no mutation
//! of stored state; do not expose it beyond localhost.

// Submodule lives under `src/bin/control_api/` so Cargo does not auto-discover
// it as a separate binary target; `mod` alone would look in `src/bin/`.
#[path = "control_api/supervisor.rs"]
mod supervisor;

use axum::{
    extract::{Query, State},
    http::{header, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use ising_engine::engine_v2::ai_scientist::{
    recall, BatchExecutor, ExperimentDb, ExperimentTask, InstanceSignature, KnowledgeGraph,
    MemoryManager, RuntimeExecutor,
};
use ising_engine::engine_v2::decision::DecisionEngine;
use ising_engine::engine_v2::evolution::Schedule;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::registry::OperatorRegistry;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use supervisor::{SharedSupervisor, StartReq, Status as CampaignStatus, Supervisor};
use tokio::net::TcpListener;

#[derive(Clone)]
struct Ctx {
    /// Campaign directory holding the append-only artifacts.
    dir: PathBuf,
    /// Where instance files live (rudy MaxCut format).
    instances: PathBuf,
    /// The supervised orchestrator (AD-1). One per directory.
    sup: SharedSupervisor,
}

fn arg(name: &str) -> Option<String> {
    let mut it = std::env::args();
    while let Some(a) = it.next() {
        if a == name {
            return it.next();
        }
    }
    None
}

// ───────────────────────────── health ─────────────────────────────

#[derive(Serialize)]
struct Health {
    ok: bool,
    service: &'static str,
    dir: String,
    dir_exists: bool,
    instances_dir: String,
    /// Endpoints this build actually serves — the UI reads this to decide what is live.
    endpoints: Vec<&'static str>,
}

async fn health(State(ctx): State<Arc<Ctx>>) -> Json<Health> {
    Json(Health {
        ok: true,
        service: "ising-control-api",
        dir: ctx.dir.display().to_string(),
        dir_exists: ctx.dir.is_dir(),
        instances_dir: ctx.instances.display().to_string(),
        endpoints: vec![
            "/health",
            "/api/system",
            "/api/db/stats",
            "/api/artifacts",
            "/api/replay",
            "/api/memory/report",
            "/api/memory/recall",
            "/api/campaign/status",
            "/api/campaign/start",
            "/api/campaign/pause",
            "/api/campaign/resume",
            "/api/campaign/step",
            "/api/campaign/stop",
        ],
    })
}

// ───────────────────────────── system ─────────────────────────────

#[derive(Serialize)]
struct Gpu {
    name: String,
    memory_total_mib: u64,
    memory_used_mib: u64,
    utilization_pct: u64,
}

#[derive(Serialize)]
struct Sys {
    cpu_threads: usize,
    /// From /proc/meminfo; None on platforms without it.
    mem_total_kb: Option<u64>,
    mem_available_kb: Option<u64>,
    load_avg_1m: Option<f64>,
    /// Empty when `nvidia-smi` is absent — never fabricated.
    gpus: Vec<Gpu>,
}

fn proc_meminfo() -> (Option<u64>, Option<u64>) {
    let Ok(text) = std::fs::read_to_string("/proc/meminfo") else {
        return (None, None);
    };
    let mut total = None;
    let mut avail = None;
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        match (parts.next(), parts.next()) {
            (Some("MemTotal:"), Some(v)) => total = v.parse().ok(),
            (Some("MemAvailable:"), Some(v)) => avail = v.parse().ok(),
            _ => {}
        }
    }
    (total, avail)
}

fn load_avg() -> Option<f64> {
    std::fs::read_to_string("/proc/loadavg")
        .ok()?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// Query nvidia-smi. Returns an empty vec if it is missing or fails — the UI
/// then shows "no GPU reported" rather than a made-up figure.
fn gpus() -> Vec<Gpu> {
    let Ok(out) = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,memory.used,utilization.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(',').map(str::trim).collect();
            if f.len() < 4 {
                return None;
            }
            Some(Gpu {
                name: f[0].to_string(),
                memory_total_mib: f[1].parse().ok()?,
                memory_used_mib: f[2].parse().ok()?,
                utilization_pct: f[3].parse().ok()?,
            })
        })
        .collect()
}

async fn system() -> Json<Sys> {
    let (mem_total_kb, mem_available_kb) = proc_meminfo();
    Json(Sys {
        cpu_threads: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        mem_total_kb,
        mem_available_kb,
        load_avg_1m: load_avg(),
        gpus: gpus(),
    })
}

// ───────────────────────── live db stats ─────────────────────────

#[derive(Serialize)]
struct DbStats {
    dir: String,
    /// Line count of the append-only DB *right now*.
    experiments: usize,
    knowledge_facts: usize,
    reports: usize,
    proposals: usize,
    /// Mtime (unix secs) so the UI can show data freshness.
    db_mtime: Option<u64>,
    /// Highest recorded id — useful for "N new since your snapshot".
    max_id: Option<u64>,
}

fn count_lines(p: &Path) -> usize {
    std::fs::read_to_string(p)
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0)
}

fn mtime_secs(p: &Path) -> Option<u64> {
    p.metadata()
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

fn count_dir(p: &Path, pat: &str) -> usize {
    std::fs::read_dir(p)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .filter(|e| e.file_name().to_string_lossy().contains(pat))
                .count()
        })
        .unwrap_or(0)
}

async fn db_stats(State(ctx): State<Arc<Ctx>>) -> Json<DbStats> {
    let db = ctx.dir.join("ai_experiments.txt");
    let max_id = std::fs::read_to_string(&db).ok().and_then(|s| {
        s.lines()
            .rfind(|l| !l.trim().is_empty())
            .and_then(|l| l.split('|').next()?.parse().ok())
    });
    Json(DbStats {
        dir: ctx.dir.display().to_string(),
        experiments: count_lines(&db),
        knowledge_facts: count_lines(&ctx.dir.join("knowledge_graph.txt")),
        reports: count_dir(&ctx.dir.join("reports"), "report_"),
        proposals: count_dir(&ctx.dir.join("proposals"), "operator_proposal_"),
        db_mtime: mtime_secs(&db),
        max_id,
    })
}

// ───────────────────────── artifact reading ─────────────────────────

/// Artifacts the API is allowed to serve. An explicit allow-list, so a path
/// cannot be used to read arbitrary files off the host.
const ALLOWED: &[&str] = &[
    "ai_experiments.txt",
    "knowledge_graph.txt",
    "knowledge.txt",
    "campaign_state.txt",
    "model_registry.txt",
    "evaluation_report.md",
    "portability_report.md",
    "dataset/foundation_manifest.md",
];

#[derive(Deserialize)]
struct ArtifactQ {
    path: String,
    /// Return only the last N lines (for tailing big append-only files).
    tail: Option<usize>,
}

#[derive(Serialize)]
struct Artifact {
    path: String,
    lines: usize,
    truncated: bool,
    content: String,
}

async fn artifact(
    State(ctx): State<Arc<Ctx>>,
    Query(q): Query<ArtifactQ>,
) -> Result<Json<Artifact>, (StatusCode, String)> {
    let rel = q.path.trim().trim_start_matches('/');
    let allowed = ALLOWED.contains(&rel)
        || (rel.starts_with("reports/") && rel.ends_with(".md"))
        || (rel.starts_with("proposals/") && rel.ends_with(".md"));
    if !allowed || rel.contains("..") {
        return Err((
            StatusCode::FORBIDDEN,
            format!("not an allowed artifact: {rel}"),
        ));
    }
    let full = ctx.dir.join(rel);
    let text = std::fs::read_to_string(&full)
        .map_err(|e| (StatusCode::NOT_FOUND, format!("{}: {e}", full.display())))?;
    let all: Vec<&str> = text.lines().collect();
    let (content, truncated) = match q.tail {
        Some(n) if n < all.len() => (all[all.len() - n..].join("\n"), true),
        _ => (text.clone(), false),
    };
    Ok(Json(Artifact {
        path: rel.to_string(),
        lines: all.len(),
        truncated,
        content,
    }))
}

// ───────────────────────────── REPLAY ─────────────────────────────

#[derive(Deserialize)]
struct ReplayReq {
    /// Recorded experiment id to replay.
    id: u64,
    /// Override the instance file directory lookup (optional).
    instance_file: Option<String>,
}

#[derive(Serialize)]
struct ReplayResp {
    id: u64,
    instance: String,
    instance_file: String,
    /// The exact schedule that was recorded.
    operators: Vec<String>,
    sweeps: Vec<u32>,
    temp_hi: f64,
    temp_lo: f64,
    replicas: usize,
    seed: u64,
    recorded_score: f64,
    replayed_score: f64,
    /// True when the replay reproduced the recorded score exactly.
    identical: bool,
    /// Absolute difference; 0.0 when identical.
    delta: f64,
    backend_used: String,
    wall_ms: f64,
    /// Stated plainly so a mismatch is never silently rounded away.
    note: &'static str,
}

/// Replay one recorded experiment through the read-only Runtime and compare.
async fn replay(
    State(ctx): State<Arc<Ctx>>,
    Json(req): Json<ReplayReq>,
) -> Result<Json<ReplayResp>, (StatusCode, String)> {
    let dir = ctx.dir.clone();
    let instances = ctx.instances.clone();

    // The Runtime is synchronous and CPU-bound; keep it off the async worker.
    tokio::task::spawn_blocking(move || -> Result<ReplayResp, (StatusCode, String)> {
        let db = ExperimentDb::load(dir.join("ai_experiments.txt"))
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db: {e}")))?;
        let rec = db.all().iter().find(|r| r.id == req.id).cloned().ok_or((
            StatusCode::NOT_FOUND,
            format!("no experiment with id {}", req.id),
        ))?;

        // Locate the instance file the record refers to.
        let file = match req.instance_file {
            Some(f) => PathBuf::from(f),
            None => instances.join(&rec.instance_id),
        };
        let text = std::fs::read_to_string(&file).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!(
                    "cannot read instance {} ({}): {e}",
                    rec.instance_id,
                    file.display()
                ),
            )
        })?;
        let ir = rudy_maxcut_ir(&text)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("lowering failed: {e}")))?;

        // Rebuild the recorded schedule verbatim.
        let schedule = Schedule {
            ops: rec.sequence.clone(),
            sweeps: rec.sweeps.clone(),
            temp_hi: rec.temp_hi,
            temp_lo: rec.temp_lo,
        };
        let task = ExperimentTask {
            schedule,
            num_replicas: rec.num_replicas,
            seed: rec.seed,
        };

        // `standard()` — NOT `new()`, which is an empty registry and would
        // silently resolve no operators and return a non-finite score.
        let registry = OperatorRegistry::standard();
        let executor = RuntimeExecutor::auto();
        let started = std::time::Instant::now();
        let outcomes = executor.run_batch(&ir, &registry, std::slice::from_ref(&task));
        let wall_ms = started.elapsed().as_secs_f64() * 1000.0;
        let out = outcomes.first().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "executor returned no outcome".to_string(),
        ))?;

        // A non-finite score means the run produced nothing usable (e.g. an
        // operator failed to resolve). Report that as an error rather than
        // serialising it to `null` and letting the UI read it as "not identical".
        if !out.score.is_finite() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "replay produced a non-finite score for id {} — operators {:?} may not be \
                     registered in the standard registry",
                    rec.id, rec.sequence
                ),
            ));
        }
        let delta = (out.score - rec.score).abs();
        Ok(ReplayResp {
            id: rec.id,
            instance: rec.instance_id.clone(),
            instance_file: file.display().to_string(),
            operators: rec.sequence.clone(),
            sweeps: rec.sweeps.clone(),
            temp_hi: rec.temp_hi,
            temp_lo: rec.temp_lo,
            replicas: rec.num_replicas,
            seed: rec.seed,
            recorded_score: rec.score,
            replayed_score: out.score,
            identical: delta == 0.0,
            delta,
            backend_used: format!("{:?}", out.backend),
            wall_ms,
            note: "Determinism (ADR-0004) means an identical seed must reproduce an identical \
                   score. A non-zero delta is a real finding, not a rounding artifact — the \
                   recorded backend may differ from the one selected now.",
        })
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?
    .map(Json)
}

// ─────────────────────── scientific memory ───────────────────────
//
// `memory_os` computes recall in memory and never serialises it, so the
// laboratory had nothing honest to render. These two endpoints are pure
// serialisation of what the faculty already returns — no new engine logic.

#[derive(Serialize)]
struct BucketOut {
    label: String,
    experiments: usize,
    instances: usize,
    /// Enough data + a stable dominant operator that the raw detail COULD be
    /// summarised. The store is append-only: nothing is ever deleted.
    compactable: bool,
    dominant_operator: Option<String>,
}

#[derive(Serialize)]
struct MemoryReportOut {
    total_experiments: usize,
    distinct_instances: usize,
    buckets: Vec<BucketOut>,
    note: String,
}

fn load_db(dir: &Path) -> Result<ExperimentDb, (StatusCode, String)> {
    ExperimentDb::load(dir.join("ai_experiments.txt"))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db: {e}")))
}

async fn memory_report(
    State(ctx): State<Arc<Ctx>>,
) -> Result<Json<MemoryReportOut>, (StatusCode, String)> {
    let dir = ctx.dir.clone();
    // Scans the whole store — keep it off the async runtime.
    tokio::task::spawn_blocking(move || -> Result<MemoryReportOut, (StatusCode, String)> {
        let db = load_db(&dir)?;
        let rep = MemoryManager::default().analyze(&db);
        Ok(MemoryReportOut {
            total_experiments: rep.total_experiments,
            distinct_instances: rep.distinct_instances,
            buckets: rep
                .buckets
                .into_iter()
                .map(|b| BucketOut {
                    label: b.label,
                    experiments: b.experiments,
                    instances: b.instances,
                    compactable: b.compactable,
                    dominant_operator: b.dominant_operator,
                })
                .collect(),
            note: rep.note,
        })
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?
    .map(Json)
}

#[derive(Deserialize)]
struct RecallQ {
    /// Instance file name under the instances dir, e.g. "G22".
    instance: String,
    /// Feature-distance radius; instances within it count as similar.
    radius: Option<f64>,
}

#[derive(Serialize)]
struct BestOperator {
    operator: String,
    mean_improvement: f64,
}

#[derive(Serialize)]
struct RecallOut {
    instance: String,
    radius: f64,
    signature: SignatureOut,
    similar_instances: Vec<String>,
    best_operators: Vec<BestOperator>,
    applicable_facts: Vec<String>,
    narrative: String,
    /// Stated so the UI never implies this came from a build-time snapshot.
    computed_from: String,
}

#[derive(Serialize)]
struct SignatureOut {
    n: usize,
    density: f64,
    clustering: f64,
    mean_degree: f64,
    degree_cv: f64,
}

async fn memory_recall(
    State(ctx): State<Arc<Ctx>>,
    Query(q): Query<RecallQ>,
) -> Result<Json<RecallOut>, (StatusCode, String)> {
    let dir = ctx.dir.clone();
    let instances = ctx.instances.clone();
    let radius = q.radius.unwrap_or(0.35);
    if q.instance.contains('/') || q.instance.contains("..") {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("invalid instance name: {}", q.instance),
        ));
    }

    tokio::task::spawn_blocking(move || -> Result<RecallOut, (StatusCode, String)> {
        let file = instances.join(&q.instance);
        let text = std::fs::read_to_string(&file).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("cannot read instance {}: {e}", file.display()),
            )
        })?;
        let ir = rudy_maxcut_ir(&text)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("lowering failed: {e}")))?;

        // Same signature the orchestrator builds each tick.
        let stats = DecisionEngine::analyze(&ir);
        let sig = InstanceSignature {
            n: stats.n,
            density: stats.density,
            clustering: stats.clustering,
            mean_degree: stats.mean_degree,
            degree_cv: stats.degree_cv,
        };

        let db = load_db(&dir)?;
        let graph = KnowledgeGraph::load(dir.join("knowledge_graph.txt"))
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("graph: {e}")))?;
        let rec = recall(&db, &graph, &sig, radius);

        Ok(RecallOut {
            instance: q.instance.clone(),
            radius,
            signature: SignatureOut {
                n: sig.n,
                density: sig.density,
                clustering: sig.clustering,
                mean_degree: sig.mean_degree,
                degree_cv: sig.degree_cv,
            },
            similar_instances: rec.similar_instances,
            best_operators: rec
                .best_operators
                .into_iter()
                .map(|(operator, mean_improvement)| BestOperator {
                    operator,
                    mean_improvement,
                })
                .collect(),
            applicable_facts: rec.applicable_facts,
            narrative: rec.narrative,
            computed_from: format!(
                "live scan of {} and knowledge_graph.txt",
                dir.join("ai_experiments.txt").display()
            ),
        })
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?
    .map(Json)
}

// ─────────────────────── campaign lifecycle ───────────────────────

async fn campaign_status(State(ctx): State<Arc<Ctx>>) -> Json<CampaignStatus> {
    Json(ctx.sup.status())
}

/// Map a supervisor error to a 409 (conflict) — the usual cause is "already
/// running" or "nothing to act on", both of which are caller mistakes.
fn conflict(e: String) -> (StatusCode, String) {
    (StatusCode::CONFLICT, e)
}

async fn campaign_start(
    State(ctx): State<Arc<Ctx>>,
    Json(req): Json<StartReq>,
) -> Result<Json<CampaignStatus>, (StatusCode, String)> {
    ctx.sup.start(req).map(Json).map_err(conflict)
}

macro_rules! lifecycle {
    ($name:ident, $method:ident) => {
        async fn $name(
            State(ctx): State<Arc<Ctx>>,
        ) -> Result<Json<CampaignStatus>, (StatusCode, String)> {
            // `stop` joins the worker so state is persisted before we reply;
            // keep it off the async runtime.
            let sup = Arc::clone(&ctx.sup);
            tokio::task::spawn_blocking(move || sup.$method())
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?
                .map(Json)
                .map_err(conflict)
        }
    };
}
lifecycle!(campaign_pause, pause);
lifecycle!(campaign_resume, resume);
lifecycle!(campaign_step, step);
lifecycle!(campaign_stop, stop);

// ───────────────────────────── CORS ─────────────────────────────

/// Origins the laboratory dev/prod server runs on. Hand-rolled rather than
/// pulling in `tower-http` for four headers (CLAUDE.md §4: no new crate without
/// justification).
const ALLOWED_ORIGINS: &[&str] = &[
    "http://localhost:3000",
    "http://localhost:3100",
    "http://127.0.0.1:3000",
    "http://127.0.0.1:3100",
];

async fn cors(req: Request<axum::body::Body>, next: Next) -> Response {
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .filter(|o| ALLOWED_ORIGINS.contains(o))
        .map(str::to_owned);
    let preflight = req.method() == axum::http::Method::OPTIONS;

    let mut res = if preflight {
        Response::new(axum::body::Body::empty())
    } else {
        next.run(req).await
    };

    if let Some(o) = origin {
        let h = res.headers_mut();
        if let Ok(v) = HeaderValue::from_str(&o) {
            h.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, v);
        }
        h.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET, POST, OPTIONS"),
        );
        h.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static("content-type"),
        );
    }
    res
}

// ───────────────────────────── main ─────────────────────────────

#[tokio::main]
async fn main() {
    let dir =
        PathBuf::from(arg("--dir").unwrap_or_else(|| "experiments/platform_gset".to_string()));
    let instances = PathBuf::from(
        arg("--instances").unwrap_or_else(|| "benchmark_suite/data/gset".to_string()),
    );
    let port: u16 = arg("--port").and_then(|s| s.parse().ok()).unwrap_or(7878);

    if !dir.is_dir() {
        eprintln!("warning: --dir {} does not exist yet", dir.display());
    }

    let sup: SharedSupervisor = Arc::new(Supervisor::new(dir.clone(), instances.clone()));
    let ctx = Arc::new(Ctx {
        dir,
        instances,
        sup,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/system", get(system))
        .route("/api/db/stats", get(db_stats))
        .route("/api/artifacts", get(artifact))
        .route("/api/replay", post(replay))
        .route("/api/memory/report", get(memory_report))
        .route("/api/memory/recall", get(memory_recall))
        .route("/api/campaign/status", get(campaign_status))
        .route("/api/campaign/start", post(campaign_start))
        .route("/api/campaign/pause", post(campaign_pause))
        .route("/api/campaign/resume", post(campaign_resume))
        .route("/api/campaign/step", post(campaign_step))
        .route("/api/campaign/stop", post(campaign_stop))
        .layer(middleware::from_fn(cors))
        .with_state(ctx.clone());

    let addr = format!("127.0.0.1:{port}");
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot bind {addr}: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "control_api on http://{addr}\n  dir       = {}\n  instances = {}\n  endpoints = /health /api/system /api/db/stats /api/artifacts /api/replay /api/campaign/*",
        ctx.dir.display(),
        ctx.instances.display()
    );
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("server error: {e}");
    }
}
