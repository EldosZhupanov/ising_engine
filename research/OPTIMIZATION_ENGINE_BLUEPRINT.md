# Optimization Engine Blueprint

**Architecture, not algorithm.** This document designs a universal physics-inspired
optimization computation architecture for commodity CPUs — the execution-engine tier
that sits where LLVM sits for compilers and CUDA sits for GPU compute: a substrate of
typed state, a set of interchangeable physical operators, and a control plane that
schedules them. Every design decision below is evaluated by exactly one metric:

> **Does this make the engine fundamentally stronger?**

Not "is it publishable." We reuse ideas from 1953 (Metropolis), 1987 (Swendsen-Wang),
2002 (energy landscape paving), 2019 (momentum annealing), and 2025 (AMFD) without
apology.

Status: DESIGN. Companion documents: `research/PLATFORM_BLUEPRINT.md` (research
platform), `research/adversarial_architecture_review.md` (current-engine bottlenecks),
research validation program results (survivor operators referenced throughout).

---

## 0. Ground truth this design is anchored to

### 0.1 The empirical driver

The current engine (UltimateSolver: byte-per-replica SIMD, 64 replicas × 10
temperatures, PT/DEO) **wins dense QUBO** (ORLIB/Biq Mac) and **loses large sparse**
(G-Set, n ≥ 800) to vanilla single-threaded SA. Root cause (adversarial review,
Rank 1): fixed 64×10 ensemble geometry divides the sweep budget so that G60 gets
~60 sweeps per chain where OpenJij gets ~15,756. The architecture below is the
generalization of that lesson: **ensemble geometry, state layout, and operator choice
must be decisions made per instance by the engine, not constants compiled into a
solver.**

### 0.2 The target machine (measured, not assumed)

| Resource | Value | Architectural consequence |
|---|---|---|
| CPU | AMD Ryzen 7 (16 logical = 8c × 2t, WSL2) | Work units sized to 16, SMT pairs share L1/L2 |
| Vector ISA | **AVX2 + FMA** (256-bit). BMI1/BMI2, POPCNT, VAES, VPCLMULQDQ. **No AVX512** | 32×i8 / 16×i16 / 8×i32 lanes; no masked scatter, no vpopcntdq — popcount via scalar `popcnt` on extracted quadwords or Muła SSE/AVX2 nibble-LUT |
| No i8 SIMD multiply | LLVM emits vpmovzxbw+vpmullw+vpshufb | Replace multiplies with XOR/SUB identities (already deployed: `(x_j^x_v)-x_v`) |
| Cache line | 64 B | One line = **512 bit-sliced replicas** of one spin |
| L1d / L2 | 32 KB / 512 KB per core | Kernel working sets (RNG state, threshold tables) must be L1-resident |
| L3 | **16 MB shared** | The "chip" — the whole hot state of a sparse instance must fit here |
| Timing | WSL2, ~9% same-host drift | A/B only back-to-back; integer-exact trajectories make correctness A/B timing-independent |

### 0.3 The landscape gap (verified by search)

- **Hardware Ising machines** (D-Wave, Fujitsu DA, Toshiba SBM, NTT CIM) have
  vendor HALs but fixed-function dynamics — one physical process each, in silicon.
- **MQLib** is a zoo of 37 heuristics with *no shared computational substrate* —
  every heuristic re-implements its own state, moves, and bookkeeping.
- **dimod/Ocean** is a *model* API (how to describe a QUBO), not an *execution*
  engine (how to compute on it).
- **Nobody has built the middle tier**: an execution engine where physical dynamics
  are interchangeable operators over a shared, CPU-native state substrate. That
  tier is this blueprint.

---

## 1. RQ1 — Computational primitives, organized into layers

A *primitive* is the smallest operation with a defined contract on state. Operators
(§2) are compositions of primitives; algorithms are schedules of operators.

### P0 — Bit/word primitives (the "ALU" of the engine)

| Primitive | Contract | CPU cost |
|---|---|---|
| `xor_plane(a,b)→c` | bitwise disagreement of two spin planes | 1 vpxor / 256 spins |
| `popcount_plane(a)→n` | count set bits | ~1.3 cyc / 256 bits (Muła nibble-LUT + vpsadbw) |
| `select_mask(m,a,b)` | branch-free blend | vpblendvb, 1/cyc |
| `bitslice_transpose` | 64×64 bit matrix transpose (replica↔site major) | BMI2 pdep/pext or Hacker's-Delight shuffle network |
| `rng_stream_u64x4` | counter/xoshiro vectorized streams, per-lane-group | already deployed (Xoshiro256++×8) |
| `bernoulli_int(u, thr)` | accept iff `u < thr` (u32 vs precomputed integer threshold) | 1 vpcmpgtd — **no exp() in the hot path** |
| `prefix_scan` | cumulative sums (rejection-free rate tables) | log-depth AVX2 shuffles |
| `masked_reduce` | horizontal min/sum under mask | vphaddw / vpminsw trees |

### P1 — Field primitives (the fundamental invariant)

