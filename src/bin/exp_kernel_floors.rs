//! RC-013 — which hardware floor does the sweep kernel sit on?
//!
//! ADR-0003 asserts "cache locality outranks FLOPS in all layout and kernel
//! decisions" — asserted, never measured. Two open decisions hinge on the
//! answer: ROADMAP item 7's remaining `#[target_feature]` AVX2 SAXPY, and the
//! delete-or-implement call on the orphaned `src/core/simd_utils.rs`.
//!
//! ## Design
//!
//! Three microbenchmark floors, measured on this host:
//!   - STREAM triad        -> streaming bandwidth (GB/s)
//!   - random gather       -> dependent random-read cost (ns/access)
//!   - acceptance compute  -> the operator's own inner math (ChaCha8 draw +
//!     branch + exp on cache-resident data), ns per (site,replica) decision
//!
//! Then the real kernel (`MetropolisSweep` on `SparseBitSlice`, whose ledger is
//! `fields[site*r + rep]`, 8 B each, working set 8*n*r) over a grid:
//!
//!   topology  in { ring (neighbors i±1, i±2 — prefetch-friendly),
//!                  random 4-regular-ish (neighbors at random indices) }
//!   n         in { 2k, 8k, 32k, 131k, 524k }   [ledger 0.5 MB .. 134 MB:
//!                                               L2 -> L3 -> DRAM on this host]
//!   T (flat)  in { 0.05 (low phi), 8.0 (high phi) }
//!
//! Two phi points per (topology, n) give the RC-005 decomposition per config:
//! intercept = scan cost per (site,rep), slope = flip cost per accepted flip.
//! The ring-vs-random contrast at the SAME size isolates locality; the size
//! ladder at the SAME topology isolates the cache level.
//!
//! ## Pre-registered predictions
//!
//! P1: the scan intercept is compute-bound — within ~1.5x of the acceptance
//!     floor and ~flat (< +50%) across the ladder (prefetch hides DRAM).
//! P2: the random-graph flip slope rises >= 2x from cache-resident to
//!     DRAM-resident; the ring's stays comparatively flat.
//! P3 (consequence, not measured here): if P1+P2 hold, vectorizing the dense
//!     SAXPY cannot help the sparse path's dominant costs; bulk-RNG could.
//!
//! REFUTED: P1 if the intercept jumps > 50% across the ladder or the compute
//! floor explains < 50% of it; P2 if the random slope is flat across levels.
//!
//! Timing per this host's rules (PERF.md, ~9% drift): conditions interleaved
//! within each repetition, medians reported.
//!
//! ```text
//! cargo run --release --bin exp_kernel_floors
//! ```

use ising_engine::engine_v2::backends::SparseBitSlice;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::operator::{Budget, Operator};
use ising_engine::engine_v2::operators::{MetropolisSweep, RandomFlipSweep};
use ising_engine::engine_v2::runtime::RuntimeView;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::hint::black_box;
use std::time::Instant;

const R: usize = 32;

fn lcg(h: &mut u64) -> u64 {
    *h = h
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *h >> 33
}

/// STREAM-like triad over 3 x 64 MB arrays. Counts 24 B/element (read b, read
/// c, write a; write-allocate traffic not counted — stated, so the floor is a
/// lower bound on true traffic).
fn stream_gbps() -> f64 {
    const N: usize = 8 << 20;
    let mut a = vec![0.0f64; N];
    let b = vec![1.5f64; N];
    let c = vec![2.5f64; N];
    let mut best = f64::INFINITY;
    for _ in 0..5 {
        let t0 = Instant::now();
        for i in 0..N {
            a[i] = b[i] + 1.1 * c[i];
        }
        black_box(&a);
        best = best.min(t0.elapsed().as_secs_f64());
    }
    24.0 * N as f64 / best / 1e9
}

