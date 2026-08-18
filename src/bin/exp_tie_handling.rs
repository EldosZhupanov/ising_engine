//! RC-015 — is the cold reversal a tie-handling effect?
//!
//! Pre-registered in `research/PREREG_RC015.md`, committed before this file
//! existed. Origin: RC-014 §10 found `gibbs_color_sweep` decisively better than
//! `metropolis_sweep` on G43 at flat `T = 0.1` (`I = −44.13`, ρ = 5.31,
//! p = 0.0078) and wrongly excluded tie handling from the explanation.
//!
//! # The decomposition
//!
//! Three arms, each consuming **exactly one `f64` per (sweep, site, replica)**,
//! so every pairwise comparison is an exact counterfactual under the Runtime's
//! single stream (RC-014 Phase 0 §0.1):
//!
//! ```text
//!            ΔE < 0          ΔE = 0                    ΔE > 0
//!   M    draw, accept    draw, ACCEPT              draw, accept iff u < e^(−ΔE/T)
//!   M½   draw, accept    draw, flip iff u < ½      draw, accept iff u < e^(−ΔE/T)
//!   G    draw, flip iff u < σ (heat-bath everywhere; at ΔE = 0, σ = ½)
//!
//!   I_tie  = Y(M½) − Y(M)     tie handling alone
//!   I_off  = Y(G)  − Y(M½)    everything other than ties
//!   I_full = Y(G)  − Y(M)  =  I_tie + I_off
//! ```
//!
//! Why ties can dominate at `T = 0.1`: with integer couplings the acceptance
//! ratio Metropolis : heat-bath is **1.0000** at `ΔE = +1` and `+2`, while at
//! `ΔE = 0` it is **2.0** and temperature-independent. Off ties the kernels are
//! numerically indistinguishable there.
//!
//! ```text
//! cargo run --release --bin exp_tie_handling -- --controls
//! cargo run --release --bin exp_tie_handling -- --science
//! cargo run --release --bin exp_tie_handling -- --gradient
//! ```

use ising_engine::engine_v2::capability::{
    Capability, CapabilitySet, Complexity, Constraints, Guarantees, Observable, ObservableSet,
    OperatorDescriptor,
};
use ising_engine::engine_v2::context::RunContext;
use ising_engine::engine_v2::decision::DecisionEngine;
use ising_engine::engine_v2::evolution::boxed_state;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::operator::{
    Budget, CostEstimate, InstanceShape, Nature, Operator, Report,
};
use ising_engine::engine_v2::plan::{Phase, Plan, PlanStep};
use ising_engine::engine_v2::registry::OperatorRegistry;
use ising_engine::engine_v2::runtime::{Runtime, RuntimeView};
use ising_engine::engine_v2::state::{ReplicaMask, SpinState};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

// ------------------------------------------------------- pre-registered design

const REPLICAS: usize = 32;
const SWEEPS: u32 = 16;
const T_COLD: f64 = 0.1;
const HELD_IN: [u64; 8] = [1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008];
const HELD_OUT: [u64; 8] = [2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008];

const M: &str = "metropolis_sweep";
const G: &str = "gibbs_color_sweep";
const MHALF: &str = "metropolis_tie_half";
const MNULL: &str = "metropolis_tie_accept";

/// Set A is the matched triple containing the anomaly; set B is the same family
/// at 2× n. All are all-`+1` weights (verified from the file headers).
const SET_A: &[(&str, &str)] = &[
    ("G43", "benchmark_suite/data/gset/G43"),
    ("G44", "benchmark_suite/data/gset/G44"),
    ("G45", "benchmark_suite/data/gset/G45"),
];
const SET_B: &[(&str, &str)] = &[
    ("G22", "benchmark_suite/data/gset/G22"),
    ("G23", "benchmark_suite/data/gset/G23"),
    ("G24", "benchmark_suite/data/gset/G24"),
];

// ------------------------------------------------------------- the M½ operator

