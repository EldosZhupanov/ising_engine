# THE ISING ENGINE CONSTITUTION

**Status: RATIFIED. This is the Single Source of Truth.**

This document is not documentation, not a roadmap, not a TODO list, and not a
technical note. It is the Constitution. Every architecture decision, every
algorithm, every refactoring, every research program, every human contributor,
and every Claude agent must align with this document **before acting**. Research
conducted after ratification may clarify details; it may not change direction.
**The architectural vector is presumed fixed. It changes only if there is
reproducible experimental evidence or a demonstrable mathematical impossibility
that invalidates its assumptions — and only through the Amendment Procedure
(§15).** *(Wording per Amendment I, ADR-0006.)*

Precedence: this Constitution > `CLAUDE.md` > blueprints
(`research/OPTIMIZATION_ENGINE_BLUEPRINT.md`, `research/PLATFORM_BLUEPRINT.md`)
> everything else. Where a lower document conflicts, the lower document is wrong
and must be amended.

---

# 1. Mission

**Why the Ising Engine exists.**

Optimization is not a niche of computer science. It is the computational form of
almost every hard decision humanity makes: allocating scarce resources, routing
energy and goods, folding molecules, scheduling hospitals, designing materials,
balancing power grids, training models, verifying hardware. Most of these
problems are NP-hard, and the world's answer to NP-hardness has bifurcated:

- **Exotic hardware** — quantum annealers, coherent Ising machines, digital
  annealing ASICs. Powerful, proprietary, expensive, fixed-function, accessible
  to a few institutions.
- **Fragmented software** — thousands of one-off heuristics, each a bespoke
  implementation, unreproducible, unmeasured against each other, abandoned when
  their paper is published.

Both answers fail the same way: **capability is locked away** — in silicon
nobody can buy, or in code nobody can trust.

The Ising Engine exists to prove and permanently maintain a third answer:

> **The commodity CPU — the most widely deployed computational substrate on
> Earth — driven by physics-inspired dynamics through a rigorous, open,
> verifiable execution architecture, is a competitive optimization machine.**

The problem we solve for humanity is **access**: to make near-frontier
optimization capability available to anyone with an ordinary processor, with
results that are bit-reproducible, statistically honest, and anchored to
verifiable ground truth. Not "a solver that is fast" — an *instrument* whose
readings can be trusted.

**Why it must still exist in 20 years.** CPUs will change, accelerators will
come and go, and every individual heuristic will be superseded. But three things
in this project do not expire:

1. The **architecture** — the separation of substrate, operators, and control
   outlives any hardware generation, the way LLVM outlived every ISA it started
   with.
2. The **evidence base** — claim-anchored, reproducible measurements of what
   actually works on which problem structures accumulate value forever; they
   are the project's scientific capital.
3. The **discipline** — correctness-first, measurement-gated engineering is
   permanently rare and permanently valuable.

**Why this is more important than building another solver.** Another solver adds
one point to the heuristic zoo and depreciates immediately. An execution
architecture converts the entire zoo — past and future — into schedules over
shared, verified, optimized primitives. It compounds. A solver is a result; the
engine is a *means of producing results indefinitely*.

---

# 2. Vision

**What the engine is in 10 years.**

> **LLVM for physical optimization.**

Not a feature list — a position in the computational stack:

- **Frontends** accept problem classes (QUBO, Ising, HUBO, Max-Cut, constraint
  systems) the way compilers accept languages.
- A **typed intermediate representation** carries the problem through certified
  lowering passes (persistency, decomposition, quantization, reordering,
  coloring) the way IR carries programs through optimization passes.
- **Backends** target execution substrates — today `DenseByte` and
  `SparseBitSlice` on CPUs; tomorrow GPUs, FPGAs, ASICs — behind one interface,
  the way compiler backends target ISAs.
- **Operators** — thermal sweeps, cluster moves, population resampling, history
  fields, exact sub-inference — are the instruction set of optimization
  dynamics.
- The **scheduler and adaptive runtime** compose operators into computations,
  the way a runtime schedules kernels.