The engine's core invariant: **local fields are always current**:
`h_i = l_i + Σ_j q_ij x_j`, so `ΔE(flip i)` is O(1) to read and O(deg i) to maintain.

| Primitive | Contract |
|---|---|
| `field_read(i) → ΔE` | O(1) per replica lane |
| `field_scatter(i)` | after flip of i: `h_j ± 2·q_ij` for all neighbors j — **the** sparse hot loop |
| `field_rebuild` | full recompute; validation + drift audit (integer fields never drift) |
| `field_plane(i)[r]` | multi-replica field row: 512 replicas × i16 = 1 KB = 16 cache lines |

### P2 — Spin/move primitives

single-spin Metropolis test · heat-bath (Gibbs) test · **color-class batch update**
(all sites of one chromatic class, all replicas, synchronously) · rejection-free
selection (pick move ∝ rate; scan-based KMC) · k-flip composite evaluation ·
cardinality-preserving swap · backbone freeze/unfreeze (mask a variable out of all
kernels) · cluster grow (Wolff/SW bond percolation) · isoenergetic cluster move
(Houdayer: connected component of the XOR plane of two replicas — built from P0
`xor_plane` + BFS).

### P3 — Ensemble primitives

replica exchange test (PT) · **DEO deterministic-even-odd non-reversible sweep**
(deployed) · population resample with weights (PA) · overlap `q_ab = 1 − 2·popcount(xor)/n`
(free via P0!) · best-replica extraction (deployed, de-interleaved) · replica
recombination (take agreeing bits, re-randomize disagreeing — ICM crossover).

### P4 — Landscape/ledger primitives

history field `V(x)` accumulation (metadynamics bias on collective variables) ·
tabu ledger (recency/frequency) · energy histogram (Wang-Landau / multicanonical
weights) · frustration detector (odd-cycle / plaquette parity via XOR around cycles)
· autocorrelation & round-trip-time estimators · acceptance-rate counters (per
temperature, per operator) · overlap histogram P(q) (OGP signature input).

### P5 — Exact-inference primitives

exhaustive solve ≤ ~40 vars (Gray-code enumeration, ΔE incremental) · **tree DP**
over low-treewidth induced subgraphs (tropical/min-plus semiring) · QPBO/roof-duality
persistency (deployed as presolve; here also as an *in-loop* primitive on subproblems)
· max-flow on submodular subregions · DP on induced paths/cycles (exact 1D chains).

### P6 — Structural primitives

greedy graph coloring (enables P2 color-class batches) · degeneracy/BFS reorder
(cache locality) · connected components · community detection (Louvain-lite) for
decomposition · separator finding · **condensation/renormalization**: contract a
locked cluster into a supervariable, produce the renormalized instance ·
frozen-core detection (variables identical across replicas & time → candidates for
conditioning).

### P7 — Control primitives

temperature-ladder placement (equal acceptance / round-trip flow) · budget
allocator (sweeps ↔ replicas ↔ temps as a *resource vector*, cf. §0.1) · restart
policy · operator bandit (UCB over operator success-per-second) · convergence
detector · OGP/overlap-gap diagnostic (bimodal P(q) with forbidden middle →
clustering phase → switch strategy from equilibration to restarts+recombination).

**Layering rule:** a primitive at layer k may call only layers < k. Operators
compose primitives; the scheduler composes operators. This is the same discipline
that makes LLVM passes composable.

---

## 2. RQ2 — Physical phenomena as computational operators

For each phenomenon: *what does the physics compute for free?* → *the operator it becomes.*