#[derive(Clone, Copy, PartialEq)]
enum TieMode {
    /// ΔE = 0 ⇒ accept. Identical to `metropolis_sweep`; the null by construction.
    Accept,
    /// ΔE = 0 ⇒ flip with probability ½, i.e. heat-bath's tie rule.
    Half,
    /// D-15: ΔE = 0 ⇒ flip with probability `q`. `Accept` is q = 1, `Half` is
    /// q = 0.5; this variant carries the intermediate dial points.
    Prob(f64),
}

/// The D-15 dial. `q = 1` is Metropolis, `q = 0.5` is M½/heat-bath's tie rule.
const Q_GRID: [f64; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

/// Operator name for a dial point. Leaked once per q — the registry needs
/// `&'static str` and the grid is a five-element compile-time constant.
fn q_name(q: f64) -> &'static str {
    Box::leak(format!("metropolis_tie_q{:.2}", q).into_boxed_str())
}

/// Metropolis with a parameterised tie rule. Everything off `ΔE = 0` is
/// byte-for-byte the production Metropolis decision, and the draw count is
/// `sweeps · n · r` `f64` in both modes — so M, M½ and G are mutually
/// draw-identical and the substitution stays exact.
struct MetropolisTie {
    mode: TieMode,
    name: &'static str,
    order: Vec<usize>,
    de: Vec<f64>,
    mask: ReplicaMask,
    ready: bool,
    /// ΔE class counts for THIS arm's own trajectory: (<0, =0, >0).
    classes: (u64, u64, u64),
    /// Smallest non-zero |ΔE| seen.
    min_abs_de: f64,
}

impl MetropolisTie {
    fn new(mode: TieMode) -> Self {
        Self {
            mode,
            name: match mode {
                TieMode::Accept => MNULL,
                TieMode::Half => MHALF,
                TieMode::Prob(q) => q_name(q),
            },
            order: Vec::new(),
            de: Vec::new(),
            mask: ReplicaMask::new(0),
            ready: false,
            classes: (0, 0, 0),
            min_abs_de: f64::INFINITY,
        }
    }

    /// The same colour order `metropolis_sweep`/`gibbs_color_sweep` walk: colours
    /// ascending, sites ascending within a colour.
    fn ensure_order(&mut self, state: &dyn SpinState) {
        if self.ready {
            return;
        }
        let colors = state.coloring();
        let ncolors = colors.iter().copied().max().map_or(0, |c| c as usize + 1);
        let mut order = Vec::with_capacity(state.num_vars());
        for c in 0..ncolors as u32 {
            for (site, &col) in colors.iter().enumerate() {
                if col == c {
                    order.push(site);
                }
            }
        }
        self.order = order;
        self.de = vec![0.0; state.num_replicas()];
        self.mask = ReplicaMask::new(state.num_replicas());
        self.ready = true;
    }
}

impl Operator for MetropolisTie {
    fn descriptor(&self) -> OperatorDescriptor {
        OperatorDescriptor {
            name: self.name,
            nature: Nature::TrajectoryPreserving,
            // Identical passport to `metropolis_sweep`: this operator IS
            // Metropolis everywhere except at ΔE = 0.
            capabilities: CapabilitySet::empty()
                .with(Capability::Exploration)
                .with(Capability::Exploitation),
            constraints: Constraints {
                needs_integer: false,
                needs_float: false,
                supports_sparse: true,
                supports_dense: true,
                needs_replicas: false,
                needs_temperature: true,
            },
            observables: ObservableSet::empty()
                .with(Observable::Acceptance)
                .with(Observable::EnergyDelta),
            guarantees: Guarantees {
                equilibrates: true,
                deterministic: true,
                ..Guarantees::none()
            },
            complexity: Complexity::LinearEdges,
        }
    }

    fn cost_model(&self, shape: InstanceShape) -> CostEstimate {
        CostEstimate {
            work_per_sweep: ((shape.n + 2 * shape.num_pairs) * shape.num_replicas.max(1)) as f64,
        }
    }