- The **knowledge system** closes the loop: every run makes the next run
  smarter.

In 10 years, "we implemented our method as an Ising Engine schedule" should be a
normal sentence in a paper, and "we validated our results against the Engine's
claim-anchored benchmark base" should be a normal sentence in a review. The
engine is infrastructure, in the exact sense that LLVM, BLAS, and CUDA are
infrastructure: invisible when it works, irreplaceable when it's gone.

---

# 3. Main Architectural Idea

**Fixed once and for all:**

> **We are NOT building another solver.
> We are building a Universal Physics-Inspired Optimization Architecture.**

Its three axioms:

1. **Algorithms become operators.** Every optimization dynamic — Metropolis,
   heat-bath, parallel tempering, cluster moves, population annealing, exact
   sub-solves, history biasing — is expressed as an operator: a typed
   transformation over shared state, with a declared contract, cost model, and
   sensor report. There are no free-standing algorithms.

2. **The solver becomes the execution system.** What used to be "a solver" is
   now: substrate (state layouts + kernels) + operator set + scheduler +
   adaptive control + verification. "UltimateSolver" is, going forward, one
   backend and one default schedule inside this system — not the system.

3. **Behavior is determined by the schedule of operators.** Retargeting the
   engine to a new problem family means changing the *plan* — which operators,
   in what phases, under what budgets — not writing new solver code. If solving
   a new family well requires bespoke solver code rather than a schedule plus at
   most a new operator, that is an architecture bug, and the architecture gets
   fixed.

Every simulated annealer, tempering scheme, and hybrid in the literature is,
from this architecture's viewpoint, *a schedule*. That is the whole idea, and it
does not change.

---

# 4. Immovable Principles

These are constraints, not guidelines. Violations are reverted, not debated.

1. **DO NOT build a monolithic solver.** No new all-in-one solver structs. New
   capability enters as an operator, a pass, a backend, or a schedule.
2. **DO NOT write huge switches.** Dispatch happens through the operator trait,
   the backend trait, and the plan — never through growing `match` towers over
   algorithm enums.
3. **DO NOT mix algorithms.** Operators do not call, know about, or special-case
   other operators. Composition happens only in the scheduler, only through
   state and reports.
4. **DO NOT tie the architecture to a single problem type.** The IR is the only
   thing frontends and backends share. Nothing below the frontend may assume
   "this is Max-Cut."
5. **DO NOT break determinism.** Given (problem, plan, seeds, thread layout),
   trajectories are bit-identical. FP contraction (`mul_add`), reordered float
   reductions, reduced-precision RNG compares, and iteration-order changes in
   hot loops are trajectory changes — rejected by default. The golden
   regression (`tests/test_regression_golden.rs`) must pass byte-identical.
6. **DO NOT use heuristics without measurements.** No operator, parameter, or
   "improvement" ships on intuition. Unmeasured = nonexistent.
7. **DO NOT make decisions without benchmarks.** Performance claims require
   identical-seed A/B against the preserved previous binary
   (`ab_engine_compare.py`), energies bit-identical, >1% measured geomean win.
   Quality claims require the calibrated equal-wall-clock protocol with fixed
   seed lists, bootstrap CIs, and paired nonparametric tests.
8. **DO NOT add code without purpose.** Every merged line traces to a mission
   need, a measured win, or a verification requirement. Speculative
   generality is deleted.
9. **DO NOT create complexity without gain.** Complexity is a cost paid in
   perpetuity; it is only purchased with measured capability. When a simpler
   construction ties a complex one, the simpler one wins by law.
10. **DO NOT trade correctness for anything.** Not for speed, not for elegance,
    not for a deadline, not for a benchmark win. There is no exchange rate.
11. **DO NOT bypass verification.** Behavior-changing operators must be typed
    as behavior-changing in the plan; they never silently enter a bit-identical
    comparison; final energies are always re-scored by the canonical scorer
    against the bare model.
12. **DO NOT trust a parser without external ground truth.** Every input format
    is validated against published optima or official solution files before its
    results are believed (the OR-Library/Biq Mac/QPLIB convention traps are the
    permanent reminder).