| # | Phenomenon (field) | What it computes naturally | Operator |
|---|---|---|---|
| 1 | Thermal fluctuation (stat mech) | Boltzmann sampling; barrier crossing ∝ e^(−βΔE) | `MetropolisSweep`, `GibbsColorSweep` |
| 2 | Phase transition / criticality | Divergent susceptibility near T_c = maximal information flow per flip | Ladder auto-placement clustered around measured C_v peak (`LadderTuner`) |
| 3 | Non-equilibrium driving (irreversible processes) | Non-reversible chains mix ≥ reversible; momentum through state space | `DEO` (deployed), lifted single-spin dynamics (Turitsyn-Chertkov-Vucelja skew) — `LiftedSweep` |
| 4 | Nucleation & domain growth (condensed matter) | Collective flips of correlated regions — moves no single-spin dynamic makes | `HoudayerICM` (isoenergetic cluster), `SWCluster` (near T_c only) |
| 5 | Catalysis / rare-event acceleration (chemical physics) | Fill visited basins so dynamics is repelled from them | `HistoryField` — metadynamics / energy-landscape-paving bias (survivor of validation program; gated on measured revisit statistics) |
| 6 | Percolation | Cluster-size distribution = proximity to critical connectivity | percolation probe feeding `SWCluster` on/off gate (SW clusters percolate uselessly below T_c) |
| 7 | Renormalization group | Coarse-graining preserves low-energy physics, shrinks the problem | `Condense`: lock high-confidence correlated blocks → supervariable instance → recurse |
| 8 | Spin-glass theory: overlaps, RSB, temperature chaos | P(q) structure reveals landscape geometry; chaos ⇒ solutions at nearby T are uninformative | `OverlapProbe` (free, P0), `OGPController` (research-track survivor): bimodal-gap detection switches explore/exploit regime |
| 9 | Population dynamics / nonequilibrium quench (Jarzynski) | Resampling keeps the ensemble on the equilibrium ridge through the transition | `PopulationAnneal` (PA-PT already in repo, opt-in; promoted to first-class operator) |
| 10 | Synchronization (Kuramoto), Kerr-nonlinear oscillators | Continuous relaxation finds smooth descent basins discrete moves can't see | `ContinuousRelax`: momentum-annealing / simulated-bifurcation style f32 dynamics used **only** as an initializer/polisher producing spin sign patterns — never the trajectory of record |
| 11 | Reaction-diffusion / wavefronts | Locality of propagation = cache locality of computation | wavefront-ordered sweeps: update frontier of recently-flipped spins first (`ActiveList` scheduling, = adaptive variant of event-driven MC) |
| 12 | Self-organized criticality (Bak-Sneppen) | Power-law selection of the *worst* element drives avalanches through barriers without a temperature | `ExtremalOpt` (τ-EO, Boettcher-Percus): flip rank-selected worst-field spins; complements thermal ops at ΔE plateaus |
| 13 | Belief propagation (info theory / stat mech duality) | Marginals on trees exactly; strong hints on loopy sparse graphs | `BPWarmstart`: damped BP → per-site magnetization → biased init + freeze candidates |
| 14 | Hamiltonian dynamics / symplectic flows | Energy-conserving exploration, momentum carries over barriers | folded into #10 (SB is the discrete-time symplectic case) |
| 15 | Optimal transport / info geometry | Minimal-distortion redistribution of ensemble mass | PA resampling with systematic (low-variance) resampler = discrete OT step; ladder spacing along constant-KL steps |
| 16 | Neural: synaptic pruning, Hebbian traces | Sparsify what doesn't matter; amplify what repeats | coupling-dilution during exploration (park: behavior-changing, weak evidence); frequency ledger already covered by P4 |
| 17 | Morphogenesis / Turing patterns | Pattern formation from local activation-inhibition | **parked** — no credible mapping to energy minimization better than #4/#5 |

Filter applied: an operator earns a place only if (a) it computes something the
substrate makes cheap (e.g. overlaps via XOR-popcount), or (b) it makes moves no
other operator makes (clusters, history bias, extremal avalanches, exact DP).
Items 16–17 are recorded and parked, not deleted.

---

## 3. RQ3 — The modular execution engine (8 layers)

```
┌───────────────────────────────────────────────────────────────────────┐
│ L8  PROBLEM PLUGINS      QUBO · Ising · Max-Cut · HUBO→quadratization │
│                          · penalties · featureizer · answer mapping   │
├───────────────────────────────────────────────────────────────────────┤
│ L7  META-OPTIMIZATION    portfolio selection by instance features ·   │
│                          per-family tuned configs · experiment        │
│                          registry link (PLATFORM_BLUEPRINT)           │
├───────────────────────────────────────────────────────────────────────┤
│ L6  EXACT SUBSOLVERS     QPBO/persistency · tree-DP (tropical) ·      │
│                          ≤40-var enumeration · exact LNS finisher     │
├───────────────────────────────────────────────────────────────────────┤
│ L5  ADAPTIVE CONTROL     sensors: acceptance, round-trips, P(q),      │
│                          C_v, revisit stats → controllers: ladder,    │
│                          budget vector, operator bandit, OGP switch   │
├───────────────────────────────────────────────────────────────────────┤
│ L4  SCHEDULER            phase pipeline · operator graph executor ·   │
│                          thread/work-unit placement (16 threads) ·    │
│                          budget accounting (wall-clock contract)      │
├───────────────────────────────────────────────────────────────────────┤
│ L3  PHYSICS OPERATORS    MetropolisSweep · GibbsColorSweep ·          │
│                          RejectionFree · DEO · HoudayerICM · PA ·     │
│                          HistoryField · ExtremalOpt · BPWarmstart ·   │
│                          ContinuousRelax · Condense                   │
├───────────────────────────────────────────────────────────────────────┤
│ L2  SPIN ENGINE          state layouts: DenseByte (current engine) ·  │
│                          SparseBitSlice (new) · field maintenance ·   │
│                          move application · layout-selection pass     │
├───────────────────────────────────────────────────────────────────────┤
│ L1  BIT ENGINE           planes · PRNG streams · popcount/XOR/blend   │
│                          kernels · transpose · threshold tables ·     │
│                          memory pools · thread pinning                │
└───────────────────────────────────────────────────────────────────────┘
```

### Responsibilities and interfaces

**L1 Bit Engine.** Owns memory. Exposes `Plane` (bit matrix, site-major,
64 B-aligned), `FieldPlane<i16/i32>`, `RngStreams`, and the P0 kernel set. No
physics. Contract: every kernel is branch-free in the lane dimension and
deterministic given stream seeds.