    fn apply(
        &mut self,
        state: &mut dyn SpinState,
        view: &RuntimeView,
        rng: &mut ChaCha8Rng,
        budget: Budget,
    ) -> Report {
        self.ensure_order(state);
        let r = state.num_replicas();
        let temps = view.temperatures;
        let mut report = Report::default();
        for _ in 0..budget.sweeps {
            for &site in &self.order {
                state.delta_e_into(site, &mut self.de);
                self.mask.clear();
                for rep in 0..r {
                    let d = self.de[rep];
                    let t = temps[rep % temps.len()];
                    // Per-arm ΔE census (RC-015 §7: never inferred from another arm).
                    if d < 0.0 {
                        self.classes.0 += 1;
                    } else if d == 0.0 {
                        self.classes.1 += 1;
                    } else {
                        self.classes.2 += 1;
                    }
                    if d != 0.0 && d.abs() < self.min_abs_de {
                        self.min_abs_de = d.abs();
                    }
                    // Exactly one draw on every path, in every mode.
                    let accept = if d < 0.0 {
                        let _u: f64 = rng.gen();
                        true
                    } else if d == 0.0 {
                        let u: f64 = rng.gen();
                        // One draw on every path, in every mode, so all dial
                        // points stay mutually draw-identical.
                        match self.mode {
                            TieMode::Accept => true,
                            TieMode::Half => u < 0.5,
                            TieMode::Prob(q) => u < q,
                        }
                    } else {
                        let p = if t > 0.0 { (-d / t).exp() } else { 0.0 };
                        let u: f64 = rng.gen();
                        u < p
                    };
                    report.proposed += 1;
                    if accept {
                        self.mask.set(rep);
                        report.accepted += 1;
                    }
                }
                if !self.mask.is_empty() {
                    state.apply_flips(site, &self.mask);
                }
            }
        }
        report.work = self
            .cost_model(InstanceShape {
                n: self.order.len(),
                num_pairs: 0,
                num_replicas: r,
            })
            .work_per_sweep
            * budget.sweeps as f64;
        report
    }
}

/// Standard registry plus the two experiment-local tie variants. Production's
/// registry is untouched (Constitution §13: prototypes live in isolation).
fn local_registry() -> OperatorRegistry {
    let mut reg = OperatorRegistry::standard();
    reg.register(|| Box::new(MetropolisTie::new(TieMode::Half)));
    reg.register(|| Box::new(MetropolisTie::new(TieMode::Accept)));
    // D-15 dial points. q = 1.0 must be bit-identical to `metropolis_sweep`
    // (null by construction) and q = 0.5 must reproduce the G arm.
    for q in Q_GRID {
        reg.register(move || Box::new(MetropolisTie::new(TieMode::Prob(q))));
    }
    reg
}

// ------------------------------------------------------------------- plumbing

fn flag(n: &str) -> bool {
    std::env::args().any(|a| a == n)
}

fn load(path: &str) -> Result<ProblemIR, String> {
    rudy_maxcut_ir(&std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?)
}

struct Out {
    /// Best energy after the substituted step, BEFORE `greedy_descent`.
    pre: f64,
    /// Best energy at the end of the run.
    post: f64,
    best_state: Vec<u8>,
}

fn plan_for(ir: &ProblemIR, op: &str, seed: u64, temp: f64) -> Plan {
    Plan {
        name: "rc015".into(),
        backend: DecisionEngine::analyze(ir).select_backend(),
        num_replicas: REPLICAS,
        temperatures: vec![temp],
        steps: vec![
            PlanStep {
                operator: op.into(),
                phase: Phase::Exploit,
                sweeps: SWEEPS,
                repeat: 1,
            },
            PlanStep {
                operator: "greedy_descent".into(),
                phase: Phase::Exploit,
                sweeps: SWEEPS,
                repeat: 1,
            },
        ],
        seed,
        rationale: Default::default(),
    }
}