---

# 5. Scientific Philosophy

**The criterion is engine strength. Nothing else.**

- We do **not** chase publications. Publishability is never an argument for or
  against a change.
- We do **not** invent ideas for the sake of novelty. Novelty has zero intrinsic
  value here.
- We **integrate the best ideas from global science, regardless of age or
  origin.** Metropolis (1953), Swendsen–Wang (1987), Houdayer (2001),
  Boettcher–Percus (2001), metadynamics (2002), population annealing (2003),
  Selby's exact subgraphs (2014), non-reversible tempering (2021), momentum
  annealing, digital-annealer techniques — if it is strong, it is ours to study,
  extract, improve, and integrate.
- **If a published idea makes the engine better, we are obliged to use it.**
  Prior existence is evidence of value, never grounds for rejection.
- **If a new idea is weaker than an existing one, it is unnecessary** — killed
  or archived with its measurements, so the knowledge of *why* it failed is
  retained.
- Negative results are results. A validation program that kills 18 of 20 ideas
  and proves 2 is a success, and its kill-reports are permanent assets.
- Every claim binds to a machine-checkable anchor: brute force on small
  instances, published optima, official solution files, bit-identical replay,
  content-addressed artifacts, or statistical certificates. A claim without an
  anchor is an opinion, and opinions do not merge.
- Honesty about losses is mandatory. Where the engine loses (as it historically
  lost large sparse instances to vanilla SA), the loss is documented, explained,
  and turned into the next priority — never hidden, reframed, or benchmarked
  around.

---

# 6. Engineering Philosophy

- **CPU-first.** The commodity processor is the primary target. Designs start
  from what the real machine measures (ISA flags, cache sizes, line width), not
  from an abstract machine.
- **SIMD-first.** Hot paths are designed as vector kernels from the start —
  branch-free in the lane dimension — not scalar code vectorized later.
- **Cache-first.** Data layouts are chosen so working sets live in a named cache
  level by design (L1 tables, L2 work-unit slices, L3-resident state). Memory
  layout is an architectural decision, never an accident of struct definition.
- **Deterministic-first.** Reproducibility is a feature designed in (integer
  substrates, seeded streams, fixed orders), not a tax paid later. Bit-identical
  replay is the cheapest and strongest correctness instrument we own.
- **Data-oriented.** State is planes, fields, ledgers — contiguous, aligned,
  pooled. No allocation in hot paths, no `HashMap` in kernels, no string traffic,
  buffers reused. Structs serve layout; layout does not serve structs.
- **Energy-preserving.** Every IR transformation carries a certificate that the
  objective is preserved (or an explicit, typed ε with justification). Local
  fields are an invariant, auditable against full recomputation at any time.
- **Composable.** Primitives compose into operators; operators compose into
  schedules; layering is strict (a layer calls only layers below it).
- **Typed.** Contracts live in the type system: state traits, operator traits,
  behavior-changing markers, certified passes. If a rule can be a type, it must
  not be a comment.
- **Verifiable.** Everything important is checkable by a machine: `cargo fmt`,
  `clippy -D warnings`, the full test suite, the golden gate, parser self-tests,
  identical-seed A/B. The verification loop (`.claude/commands/verify.md`) is
  the gate for every change; a change that cannot be verified cannot be merged.

---

# 7. Engine Architecture

The fixed pipeline. Every stage exists, in this order, permanently:

```
Problem IR → Lowering → Backend Selection → Operators → Scheduler
          → Execution Engine → Verification → Benchmark → Knowledge Base
```

**Problem IR.** The single canonical representation: variables, offset, linear
coefficients, symmetric quadratic terms in CSR, plus structural metadata
(components, coloring, degeneracy order, integrality, density, frustration and
treewidth probes). Frontends normalize every problem class into it; nothing
downstream sees the original format. Coefficients are typed: exact integer,
certified-scaled integer, or float.

