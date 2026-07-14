//! `RunContext` — the execution environment (Constitution §6, §10; ADR-0003).
//! Hardware profile, budget/deadline, seed, experiment id, profiler, logger.
//! Operators never touch this: the Runtime reads it and hands operators a
//! read-only `RuntimeView`. This is the "lab bench", separate from the physics.

use std::collections::HashMap;
use std::time::Instant;

/// Measured / documented properties of the target machine (ADR-0003). Cache
/// sizes are the documented Ryzen 7 values; core count is detected. The
/// sparse backend derives its ensemble width from `l3_bytes`.
#[derive(Clone, Copy, Debug)]
pub struct HardwareProfile {
    pub logical_cores: usize,
    pub cache_line: usize,
    pub l1d_bytes: usize,
    pub l2_bytes: usize,
    pub l3_bytes: usize,
    pub has_avx2: bool,
}

impl HardwareProfile {
    /// The documented target machine (research/adversarial_architecture_review,
    /// ADR-0003): AMD Ryzen 7, AVX2 (no AVX512), 64 B lines, 16 MB shared L3.
    pub const fn target_ryzen() -> Self {
        Self {
            logical_cores: 16,
            cache_line: 64,
            l1d_bytes: 32 * 1024,
            l2_bytes: 512 * 1024,
            l3_bytes: 16 * 1024 * 1024,
            has_avx2: cfg!(target_feature = "avx2"),
        }
    }

    /// Detect what is portably knowable (core count); cache sizes fall back to
    /// the documented profile — no portable API exposes them.
    pub fn detect() -> Self {
        let mut p = Self::target_ryzen();
        if let Ok(n) = std::thread::available_parallelism() {
            p.logical_cores = n.get();
        }
        p
    }
}

/// Per-operator timing/work accounting (Constitution §6). Wall-clock here is
/// diagnostic only and never influences the trajectory (ADR-0004).
#[derive(Clone, Debug, Default)]
pub struct ProfEntry {
    pub calls: u64,
    pub total_ms: f64,
    pub work: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Profiler {
    entries: HashMap<&'static str, ProfEntry>,
}

impl Profiler {
    pub fn record(&mut self, op: &'static str, ms: f64, work: f64) {
        let e = self.entries.entry(op).or_default();
        e.calls += 1;
        e.total_ms += ms;
        e.work += work;
    }

    pub fn entries(&self) -> impl Iterator<Item = (&&'static str, &ProfEntry)> {
        self.entries.iter()
    }
}

/// The run environment. Held by the Runtime, never by operators.
pub struct RunContext {
    pub hardware: HardwareProfile,
    pub seed: u64,
    pub experiment: Option<String>,
    /// Wall-clock guard. Deterministic runs set this to INFINITY and terminate
    /// by sweep budget instead (ADR-0004); this is only a safety cap.
    pub wall_budget_ms: f64,
    pub profiler: Profiler,
    pub log: Vec<String>,
    start: Instant,
}

impl RunContext {
    pub fn new(seed: u64) -> Self {
        Self {
            hardware: HardwareProfile::detect(),
            seed,
            experiment: None,
            wall_budget_ms: f64::INFINITY,
            profiler: Profiler::default(),
            log: Vec::new(),
            start: Instant::now(),
        }
    }

    pub fn with_wall_budget(mut self, ms: f64) -> Self {
        self.wall_budget_ms = ms;
        self
    }

    pub fn with_experiment(mut self, exp: impl Into<String>) -> Self {
        self.experiment = Some(exp.into());
        self
    }

    /// Reset the wall-clock origin (call right before the run loop).
    pub fn start_clock(&mut self) {
        self.start = Instant::now();
    }

    pub fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }

    pub fn remaining_ms(&self) -> f64 {
        (self.wall_budget_ms - self.elapsed_ms()).max(0.0)
    }

    pub fn deadline_hit(&self) -> bool {
        self.wall_budget_ms.is_finite() && self.elapsed_ms() >= self.wall_budget_ms
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        self.log.push(msg.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadline_defaults_to_infinite() {
        let ctx = RunContext::new(7);
        assert!(!ctx.deadline_hit());
        assert!(ctx.remaining_ms().is_infinite());
        assert!(ctx.hardware.logical_cores >= 1);
    }
}