fn execute(ir: &ProblemIR, reg: &OperatorRegistry, plan: &Plan) -> Result<Out, String> {
    let init = vec![0u8; ir.n];
    let mut state = boxed_state(ir, plan.backend, REPLICAS, &init);
    let mut rt = Runtime::new(RunContext::new(plan.seed), plan);
    let rec = rt.run(plan, state.as_mut(), reg, ir)?;
    // events[0] is the substituted step; best_energy is the running minimum, so
    // events[0] is exactly "before greedy_descent".
    let pre = rec
        .events
        .first()
        .map_or(rec.best_energy, |e| e.best_energy);
    Ok(Out {
        pre,
        post: rec.best_energy,
        best_state: rec.best_state.clone(),
    })
}

// -------------------------------------------------------- paired statistics
// Same layer RC-014 validated against a fixed analytic table.

struct Paired {
    base: Vec<f64>,
    other: Vec<f64>,
}

impl Paired {
    fn effects(&self) -> Vec<f64> {
        self.other
            .iter()
            .zip(&self.base)
            .map(|(a, b)| a - b)
            .collect()
    }
    fn mean(&self) -> f64 {
        let e = self.effects();
        e.iter().sum::<f64>() / e.len() as f64
    }
    fn d_seed(&self) -> f64 {
        let n = self.base.len() as f64;
        if n < 2.0 {
            return 0.0;
        }
        let m = self.base.iter().sum::<f64>() / n;
        (self.base.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
    }
    fn degenerate(&self) -> bool {
        self.d_seed() < 1e-9
    }
    fn rho(&self) -> Option<f64> {
        if self.degenerate() {
            None
        } else {
            Some(self.mean().abs() / self.d_seed())
        }
    }
    fn rel(&self) -> f64 {
        let s = (self.base.iter().sum::<f64>() / self.base.len() as f64).abs();
        if s < 1e-12 {
            0.0
        } else {
            self.mean().abs() / s
        }
    }
    fn p(&self) -> f64 {
        let e = self.effects();
        let k = e.len();
        let obs = (e.iter().sum::<f64>() / k as f64).abs();
        let total = 1usize << k;
        let mut ge = 0usize;
        for mask in 0..total {
            let mut s = 0.0;
            for (i, v) in e.iter().enumerate() {
                if mask >> i & 1 == 1 {
                    s -= v
                } else {
                    s += v
                }
            }
            if (s / k as f64).abs() >= obs {
                ge += 1;
            }
        }
        ge as f64 / total as f64
    }
    fn fmt(&self) -> String {
        format!(
            "I={:+9.4} rho={:<7} rel={:6.4}% p={:.4}{}",
            self.mean(),
            self.rho().map_or("DEGEN".into(), |r| format!("{r:.3}")),
            self.rel() * 100.0,
            self.p(),
            if self.degenerate() { " DEGEN" } else { "" }
        )
    }
}

// -------------------------------------------------------------------- controls

/// Control 1 — `TieMode::Accept` must be bit-identical to `metropolis_sweep`.
/// The null lives inside the operator (RC-001's discipline), so a construction
/// error cannot be mistaken for an effect.
fn control_null(ir: &ProblemIR, reg: &OperatorRegistry) -> bool {
    let mut ok = true;
    for &s in HELD_IN.iter().take(3) {
        let (a, b) = match (
            execute(ir, reg, &plan_for(ir, M, s, T_COLD)),
            execute(ir, reg, &plan_for(ir, MNULL, s, T_COLD)),
        ) {
            (Ok(a), Ok(b)) => (a, b),
            _ => return false,
        };
        let same = a.post.to_bits() == b.post.to_bits()
            && a.pre.to_bits() == b.pre.to_bits()
            && a.best_state == b.best_state;
        println!(
            "    seed {s}: metropolis={:.4} tie_accept={:.4} state-identical={} -> {}",
            a.post,
            b.post,
            a.best_state == b.best_state,
            if same { "pass" } else { "FAIL" }
        );
        ok &= same;
    }
    ok
}

/// Control 2 — all three arms leave the generator at the same position, and an
/// injected one-word shift is detected.
fn control_alignment(ir: &ProblemIR, reg: &OperatorRegistry) -> bool {
    let backend = DecisionEngine::analyze(ir).select_backend();
    let init = vec![0u8; ir.n];
    let temps = vec![T_COLD];
    let view = RuntimeView {
        iteration: 0,
        temperatures: &temps,
        num_replicas: REPLICAS,
        recent_acceptance: 0.0,
        remaining_ms: f64::INFINITY,
    };
    let probe = |op: &str, shift: bool| -> u64 {
        let mut st = boxed_state(ir, backend, REPLICAS, &init);
        let mut rng = ChaCha8Rng::seed_from_u64(1001);
        if shift {
            let _: u32 = rng.gen();
        }
        let mut o = reg.lookup(op).expect("registered");
        o.apply(st.as_mut(), &view, &mut rng, Budget { sweeps: SWEEPS });
        rng.gen()
    };
    let (pm, ph, pg) = (probe(M, false), probe(MHALF, false), probe(G, false));
    let ps = probe(MHALF, true);
    let aligned = pm == ph && ph == pg;
    println!(
        "    M={pm:#018x} M½={ph:#018x} G={pg:#018x} -> {}",
        if aligned {
            "ALL EQUAL (pass)"
        } else {
            "DIFFER (FAIL)"
        }
    );
    println!(
        "    shifted M½={ps:#018x} -> {}",
        if ps != pm {
            "DETECTED (pass)"
        } else {
            "MISSED (FAIL)"
        }
    );
    aligned && ps != pm
}

// --------------------------------------------------------------------- science

struct Cell {
    tie: Paired,
    off: Paired,
    full: Paired,
    share: f64,
    closes: f64,
}

fn measure(
    ir: &ProblemIR,
    reg: &OperatorRegistry,
    seeds: &[u64],
    temp: f64,
    pre: bool,
) -> Option<Cell> {
    let (mut ym, mut yh, mut yg) = (Vec::new(), Vec::new(), Vec::new());
    for &s in seeds {
        let a = execute(ir, reg, &plan_for(ir, M, s, temp)).ok()?;
        let b = execute(ir, reg, &plan_for(ir, MHALF, s, temp)).ok()?;
        let c = execute(ir, reg, &plan_for(ir, G, s, temp)).ok()?;
        let pick = |o: &Out| if pre { o.pre } else { o.post };
        ym.push(pick(&a));
        yh.push(pick(&b));
        yg.push(pick(&c));
    }
    let tie = Paired {
        base: ym.clone(),
        other: yh.clone(),
    };
    let off = Paired {
        base: yh.clone(),
        other: yg.clone(),
    };
    let full = Paired {
        base: ym,
        other: yg,
    };
    let share = if full.mean().abs() < 1e-12 {
        f64::NAN
    } else {
        tie.mean().abs() / full.mean().abs()
    };
    let closes = full.mean() - (tie.mean() + off.mean());
    Some(Cell {
        tie,
        off,
        full,
        share,
        closes,
    })
}

fn report_cell(tag: &str, c: &Cell) {
    println!("    {tag}");
    println!("      I_full (G−M)   {}", c.full.fmt());
    println!("      I_tie  (M½−M)  {}", c.tie.fmt());
    println!("      I_off  (G−M½)  {}", c.off.fmt());
    println!(
        "      |I_tie|/|I_full| = {:.3}   decomposition residual = {:.3e}",
        c.share, c.closes
    );
    // The gate is only meaningful where the effect being decomposed EXISTS.
    // With |I_full| ~ 0 the share ratio is undefined (it reached 19.0 on a null
    // cell in the first run) and "residual not material" is trivially true, so
    // the verdict must not be evaluated there. Materiality is the same two-part
    // rule as everywhere else: rho >= 0.5 AND rel >= 0.1%.
    let full_material =
        !c.full.degenerate() && c.full.rho().is_some_and(|r| r >= 0.5) && c.full.rel() >= 0.001;
    if !full_material {
        println!("      verdict: I_full NOT MATERIAL — share undefined, gate not applicable");
        return;
    }
    let confirmed = c.share >= 0.80 && !c.off.rho().is_some_and(|r| r >= 0.5);
    let refuted = c.off.rho().is_some_and(|r| r >= 0.5) && c.off.rel() >= 0.001;
    println!(
        "      verdict: {}",
        if confirmed {
            "H-15 CONFIRMED (tie share >= 0.80, residual not material)"
        } else if refuted {
            "H-15 residual is MATERIAL -> refuted pending held-out replication"
        } else {
            "PARTIAL (share ok but residual rho >= 0.5 while rel < 0.1%)"
        }
    );
}

fn science(reg: &OperatorRegistry, seeds: &[u64], label: &str) {
    for (set, name) in [
        (SET_A, "SET A (n=1000, m=9990)"),
        (SET_B, "SET B (n=2000, m=19990)"),
    ] {
        println!("\n  {name}");
        for (inst, path) in set {
            let Ok(ir) = load(path) else {
                println!("  {inst}: SKIP");
                continue;
            };
            println!("  [{label}] {inst}");
            for (pre, tag) in [
                (true, "BEFORE greedy_descent"),
                (false, "AFTER greedy_descent"),
            ] {
                match measure(&ir, reg, seeds, T_COLD, pre) {
                    Some(c) => report_cell(tag, &c),
                    None => println!("    {tag}: run failed"),
                }
            }
            // Per-arm ΔE census, from each arm's OWN trajectory (RC-015 §7).
            census(&ir, reg);
        }
    }
}

/// Runs each arm once and reports its own ΔE class counts and min non-zero |ΔE|.
/// The RC-014 defect was measuring ties along one arm only; this cannot recur.
fn census(ir: &ProblemIR, _reg: &OperatorRegistry) {
    let backend = DecisionEngine::analyze(ir).select_backend();
    let init = vec![0u8; ir.n];
    let temps = vec![T_COLD];
    let view = RuntimeView {
        iteration: 0,
        temperatures: &temps,
        num_replicas: REPLICAS,
        recent_acceptance: 0.0,
        remaining_ms: f64::INFINITY,
    };
    for mode in [TieMode::Accept, TieMode::Half] {
        let mut op = MetropolisTie::new(mode);
        let mut st = boxed_state(ir, backend, REPLICAS, &init);
        let mut rng = ChaCha8Rng::seed_from_u64(HELD_IN[0]);
        op.apply(st.as_mut(), &view, &mut rng, Budget { sweeps: SWEEPS });
        let (lt, eq, gt) = op.classes;
        let tot = (lt + eq + gt).max(1) as f64;
        println!(
            "      census {:<22} ΔE<0 {:5.2}%  ΔE=0 {:5.2}%  ΔE>0 {:5.2}%  min|ΔE|≠0 = {}  ties/replica = {:.0}",
            if mode == TieMode::Accept { "M (tie=accept)" } else { "M½ (tie=half)" },
            lt as f64 / tot * 100.0,
            eq as f64 / tot * 100.0,
            gt as f64 / tot * 100.0,
            if op.min_abs_de.is_finite() {
                format!("{:.1}", op.min_abs_de)
            } else {
                "n/a".into()
            },
            eq as f64 / REPLICAS as f64
        );
    }
}

/// Non-gating: the tie share must FALL as T rises, because off-tie acceptance
/// ratios diverge (1.0000 at T=0.1, 1.14 at 0.5, 1.61 at 2.0 for ΔE=+1).
fn gradient(reg: &OperatorRegistry) {
    println!("Gradient (non-gating): tie share vs temperature, G43, AFTER greedy\n");
    let Ok(ir) = load(SET_A[0].1) else { return };
    for t in [0.1f64, 0.5, 2.0] {
        match measure(&ir, reg, &HELD_IN, t, false) {
            Some(c) => println!(
                "  T={t:<4} I_full={:+9.4}  I_tie={:+9.4}  I_off={:+9.4}  share={:.3}",
                c.full.mean(),
                c.tie.mean(),
                c.off.mean(),
                c.share
            ),
            None => println!("  T={t}: failed"),
        }
    }
}

/// Seeded percentile bootstrap on the paired mean difference (Amendment 1 A1.3).
fn bootstrap_ci(xs: &[f64], reps: usize, seed: u64) -> (f64, f64) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut means: Vec<f64> = Vec::with_capacity(reps);
    for _ in 0..reps {
        let mut s = 0.0;
        for _ in 0..xs.len() {
            s += xs[rng.gen_range(0..xs.len())];
        }
        means.push(s / xs.len() as f64);
    }
    means.sort_by(f64::total_cmp);
    (means[reps / 40], means[reps - 1 - reps / 40])
}