**L2 Spin Engine.** Owns the *state* abstraction:

```rust
trait SpinState {
    fn delta_e(&self, site: usize) -> FieldRow;       // per-replica ΔE, O(1)
    fn apply_flips(&mut self, site: usize, mask: ReplicaMask); // + field_scatter
    fn energy(&self) -> EnergyRow;                    // ledger, not recompute
    fn overlap(&self, a: Replica, b: Replica) -> f64; // P0 xor+popcount
    fn checkpoint(&self) -> StateDigest;              // content-addressed
}
```

Two concrete layouts, chosen by a **backend-selection pass** (like LLVM ISel):

- `DenseByte` — the existing engine: 1 spin = 1 i8, 64 replicas interleaved,
  f64 couplings. Optimal when the coupling matrix is dense (fields change for
  *all* sites per flip anyway; bandwidth is spent on the J row, not the state).
- `SparseBitSlice` — new: 1 spin = 1 bit × R replicas per cache line, integer
  couplings (quantized when input is float, with exactness certificate when the
  input is already integral — G-Set, ORLIB, most Biq Mac are integral), i16/i32
  field planes, chromatic synchronous updates. Optimal when deg ≪ n.

Selection rule (initial, to be measured): density ≥ ~5% or non-quantizable
couplings → `DenseByte`; else `SparseBitSlice`.

**L3 Physics Operators.** Uniform contract:

```rust
trait Operator {
    fn apply(&mut self, s: &mut dyn SpinState, ctx: &Ctx, budget: Budget) -> Report;
    fn cost_model(&self, s: &InstanceStats) -> CostEstimate; // for the scheduler
}
```

`Report` carries the sensor data L5 needs (acceptance counts, cluster sizes,
energy trace summaries). Operators never allocate in `apply` (pools from L1),
never talk to each other directly — composition happens only through state and
through L4.

**L4 Scheduler.** Executes an **operator schedule** — a small program:

```
phase presolve:  QPBO → components → coloring → BPWarmstart?
phase explore:   repeat { GibbsColorSweep ×k @ ladder; DEO; every m: HoudayerICM }
phase exploit:   PA collapse to low-T population; RejectionFree at T≈0
phase finish:    ExactLNS(tree-DP) until budget; extract best
```

Thread model: work units are (temperature-block × replica-block) pairs sized so
that active units ≈ 16 and each unit's state slice fits the local L2. The 10-task
grain of the current engine (adversarial review Rank 4) is replaced by
budget-vector-driven decomposition.

**L5 Adaptive Control.** Sensors are free or amortized (they read `Report`s and
P0-cheap probes). Controllers adjust: ladder geometry, the (replicas × temps ×
sweeps) budget vector, operator weights (bandit over marginal energy gain per
second), and the explore/exploit switch (OGP signature: when P(q) develops a
forbidden gap, equilibration stalls by construction — reallocate to restarts +
recombination + exact finishing).

**L6 Exact Subsolvers.** Called as operators. The **ExactLNS finisher** (survivor
of the validation program): pick a low-treewidth region around high-|field|
frustrated sites (P6 separators), condition on the boundary, solve exactly by
tropical tree-DP, re-inject. Guarantees monotone improvement — the correct
last-mile operator when thermal acceptance has collapsed.

**L7 Meta-Optimization.** Offline: per-family tuned schedules stored as config
records keyed by instance features (n, density, degree CV, coupling spectrum,
integrality, frustration index). Online: cheap featureization → schedule
selection. Links to the experiment registry so every schedule's provenance is a
claim with an anchor (PLATFORM_BLUEPRINT discipline).

**L8 Problem Plugins.** Frontends normalize to the canonical IR (below). Answer
mapping restores the user's variable space, applies the validated objective
conventions (see memory: ORLIB ×2 MAXIMIZE, Biq Mac ×2 MINIMIZE, QPLIB ½-factor).

### The IR (what makes this "LLVM for optimization")

```
ProblemIR {
    n, offset,
    linear:  Vec<Coeff>,           // Coeff = exact integer | scaled integer + cert | f64
    quad:    CSR { idx: u32, w: Coeff },   // symmetric, both triangles materialized
    meta:    { components, coloring, degeneracy_order, integrality, density,
               frustration_sample, treewidth_probe }
}
```

Lowering passes (each pass: IR → IR + certificate):
1. **Normalize** (dedupe, symmetrize, drop zeros)
2. **Persistency** (QPBO/roof duality → fix variables, shrink)
3. **Decompose** (connected components → independent sub-IRs)
4. **Quantize** (float → integer couplings when exact or within certified ε)
5. **Reorder** (degeneracy/BFS for cache locality; stored permutation)
6. **Color** (chromatic classes for synchronous updates)
7. **Backend-select** (DenseByte vs SparseBitSlice + budget-vector prior)

Every pass emits a machine-checkable certificate (energy-preserving bijection),
consistent with the claim-anchored engineering rules of the platform blueprint.