**Lowering.** A sequence of certified IR→IR passes: normalize (dedupe,
symmetrize), persistency (QPBO/roof duality variable fixing), decompose
(connected components), quantize (float→integer with exactness certificate or
explicit ε), reorder (BFS/degeneracy for cache locality, with stored
permutation), color (chromatic classes for synchronous updates). Every pass
emits a machine-checkable certificate that the objective is preserved under a
recorded bijection.

**Backend Selection.** A pass, not a person, chooses the execution substrate:
`DenseByte` (byte-per-replica SIMD, float couplings — the dense workhorse) or
`SparseBitSlice` (bit-per-replica planes, integer fields, chromatic updates —
the sparse machine), and derives the initial budget vector (replicas ×
temperatures × sweeps) from instance structure and cache geometry. Wrong
automatic choices are fixed by improving the selection pass, never by manual
override in production paths.

**Operators.** The instruction set of the engine (see §8). Each is a typed
transformation over `SpinState` with a declared cost model and a sensor report.

**Scheduler.** Executes the plan — a phase pipeline (presolve → explore →
exploit → finish) over an operator graph — and owns thread placement: work
units sized to the physical core count, state slices sized to local caches,
wall-clock budget accounting as a first-class contract.

**Execution Engine.** The substrate: bit/word kernels (XOR, popcount, blend,
transpose), vectorized PRNG streams, integer threshold tables, field
maintenance, memory pools, thread pinning. No physics, no policy — only fast,
deterministic mechanics.

**Verification.** Not a phase after development — a stage of the pipeline.
Golden byte-identical regression, full test suite, format/lint gates, parser
self-tests against published optima, identical-seed A/B with energy equality
assertion, canonical re-scoring of every final answer against the bare model.

**Benchmark.** The calibrated equal-wall-clock protocol: fixed seed lists,
per-instance budget calibration (<5% adherence), exactly-once accounting,
bootstrap CIs, Wilcoxon signed-rank, sign tests, effect sizes, and honest
win/loss reporting against external solvers and published results.

**Knowledge Base.** Run records, experiment registry, ADRs, per-family tuned
schedules keyed by instance features, and kill-reports of failed ideas —
content-addressed and claim-anchored, so the engine's accumulated experience is
queryable and survives every rewrite (see §12).

---

# 8. Operator Philosophy

**Each operator is a separate physical law.**

An operator embodies one physical process — thermal fluctuation, domain
nucleation, population quench, self-organized criticality, history repulsion,
exact local inference — as one typed transformation with one contract:

```
apply(state, context, budget) → report
```

- **An operator knows nothing about other operators.** It reads and writes
  shared state; it emits sensor data; it never calls, configures, inspects, or
  special-cases a peer. All coupling between operators is mediated by state and
  by the scheduler.
- **The scheduler creates a physics experiment.** A run of the engine is the
  composition of physical laws under a designed protocol — cool this ensemble,
  exchange these replicas, bias this landscape, solve this region exactly. The
  scheduler is the experimentalist; operators are nature.
- **No hard-coded algorithms.** "Simulated annealing," "parallel tempering,"
  "ICM" are not code paths — they are *plans*: named, versioned schedules over
  the operator set. A new method from the literature is reproduced as a
  schedule first; only a genuinely new *dynamic* justifies a new operator.
- Operators declare their nature in the type system: trajectory-preserving
  operators may enter bit-identical comparisons; behavior-changing operators
  (history biasing, continuous relaxation, ε-quantization) are marked, may
  serve only in typed roles (initialization, biasing, polishing), and their
  outputs are always re-scored canonically.
- Operators declare cost models so the scheduler and the bandit can allocate
  budget rationally. An operator without a cost model cannot be scheduled.

---

# 9. Backend Philosophy

**Everyone works through the same interface.**

The `SpinState` contract — read per-replica ΔE in O(1), apply masked flips with
field maintenance, report energies from ledgers, compute overlaps, produce
content-addressed checkpoints — is the boundary between physics and hardware.
Operators are written against the contract, never against a layout.

- **Dense Backend (`DenseByte`).** Byte-per-replica SIMD, float couplings.
  The proven dense-QUBO workhorse. It is preserved, not rewritten: its
  bit-identical behavior on every current win is a permanent regression anchor.