/// Random 8 B reads over a 64 MB array through a shuffled index vector.
/// Independent accesses, so memory-level parallelism is allowed — this is the
/// OPTIMISTIC gather floor, not pointer-chasing latency.
fn gather_ns() -> f64 {
    const N: usize = 8 << 20;
    let data = vec![1.0f64; N];
    let mut idx: Vec<u32> = (0..N as u32).collect();
    let mut h = 0xD1CEu64;
    for i in (1..N).rev() {
        let j = (lcg(&mut h) as usize) % (i + 1);
        idx.swap(i, j);
    }
    let mut best = f64::INFINITY;
    let mut sum = 0.0;
    for _ in 0..3 {
        let t0 = Instant::now();
        for &i in &idx {
            sum += data[i as usize];
        }
        best = best.min(t0.elapsed().as_secs_f64());
    }
    black_box(sum);
    best / N as f64 * 1e9
}

/// The operator's own acceptance math on cache-resident inputs: one ChaCha8
/// f64 draw per decision (MetropolisSweep always draws), branch, exp when
/// uphill. ns per (site,replica) decision — the pure compute floor.
fn accept_ns() -> f64 {
    const M: usize = 20_000_000;
    let mut rng = ChaCha8Rng::seed_from_u64(7);
    let mut ds = [0.0f64; 4096];
    let mut h = 0xACCEu64;
    for d in ds.iter_mut() {
        // Mix of downhill and uphill, magnitudes like integer +/-1 couplings.
        *d = (lcg(&mut h) % 17) as f64 - 8.0;
    }
    let t = 1.0;
    let mut acc = 0u64;
    let t0 = Instant::now();
    for k in 0..M {
        let d = ds[k & 4095];
        let a = if d <= 0.0 {
            let _u: f64 = rng.gen();
            true
        } else {
            let p = (-d / t).exp();
            let u: f64 = rng.gen();
            u < p
        };
        acc += a as u64;
    }
    let dt = t0.elapsed().as_secs_f64();
    black_box(acc);
    dt / M as f64 * 1e9
}

/// Ring: neighbors i±1 and i±2 — adjacent in the ledger, prefetch-friendly.
fn ring_ir(n: usize) -> ProblemIR {
    // from_pairs requires i < j < n: normalize the wrap edges and dedup.
    let mut seen = std::collections::HashSet::new();
    let mut pairs = Vec::with_capacity(2 * n);
    for i in 0..n as u32 {
        let w1 = if i % 2 == 0 { 1.0 } else { -1.0 };
        for (j, w) in [((i + 1) % n as u32, w1), ((i + 2) % n as u32, -w1)] {
            let (a, b) = (i.min(j), i.max(j));
            if a != b && seen.insert((a, b)) {
                pairs.push((a, b, w));
            }
        }
    }
    ProblemIR::from_pairs(n, 0.0, vec![0.0; n], &pairs)
}

/// Random ~4-regular: two random partners per site — neighbor rows land
/// anywhere in the ledger, defeating the prefetcher at DRAM sizes.
fn random_ir(n: usize) -> ProblemIR {
    let mut seen = std::collections::HashSet::new();
    let mut pairs = Vec::with_capacity(2 * n);
    let mut h = 0xBAD5EEDu64;
    for i in 0..n as u32 {
        for k in 0..2u32 {
            let w = if (i + k) % 2 == 0 { 1.0 } else { -1.0 };
            loop {
                let j = (lcg(&mut h) as u32) % n as u32;
                let (a, b) = (i.min(j), i.max(j));
                if a != b && seen.insert((a, b)) {
                    pairs.push((a, b, w));
                    break;
                }
            }
        }
    }
    ProblemIR::from_pairs(n, 0.0, vec![0.0; n], &pairs)
}