### Data movement (one flip, SparseBitSlice, R = 512 replicas)

```
site i chosen by color-class schedule (same site, all replicas — synchronous)
  read  field_plane(i): 512 × i16 = 1 KB = 16 lines   (L3-resident)
  ΔE lanes = 2 · s_i ⊙ h_i           (XOR-sign trick, no multiply)
  accept mask: ΔE ≤ 0  OR  u32 < thr[ΔE]              (L1 threshold table)
  flip:  spin_plane(i) ^= mask                        (1 line touched)
  scatter: for j in N(i):  field_plane(j) ±= 2·J_ij under mask
           deg × 16 lines, masked i16 add/sub (vpblendvb + vpaddw)
```

Per-replica per-spin cost ≈ deg/16 vector ops. For G60 (n = 7000, deg ≈ 5):
spins 448 KB + i16 fields 7.2 MB + CSR 0.2 MB ≈ **7.9 MB — the entire hot state
of 512 replicas of a 7000-spin instance is L3-resident.** This is the sparse
counter-punch: the current engine loses G-Set by sweep starvation; this layout
buys ~8× state bandwidth (bit vs byte) × wider ensembles × integer arithmetic.

---

## 4. RQ4 — CPU-native computation: what the CPU does almost for free

| Free computation | Instructions | Redesigned optimization use |
|---|---|---|
| 256 spin-pair comparisons / ~2 cyc | vpxor + popcount | replica overlap q_ab, Hamming distances, Houdayer XOR plane, P(q) histograms — ensemble diagnostics at ~zero cost |
| ±J energy terms as parity | XOR of endpoint planes, popcount | bit-sliced exact energy audit for ±1-coupling instances (G-Set): E = m − 2·sat_edges, no arithmetic per edge |
| Branch-free 32-way select | vpblendvb | all acceptance/flip application — zero mispredicts in lane dimension |
| 16 × i16 add per instr, 3–4/cyc | vpaddw/vpsubw | integer field scatter — the sparse hot loop becomes pure i16 adds |
| Sign application w/o multiply | vpxor + vpsubb (XOR-SUB) | deployed; generalizes to all ±1 modulation |
| Nibble LUT, 32 lookups/instr | vpshufb | popcount (Muła), small-alphabet ΔE→threshold class mapping |
| Bernoulli w/o exp() | vpcmpgtd vs table | integer thresholds thr[ΔE] = ⌊2³²·e^(−βΔE)⌋; ±1 couplings ⇒ ΔE ∈ small finite set ⇒ table is L1-resident (few KB per temperature) |
| Bit pack/unpack | BMI2 pdep/pext | bit-slice transpose at replica extraction, mask compression |
| 64 B line = 512 replicas | cache geometry | ensemble width is *free bandwidth*, not extra traffic — the central layout bet |
| SMT latency hiding | 2 threads/core | pair a scatter-heavy unit with a compute-heavy unit per core |
| Hardware prefetch on streams | linear CSR walks after BFS reorder | P6 reorder pass converts random neighbor access into near-streaming access |

Anti-patterns on this machine (measured or ISA-verified): AVX2 gathers (slow,
use only for L1-resident tables), i8 multiplies (no instruction), per-site f64
RNG (already replaced by JIT acceptance RNG), float accumulation in trajectories
(reordering hazard — integer fields eliminate the entire class).

**Design consequence:** the sparse backend is an *integer machine*. Trajectories
are exactly reproducible by construction, immune to FMA/reassociation, and the
bit-identical A/B discipline gets cheaper to enforce, not harder.

---

## 5. RQ5 — Unexploited computational phenomena (recorded, not yet judged)

1. **Bandwidth-as-ensemble**: bit-slicing converts memory bandwidth into replica
   count. Exploited by MSC in the 1980s and Fujitsu in silicon; almost absent in
   open CPU solvers. (→ core of v1.)
2. **L3 as the computer**: choose R so state+fields exactly fill 16 MB. Ensemble
   width becomes a *derived* quantity of cache geometry per instance.
3. **Non-reversible single-spin dynamics** (lifted chains, skew detailed balance)
   beyond replica-level DEO — provable mixing speedups, near-zero extra cost.
4. **Event-driven / active-list sweeps**: after the transient, most spins are
   frozen; maintaining a frontier of recently-influenced sites turns O(n) sweeps
   into O(active). Interacts badly with synchronous bit-slicing → applies to the
   low-T exploit phase, per-replica-block.
5. **Scan-based rejection-free at T≈0**: when acceptance < ~1%, select the move
   by SIMD prefix-scan over rates instead of rejecting 99 draws.
6. **Self-organized criticality as a temperature-free operator** (τ-EO): rank
   selection over local fields; avalanches without ladder tuning; known to beat
   SA on some spin glasses at large n.
7. **History fields in discrete landscapes** (metadynamics/ELP): well explored in
   MD, barely in QUBO. Gated on the measured revisit precondition (validation
   program verdict: PROTOTYPE).