- **Sparse Backend (`SparseBitSlice`).** Bit-per-replica planes (one cache line
  = hundreds of replicas of one spin), integer couplings and fields, chromatic
  synchronous updates, cache-geometry-derived ensemble width. The answer to the
  large-sparse regime.
- **Future backends — GPUs, FPGAs, ASICs, and hardware not yet named — enter
  behind the same interface.** A new backend must implement the contract, pass
  the same verification gates, and replay the same golden trajectories where it
  claims trajectory equivalence — or declare itself behavior-changing and be
  benchmarked as such. No operator is ever rewritten "for" a backend; if the
  contract is insufficient for a new substrate, the *contract* is amended by
  ADR, once, for everyone.
- Backend selection is automatic (§10). Backends compete on measurements, and
  the selection pass encodes the winner per structural regime.

The test of this section: the day a GPU backend lands, the operator layer and
scheduler must not change. If they must, the boundary was drawn wrong and fixing
the boundary is the priority.

---

# 10. Decision Engine

**The brain of the engine. Before a single sweep runs, the instance is
understood.**

Every run begins with automatic structural analysis of the lowered IR:

| Signal | What it reveals | What it decides |
|---|---|---|
| **Density** (m/n², degree distribution) | dense vs sparse regime | backend choice, sweep kernel |
| **Weight distribution** (integrality, spectrum of |q_ij|, dynamic range) | quantizability, threshold-table shape | integer substrate eligibility, RNG strategy |
| **Treewidth probe** (greedy elimination bound) | exact-inference reach | ExactLNS region sizing; whole-instance exact solve when small |
| **Clustering / community structure** | decomposability | decomposition passes, work-unit boundaries |
| **Structure detection** (bipartite, planar, lattice, chimera-like, scale-free) | known-regime shortcuts | specialized passes and schedules |
| **Symmetry** (automorphism sample, gauge freedom) | search-space redundancy | symmetry-aware restarts, canonical forms |
| **Coupling spectrum** (frustration-relevant ratios) | landscape ruggedness prior | ladder range, cluster-move viability |
| **Local connectivity** (degree CV, k-core depth) | update-order and locality strategy | coloring, reorder pass, active-list viability |
| **Frustration** (odd-cycle sampling, plaquette parity) | glassiness | explore/exploit balance, ICM pairing, restart policy |

The output is the **Execution Plan**: backend, layout parameters, budget vector,
operator schedule, controller settings — selected from the knowledge base's
per-regime records (§12), with honest fallbacks for unrecognized structure.

Rules of the Decision Engine:

1. Analysis cost is bounded and budget-accounted: never spend more than a small,
   declared fraction of the wall-clock contract on deciding.
2. Every decision is recorded with its input features — so a wrong decision is a
   reproducible, fixable defect, not a mystery.
3. The Decision Engine learns only through the knowledge base (§12), never
   through hidden in-code constants accreted over time.
4. Manual overrides exist for research; production paths always go through the
   Decision Engine, so its blind spots surface and get fixed.

---

# 11. Adaptive Runtime

**During computation, the engine analyzes its own state. If a strategy stops
working, it changes — automatically, without restart.**

Sensors (cheap by construction — they read operator reports and
substrate-native probes like XOR-popcount overlaps):

- acceptance rates per temperature and per operator;
- replica round-trip rates through the ladder;
- energy variance / specific-heat estimates (transition location);
- overlap distribution P(q) and its gap structure (landscape geometry, OGP
  signature);
- revisit statistics (basin recurrence);
- frozen-core fraction (consensus across replicas and time);
- marginal energy gain per second, per operator.

Controllers (the only entities allowed to change the plan mid-run):

- **Ladder controller** — re-places temperatures for uniform exchange flow.
- **Budget controller** — reallocates the (replicas × temperatures × sweeps)
  vector when depth or width starves; the fixed-geometry failure that once cost
  the entire large-sparse regime is, by law, impossible to reintroduce.
- **Operator bandit** — shifts budget toward operators with measured marginal
  gain; starves those that stall.