/// Median kernel time (seconds) and acceptance for `sweeps` Metropolis sweeps.
fn run_kernel(ir: &ProblemIR, t_flat: f64, sweeps: u32, reps: usize) -> (f64, f64) {
    let temps = vec![t_flat; R];
    let mut init = vec![0u8; ir.n];
    let mut h = 0x1517u64;
    for x in init.iter_mut() {
        *x = (lcg(&mut h) & 1) as u8;
    }
    let mut times = Vec::with_capacity(reps);
    let mut phi = 0.0;
    for rep in 0..reps {
        let mut st = SparseBitSlice::new(ir, R, &init).expect("integral instance");
        let view = RuntimeView {
            iteration: 0,
            temperatures: &temps,
            num_replicas: R,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        let mut rng = ChaCha8Rng::seed_from_u64(rep as u64 + 1);
        // Replica diversity first, else flip masks are all-or-nothing and the
        // dispatch (RC-005) is driven into an unrepresentative regime.
        RandomFlipSweep::new().apply(&mut st, &view, &mut rng, Budget { sweeps: 1 });
        let mut op = MetropolisSweep::new();
        let t0 = Instant::now();
        let rep_out = op.apply(&mut st, &view, &mut rng, Budget { sweeps });
        times.push(t0.elapsed().as_secs_f64());
        phi = rep_out.acceptance_rate();
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (times[times.len() / 2], phi)
}

fn main() {
    println!("# RC-013 kernel floors · SparseBitSlice ledger = 8*n*r bytes, r={R}");
    println!("# host: L1d 32K/core, L2 512K/core, L3 16M shared (lscpu)\n");

    let bw = stream_gbps();
    let g_ns = gather_ns();
    let a_ns = accept_ns();
    println!("# floor 1  STREAM triad        : {bw:.1} GB/s");
    println!("# floor 2  random gather (8B)  : {g_ns:.2} ns/access");
    println!("# floor 3  acceptance compute  : {a_ns:.2} ns per (site,replica)\n");

    let sizes = [2048usize, 8192, 32768, 131072, 524288];
    let temps = [0.05f64, 8.0];

    println!(
        "{:<7} {:>8} {:>9} | {:>8} {:>8} | {:>8} {:>8} | {:>9} {:>10}",
        "topo", "n", "ledger", "phi_lo", "phi_hi", "ns_lo", "ns_hi", "scan ns", "flip ns"
    );

    for topo in ["ring", "random"] {
        for &n in &sizes {
            let ir = if topo == "ring" {
                ring_ir(n)
            } else {
                random_ir(n)
            };
            let sweeps = ((4 << 20) / n).max(4) as u32;
            // Interleave the two temperatures within the same (topo, n) so host
            // drift lands on both equally.
            let (mut ns, mut phis) = ([0.0f64; 2], [0.0f64; 2]);
            for (ti, &t) in temps.iter().enumerate() {
                let (secs, phi) = run_kernel(&ir, t, sweeps, 3);
                ns[ti] = secs / (sweeps as f64 * n as f64 * R as f64) * 1e9;
                phis[ti] = phi;
            }
            // Two-point RC-005 decomposition: ns(phi) = scan + flip*phi.
            let (p0, p1) = (phis[0], phis[1]);
            let (scan, flip) = if p1 > p0 + 1e-6 {
                let slope = (ns[1] - ns[0]) / (p1 - p0);
                (ns[0] - slope * p0, slope)
            } else {
                (f64::NAN, f64::NAN)
            };
            let ledger_mb = 8.0 * n as f64 * R as f64 / 1e6;
            println!(
                "{topo:<7} {n:>8} {ledger_mb:>7.1}MB | {p0:>8.3} {p1:>8.3} | {:>8.2} {:>8.2} | {scan:>9.2} {flip:>10.2}",
                ns[0], ns[1]
            );
        }
        println!();
    }

    println!("# scan ns = cost per (site,replica) at phi=0 — compare to floor 3 ({a_ns:.2} ns).");
    println!("# flip ns = added cost per accepted flip — the RC-005 W_flip term.");
    println!("# P1: scan ~= floor 3 and ~flat down the ladder.  P2: random flip ns");
    println!("# rises >=2x across L2->DRAM while ring stays comparatively flat.");
}