8. **Frozen-core conditioning / dynamic decomposition**: replicas agreeing on a
   variable across time ⇒ condition on it, re-decompose the residual graph, recurse.
   The graph *changes shape* during the run (adaptive topology).
9. **Overlap fields as control signals**: because q_ab is free (P0), the engine
   can afford landscape geometry sensing *continuously* — OGP-aware scheduling.
10. **Integer-exact determinism as a feature**: reproducibility is normally a tax;
    an integer substrate makes it free and makes A/B verification exact.

Items 4–6 need experiments before they earn operator status; 1–3 and 8–10 are
structural and enter v1 directly; 7 enters as a gated prototype.

---

## 6. RQ6 + RQ7 — Optimization Architecture v1: the operator set, evaluated

Thirteen operators. Columns: physics origin; math model; per-sweep complexity
(R = replicas, m = edges, n = spins, χ = colors); memory pattern; SIMD fit;
sparse/dense suitability; difficulty (1–5); expected gain (vs current engine,
hypothesis to be measured — **never claimed without A/B**).

| Operator | Physics | Model | Complexity | Memory | SIMD | Sparse/Dense | Diff | Expected gain |
|---|---|---|---|---|---|---|---|---|
| `GibbsColorSweep` | heat-bath thermalization | P(s_i=1)=σ(2βh_i), chromatic synchronous | O(m·R/lane) | streaming rows, L3-resident | ★★★★★ (the layout is built for it) | sparse ★★★★★ / dense ★★ | 3 | **the** G-Set fix: ~8–30× effective sweep throughput (hypothesis; core v1 bet, P(win)≈0.6 from validation) |
| `MetropolisSweep` | thermal fluctuation | accept e^(−βΔE) | O(m·R/lane) | as above | ★★★★★ | both | done (DenseByte) | baseline |
| `RejectionFree` | kinetic MC / Gillespie | pick i ∝ rate_i | O(n + deg) per event | scan + scatter | ★★★ (prefix scan) | sparse ★★★★ low-T | 3 | exploit-phase only; large when acceptance <1% |
| `DEO` | non-equilibrium driving | deterministic even-odd swaps, round-trip flow | O(T·R) | tiny | ★★★★ (masked lane swaps, deployed) | both | done | in production |
| `LiftedSweep` | irreversible dynamics | skew detailed balance, momentum bit per replica | O(m·R/lane) | +1 bit plane | ★★★★ | sparse ★★★ | 2 | provable mixing ↑; cheap experiment |
| `HoudayerICM` | domain nucleation | flip connected component of XOR plane between same-T replicas | O(component) | BFS over CSR | ★★ (BFS scalar; XOR plane free) | sparse ★★★★★ / dense ★ | 3 | known large gains on spin glasses (Zhu-Ochoa-Katzgraber); in repo opt-in, promote + re-benchmark |
| `SWCluster` | percolation @ T_c | FK bond activation p=1−e^(−2βJ) | O(m·R) | union-find | ★★ | frustrated: weak (gated by percolation probe) | 3 | narrow but real near T_c on structured instances |
| `PopulationAnneal` | nonequilibrium quench | resample w ∝ e^(−ΔβE), systematic resampler | O(R) per β-step | replica copy (masked, deployed pattern) | ★★★★ | both | 2 (exists opt-in) | equilibration through the transition; replaces fixed ladder geometry at large n |
| `HistoryField` | metadynamics / catalysis | V(cv) += w·K(cv); ΔE′ = ΔE + ΔV | O(1)/flip + cv maintenance | small dense table | ★★★ | both | 3 | gated: only if revisit statistics confirm (validation verdict PROTOTYPE) |
| `ExtremalOpt` | self-organized criticality | flip rank^(−τ)-selected worst field | O(n log n) heap or bucket | heap / bucket lists | ★★ | sparse ★★★★ | 2 | temperature-free plateau breaker; cheap to test |
| `BPWarmstart` | belief propagation | damped min-sum marginals | O(m) per iter | message arrays | ★★★ | sparse ★★★★ / dense ✗ (dense BP diverges) | 3 | better-than-random init + freeze hints; init-only, preserves trajectory discipline |
| `ContinuousRelax` | Kuramoto/Kerr oscillators, momentum annealing | ẍ = (p−1)x − x³ + c·Σ J x (SB-form) | O(m·R) f32 | dense-friendly | ★★★★★ (FMA) | dense ★★★★ / sparse ★★★ | 4 | init/polish only; prior art (Toshiba SBM, Okuyama MA) shows large dense gains; **explicitly behavior-changing → separate operator, never inside thermal trajectories** |
| `ExactLNS` | — (exact inference) | tropical tree-DP on conditioned low-tw region | O(2^tw·region) | DP tables L2-resident | ★ (scalar DP) | sparse ★★★★★ | 4 | monotone last-mile improvement; validation verdict PROTOTYPE; prior art: Selby's exact-subgraph solver held G-Set records |

**Interaction matrix (key compositions and conflicts):**

- `GibbsColorSweep` × `DEO` × `PopulationAnneal` — the equilibration backbone; compose freely.
- `HoudayerICM` requires ≥2 replicas per temperature — the budget vector must
  guarantee pairs (constraint fed to L4).