- **Regime switch** — when equilibration provably stalls (acceptance collapse,
  bimodal P(q) with a forbidden gap), pivots from equilibration to
  restarts + recombination + exact finishing.
- **Dynamic decomposition** — conditions on frozen cores, re-decomposes the
  residual graph, recurses.

Constitutional constraints on adaptivity:

1. Adaptation is **deterministic**: controllers read sensors and seeded streams
   only, so an adaptive run replays bit-identically.
2. Every adaptation is **logged as an event** in the run record — the run
   remains a reproducible experiment, not an anecdote.
3. Controllers adjust *plans*, never operator internals.
4. Every controller ships with its ablation: adaptive must measurably beat
   static tuning on held-out instances, or it ships disabled.

---

# 12. Knowledge System

**After each run, the engine remembers — and gradually becomes smarter.**

Every run emits a structured record: instance features (from §10), the executed
plan, all adaptation events (from §11), outcomes (best energy, gap to best
known, time-to-target, operator-level marginal gains), environment fingerprint,
seeds, and artifact hashes. Records are append-only, content-addressed, and
claim-anchored — the same discipline as the benchmark base.

What the knowledge system stores:

- **what worked** — schedules and parameters that won, keyed by structural
  regime;
- **what didn't work** — failures and regressions, kept forever with the same
  fidelity as wins (a deleted failure will be re-attempted; a recorded one
  won't);
- **on what tasks** — the feature vectors that define each regime;
- **under what parameters** — full plans, not summaries, so any record is
  re-runnable.

How it feeds back:

1. The **Decision Engine** (§10) selects plans from regime records — this is
   the primary learning loop.
2. The **experiment registry and ADRs** capture *why* decisions were made, so
   direction survives personnel, agents, and context windows.
3. **Kill-reports** from the research pipeline (§13) are knowledge: each records
   the idea, the evidence, and the stage at which it died.
4. Aggregate views (per-family leaderboards, operator win-rates, regression
   history) are generated from records — never hand-maintained.

Constitutional constraints: knowledge influences *defaults*, never *results* —
a run's outcome depends only on (problem, plan, seeds), with the knowledge
system acting strictly upstream at plan-selection time. No opaque learned
component may silently alter execution; anything learned must be inspectable
and reproducible from the records that taught it.

---

# 13. Research Rules

**Every new idea follows the same path. No exceptions, no shortcuts, no
enthusiasm-driven merges.**

```
Idea
 ↓
Hypothesis            — falsifiable, with a named metric and target regime
 ↓
Mathematical
Justification         — the mechanism is coherent; complexity and convergence
                        properties stated; contradiction with known results = STOP
 ↓
Physical
Justification         — what does the physical process compute naturally, and
                        why should that computation transfer to this landscape?
 ↓
Architectural
Justification         — expressible as an operator/pass/schedule within §3–§9?
                        If it requires breaking the architecture, it is either
                        wrong or it is a constitutional amendment (§15) — decide
                        which before writing code
 ↓
Literature            — mandatory prior-art search. If it exists: study it,
                        extract strengths and failure modes, improve it,
                        integrate it (§5). Existence is evidence, not rejection.
 ↓
Experiment Design     — pre-registered: instances, seeds, budgets, baselines,
                        success thresholds, and the kill criterion — all fixed
                        BEFORE implementation
 ↓
Prototype             — minimal, isolated, off the production path; no
                        production code is touched by an unproven idea
 ↓
Measurement           — the pre-registered experiment, run under the benchmark
                        protocol (§7), calibrated budgets, fixed seeds
 ↓
Comparison            — against the strongest existing operator/schedule for
                        that regime, not against a strawman
 ↓
Self-Critique         — a genuine adversarial pass (Reviewer #2): what else
                        explains the result? what was overfit? what would a
                        hostile expert check first?
 ↓
Verdict               — exactly one of:
                        KILL       (record the kill-report; delete the code)
                        ARCHIVE    (sound but currently dominated; record why)
                        RESEARCH   (promising, needs theory/experiments; stays
                                    off the production path)
                        PROTOTYPE  (gated integration behind a flag, with the
                                    measured precondition that must hold)
                        IMPLEMENT  (full integration through the verification
                                    loop: fmt → clippy → tests → golden → A/B)
```

Standing rules of research:

1. **The pipeline maximizes truth, not surviving ideas.** A cycle that kills
   eighteen ideas and proves two is a successful cycle. Kill-reports are
   deliverables.
2. **Pre-registration is binding.** Success criteria set before the experiment
   cannot be adjusted after seeing results. A missed threshold is a KILL or
   ARCHIVE, not a renegotiation.
3. **No idea touches production before IMPLEMENT.** Prototypes live in
   isolation; behavior-changing prototypes are typed as such from the first
   line.
4. **Direction is presumed fixed; details are researched.** Research may
   determine *how* an operator works, *which* backend wins a regime, *what*
   the scheduler should prefer. Research may not conclude "we should build a
   different kind of project" — unless it produces reproducible experimental
   evidence or a demonstrated mathematical impossibility against a core
   assumption, in which case the finding goes to §15. (Amendment I, ADR-0006.)

---

# 14. What Can Be Changed — and What Cannot

**Immutable (change requires a constitutional amendment, §15):**

- The mission (§1) and vision (§2).
- The main architectural idea (§3): operators over a shared substrate,
  behavior as schedules, no monolithic solvers.
- The immovable principles (§4), including determinism and
  measurement-gated change.
- The scientific criterion (§5): engine strength, not novelty or
  publishability.
- The pipeline stages of §7 (each stage exists, in that order).
- The operator isolation law (§8) and the single backend interface (§9).
- The verification contract: golden byte-identical regression, identical-seed
  A/B with energy equality, canonical re-scoring, external ground truth for
  parsers.
- The decision priority: **Correctness > Reproducibility > Architecture >
  Performance > Readability > Style.**

**Freely changeable (through the normal verification loop):**

- Any operator's implementation — kernels, layouts, instruction selection —
  provided its contract and verification gates hold.
- The operator set: operators are added through §13 and retired when measurably
  dominated (with a recorded deprecation).
- Schedules, plans, controller policies, budget heuristics, Decision Engine
  features and thresholds — these are *designed to be* the mutable surface.
- Backends: new substrates may be added; existing ones optimized — behind the
  fixed interface.
- Lowering passes: added, reordered, improved — each with its certificate.
- Benchmark instances, external baselines, statistical tooling — strengthening
  the evidence base is always in order.
- Everything about developer experience: tooling, agents, hooks, indexes,
  documentation.

The dividing line, stated once: **the shape of the system is fixed; the
contents of every slot are in permanent, measured competition.**

---

# 15. Amendment Procedure

This Constitution changes rarely, deliberately, and traceably:

1. An amendment begins as a written proposal — an ADR in
   `research/architecture/ADR/` (system defined in ADR-0000) — stating: what changes,
   why the current text is wrong or insufficient, what evidence demands the
   change, and what breaks downstream.
2. Evidence must be of constitutional weight: a measured, reproduced result or
   a demonstrated architectural impossibility — never convenience, fashion, or
   a single anomalous benchmark.
3. The amendment is applied as a tracked edit to this file with the ADR linked;
   the old text remains recoverable in history. Silent drift — code that
   contradicts the Constitution without an amendment — is a defect of the code,
   categorically.
4. §1–§3 (mission, vision, main idea) are expected to survive the life of the
   project unamended. Treat any proposal against them as presumptively wrong
   until the evidence is overwhelming.

---

# 16. Oath of the Contributor

Human or agent, before acting in this repository:

1. Read this Constitution; align the plan with it before the first edit.
2. Search before reading; read before writing; measure before claiming.
3. Prefer the surgical change; leave every gate green; leave every claim
   anchored.
4. When the Constitution and convenience conflict, the Constitution wins.
5. When the Constitution and *evidence* conflict — bring the evidence to §15,
   and change the law, not the discipline.

**The criterion, always and only: does this make the engine fundamentally
stronger?**