/// D-15 — dose–response in the tie-acceptance probability `q`.
///
/// Pre-registered in `PREREG_RC015.md` §9, semantics fixed by
/// `PREREG_RC015_AMENDMENT_1.md` A1.2–A1.4:
/// * non-monotonicity refutes **D-15**, not H-15;
/// * a *reproducible* interior optimum means tie handling is a NON-LINEAR
///   control parameter — a stronger practical result than monotonicity;
/// * `q = 0.5 ≡ G` is decided by a machine criterion, not by eye;
/// * held-out must replicate the curve's SHAPE, not just the endpoints.
fn d15(reg: &OperatorRegistry, seeds: &[u64], label: &str) {
    println!("D-15 — dose–response in the tie probability q  [{label}], flat T={T_COLD}\n");
    for (inst, path) in SET_A.iter().take(1).chain(SET_B.iter().skip(1)) {
        let Ok(ir) = load(path) else { continue };
        println!("  {inst}");
        let mut curve: Vec<(f64, f64)> = Vec::new();
        let mut ys: Vec<Vec<f64>> = Vec::new();
        for q in Q_GRID {
            let mut y = Vec::new();
            for &s in seeds {
                match execute(&ir, reg, &plan_for(&ir, q_name(q), s, T_COLD)) {
                    Ok(o) => y.push(o.post),
                    Err(_) => break,
                }
            }
            if y.len() != seeds.len() {
                println!("    q={q}: run failed");
                return;
            }
            let mean = y.iter().sum::<f64>() / y.len() as f64;
            println!("    q={q:<5} mean Y = {mean:12.4}");
            curve.push((q, mean));
            ys.push(y);
        }

        // Shape: monotone in either direction, or an interior optimum.
        let m: Vec<f64> = curve.iter().map(|c| c.1).collect();
        let nondec = m.windows(2).all(|w| w[1] >= w[0]);
        let noninc = m.windows(2).all(|w| w[1] <= w[0]);
        let best = m
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i)
            .unwrap_or(0);
        let shape = if nondec || noninc {
            "MONOTONE"
        } else {
            "NON-MONOTONE (interior structure)"
        };
        println!(
            "    shape: {shape}   best q = {:.2} (lowest mean Y)   argmin index {best}",
            Q_GRID[best]
        );

        // EXPLORATORY, not pre-registered: does the argmin beat BOTH shipped
        // operators? This is the decision-relevant question for a
        // `neutral_move_rate` policy variable, and it is labelled exploratory
        // rather than folded into D-15's verdict.
        if best != 4 && best != 2 {
            for (rival, rname) in [
                (2usize, "q=0.5 (heat-bath tie rule)"),
                (4, "q=1 (metropolis)"),
            ] {
                let pe = Paired {
                    base: ys[rival].clone(),
                    other: ys[best].clone(),
                };
                println!(
                    "    [exploratory] q={:.2} vs {rname}: {}",
                    Q_GRID[best],
                    pe.fmt()
                );
            }
        }

        // Direction check from the pre-registration: sign(Y(1) − Y(0.5)).
        let d_endpoints = m[4] - m[2];
        println!("    Y(q=1) − Y(q=0.5) = {d_endpoints:+.4}  (D-15 predicted sign = sign(−I_tie))");

        // Machine equivalence: q = 0.5 vs the real G arm (Amendment 1 A1.3).
        let mut yg = Vec::new();
        for &s in seeds {
            match execute(&ir, reg, &plan_for(&ir, G, s, T_COLD)) {
                Ok(o) => yg.push(o.post),
                Err(_) => break,
            }
        }
        if yg.len() == seeds.len() {
            let pe = Paired {
                base: yg.clone(),
                other: ys[2].clone(),
            };
            let diffs = pe.effects();
            let (lo, hi) = bootstrap_ci(&diffs, 2000, 0xD15);
            let scale = (yg.iter().sum::<f64>() / yg.len() as f64).abs().max(1.0);
            let ci_ok = lo.abs() / scale < 0.001 && hi.abs() / scale < 0.001;
            let mat_ok = !pe.rho().is_some_and(|r| r >= 0.5) && pe.rel() < 0.001;
            println!(
                "    q=0.5 vs G: mean diff {:+.4}  CI95 [{:+.4}, {:+.4}] = [{:+.4}%, {:+.4}%]  \
                 CI-inside-0.1%={ci_ok}  non-material={mat_ok}  -> {}",
                pe.mean(),
                lo,
                hi,
                lo / scale * 100.0,
                hi / scale * 100.0,
                if ci_ok || mat_ok {
                    "EQUIVALENT (pass)"
                } else {
                    "NOT EQUIVALENT (D-15 refuted on this clause)"
                }
            );
        }
        println!();
    }
}