- `HistoryField` modifies ΔE ⇒ composes with any sweep operator but **invalidates
  detailed balance** w.r.t. the bare Hamiltonian — flagged behavior-changing;
  final energies always re-scored against the bare model by the canonical scorer.
- `ExtremalOpt` and `RejectionFree` are both exploit-phase; bandit chooses.
- `ContinuousRelax` and `BPWarmstart` are init-phase only; outputs enter as spin
  configurations, so downstream bit-identical discipline is unaffected.
- `Condense` (renormalization) wraps *any* inner schedule on the contracted
  instance — it is a higher-order operator.

---

## 7. RQ8 — Prior art per operator: study, extract, improve, integrate

| Operator | Prior art (studied) | Strengths to keep | Failure to fix | Our improvement |
|---|---|---|---|---|
| SparseBitSlice substrate | Multi-spin coding (1980s, Ito/Kanada); Fujitsu Digital Annealer (silicon); Isakov et al. `an_ms` codes (2015) | 8× density; MSC codes are the fastest CPU SA known | MSC classically restricted to same-J models & shared randomness (correlates replicas); DA is fixed-function | integer field planes + per-replica JIT RNG (our deployed pattern) → independent replicas, general integer couplings |
| GibbsColorSweep | chromatic Gibbs (Gonzalez et al. 2011, ML); hardware annealers' parallel updates | exact parallelism within a color class | ML literature targets marginals, not optimization schedules | fuse with ladder/PA control; colors reordered for cache streaming |
| DEO | Syed et al. 2021 (non-reversible PT) | round-trip rate ↑ | — | deployed |
| HoudayerICM | Houdayer 2001; Zhu-Ochoa-Katzgraber 2015 (ICM) | best-known sparse spin-glass moves | dense graphs: clusters span everything (useless) | density gate + XOR-plane construction on the bit substrate (component finding on words, not spins) |
| PopulationAnneal | Hukushima-Iba 2003; Machta 2010; Weigel et al. | equilibrium through transitions; embarrassingly parallel | naive multinomial resampling adds variance | systematic (OT-optimal) resampler; PA geometry replaces the fixed 64×10 (root cause §0.1) |
| HistoryField | Wang-Landau; metadynamics (Laio-Parrinello 2002); ELP (Hansmann-Wille 2002); Guided Local Search | proven barrier escape in MD; GLS won MQLib categories | CV choice unclear in discrete landscapes; fills memory | validation-gated prototype: CV = (energy, distance-to-best) pair; deposit only on measured revisits |
| ExtremalOpt | Boettcher-Percus 2001 | no schedule to tune; strong on Gaussian spin glasses | O(log n) heap per flip; τ sensitivity | bucketed integer fields (±1 couplings ⇒ few field values ⇒ O(1) buckets) |
| BPWarmstart | min-sum BP; survey propagation (Mézard-Parisi-Zecchina) | near-exact on locally tree-like (G-Set random cuts!) | divergence on loopy/dense | damping + init-only role (no correctness exposure) |
| ContinuousRelax | Toshiba SBM (Goto 2019/2021); Okuyama momentum annealing 2019; AMFD 2025; SimCIM | state-of-the-art dense MaxCut throughput | needs float, breaks reproducibility discipline; quality plateaus | quarantined as init/polish operator producing configurations only; integer re-score |
| ExactLNS | Selby 2014 (exact subgraph, held G-Set records); Hamze-de Freitas trees; qbsolv decomposition | monotone; provably optimal locally | region selection is the art; Selby's regions were static | frustration+field-guided dynamic region selection; boundary conditioning from replica consensus |
| QPBO presolve | Rother et al. 2007; roof duality | free persistencies | dense frustrated: fixes ~0 | deployed; extend to in-loop use on subregions |
| OGPController | Gamarnik-Sudan; PNAS 2021 | *explains* hardness; predicts when equilibration is wasted | descriptive theory, not an algorithm | novel-as-control: P(q) sensor (free on this substrate) → phase switch; research track |
| MQLib zoo | Dunning-Gupta-Silberholz 2018 | 37 heuristics, honest benchmarking | no shared substrate; each heuristic bespoke | our whole thesis: heuristics become *schedules over shared operators* |

Nothing above is rejected for existing. Existence is evidence it works.

---

## 8. RQ9 — The Blueprint

### 8.1 Execution pipeline

```
            ┌────────────┐   lowering    ┌─────────────┐  backend   ┌──────────────┐
 model ───▶ │ ProblemIR  │──passes+certs▶│ LoweredIR   │──select───▶│ ExecutionPlan │
 (L8)       │ normalize  │               │ persistency │            │ layout+budget │
            └────────────┘               │ components  │            │ +schedule     │
                                         │ quantize    │            └──────┬───────┘
                                         │ reorder     │                   │
                                         │ color       │                   ▼
                                         └─────────────┘        ┌────────────────────┐
                                                                │ RUNTIME (L4+L5)    │
   sensors: acceptance, C_v, P(q), round-trips, revisits        │ presolve → explore │
   ◀──────────────────────────────────────────────────────────  │ → exploit → finish │
   controllers: ladder, budget vector, operator bandit,         │ 16 work units      │
   OGP explore/exploit switch                                   └─────────┬──────────┘
                                                                          │
                                                                          ▼
                                              best config + energy + certificates
                                              (canonical scorer, claim-anchored)
```