fn main() {
    let reg = local_registry();
    let ctrl =
        flag("--controls") || !(flag("--science") || flag("--holdout") || flag("--gradient"));

    if ctrl {
        println!("RC-015 — Gate controls (PREREG_RC015.md §6)\n");
        let Ok(ir) = load(SET_A[0].1) else {
            println!("cannot load G43");
            std::process::exit(2);
        };
        let mut pass = true;
        println!("[1] TieMode::Accept bit-identical to metropolis_sweep");
        pass &= control_null(&ir, &reg);
        println!("\n[2] all three arms leave the generator aligned; shift detected");
        pass &= control_alignment(&ir, &reg);
        println!("\n[3] decomposition closes exactly (I_full − (I_tie + I_off))");
        match measure(&ir, &reg, &HELD_IN, T_COLD, false) {
            Some(c) => {
                println!(
                    "    residual = {:.3e} -> {}",
                    c.closes,
                    if c.closes == 0.0 {
                        "exactly 0 (pass)"
                    } else {
                        "NONZERO (FAIL)"
                    }
                );
                pass &= c.closes == 0.0;
            }
            None => pass = false,
        }
        println!(
            "\nCONTROLS: {}",
            if pass {
                "ALL PASS — science authorised"
            } else {
                "FAILED — instrument invalid, NOT evidence about H-15"
            }
        );
        if !pass {
            std::process::exit(1);
        }
    }

    if flag("--science") {
        println!("\nRC-015 held-in (seeds {HELD_IN:?}), flat T={T_COLD}");
        science(&reg, &HELD_IN, "in");
    }
    if flag("--holdout") {
        println!("\nRC-015 HELD-OUT (seeds {HELD_OUT:?})");
        science(&reg, &HELD_OUT, "out");
    }
    if flag("--gradient") {
        gradient(&reg);
    }
    if flag("--d15") {
        d15(&reg, &HELD_IN, "held-in");
    }
    if flag("--d15-holdout") {
        d15(&reg, &HELD_OUT, "HELD-OUT");
    }
}