### 8.2 Operator graph (default sparse schedule)

```
BPWarmstart ─┐
             ├─▶ GibbsColorSweep ⟲ ──▶ DEO ──▶ (every m sweeps) HoudayerICM
QPBO fixes ──┘        │                              │
                      ▼ (C_v sensor)                 ▼ (P(q) sensor)
              LadderTuner / PopulationAnneal   OGPController ──▶ regime switch
                      │                                              │
                      ▼  (acceptance < 1%)                           ▼
              RejectionFree / ExtremalOpt ──────────────▶ ExactLNS ──▶ best
```

Dense schedule differs: `ContinuousRelax` init → `MetropolisSweep`(DenseByte) →
DEO → exploit → finish. Same runtime, different plan — that is the architecture
working.

### 8.3 Memory layout (SparseBitSlice, per component)

```
spin planes    : n_sites × R bits          site-major, 64 B rows = 512 replicas
field planes   : n_sites × R × i16         1 KB row/site, L3 budget-derived R
couplings      : CSR u32 idx + i8/i16 w    BFS-reordered, streamed
threshold tabs : per-T, few KB             L1-resident (integer Bernoulli)
ledgers        : energy i64 × R, tabu/history small planes
RNG            : Xoshiro256++ ×8 streams   per work-unit, L1-resident
─────────────────────────────────────────────────────────────────
R chosen so total ≤ ~14 MB (L3 minus slack); G60 example: R=512 → 7.9 MB
```

### 8.4 Scheduler core (pseudocode)

```rust
loop {
    let unit = budget.next_work_unit();            // (temp_block × replica_block)
    let op   = bandit.pick(phase, unit, cost_models);
    let rep  = op.apply(&mut state[unit], &ctx, unit.budget);
    sensors.ingest(rep);
    if sensors.phase_transition_signal() { phase = controller.advance(phase); }
    if budget.wall_clock_exhausted()     { break; }
}
finish: exact_lns.polish(&mut best); score_canonical(&best);
```

### 8.5 Verification contract (non-negotiable, inherited)

- Integer substrate ⇒ trajectories exactly reproducible; golden regression
  byte-identical; A/B via `ab_engine_compare.py` asserts identical energies.
- Behavior-changing operators (`HistoryField`, `ContinuousRelax`, quantization
  with ε) are **typed as such in the plan**; they may never silently enter a
  bit-identical comparison; final energies always re-scored by the canonical
  scorer against the bare model.
- Every lowering pass ships a certificate; every performance claim ships a
  measured A/B (>1% geomean or it doesn't merge).

### 8.6 Roadmap (each phase gated by measurement, not by completion)

| Phase | Deliverable | Gate to proceed |
|---|---|---|
| **v0.1** | `SparseBitSlice` substrate + `GibbsColorSweep` + integer thresholds + coloring/reorder passes; standalone binary | beats OpenJij SA at equal wall-clock on ≥ half of G-Set n ≤ 2000, seeds/budgets per benchmark protocol |
| **v0.2** | `Operator` trait; port DEO + ladder onto both backends; UltimateSolver becomes the DenseByte backend behind the same interface (no rewrite of legacy solvers) | golden + full 129 tests + dense results unchanged (bit-identical) |
| **v0.3** | `HoudayerICM` on XOR planes + `PopulationAnneal` geometry replacing fixed 64×10 | measured gain on G-Set n ≥ 800 vs v0.1 |
| **v0.4** | `ExactLNS` (tree-DP finisher) + `RejectionFree`/`ExtremalOpt` exploit phase | monotone final-gap improvement, ≥1% median gap reduction |
| **v0.5** | L5 sensors + controllers (ladder auto-tune, bandit, OGP switch) | ablation: adaptive ≥ hand-tuned on held-out instances |
| **v0.6** | IR + lowering passes formalized; L7 portfolio + featureizer; `ContinuousRelax`/`BPWarmstart` init operators | full 499-instance campaign vs v0 baseline + OpenJij + published DA/SBM G-Set numbers |
| **v1.0** | "Optimization Architecture v1" — documented operator ISA, plugin API, claim-anchored result base | REPORT.md-grade publication-quality evidence; external reproduction from a single command |

### 8.7 What success means

One number to watch first: **G60-class instances at equal wall-clock vs vanilla
SA** — the regime we currently lose. Then: hold every dense win bit-for-bit
(v0.2 gate), close toward published Fujitsu DA / Toshiba SBM G-Set results, and
demonstrate the architectural claim itself — that a *schedule change* (no new
code) retargets the engine across problem families. That last demonstration is
what makes this an architecture and not the 38th heuristic in the zoo.
