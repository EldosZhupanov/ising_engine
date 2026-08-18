# Research Cycle 003 — Architectural Assumption Audit

**Change of level.** RC-001 and RC-002 were operator research with rigorous
statistics wrapped around them. Both ended in a p-value on a move. This cycle
does not propose a move. It audits the assumptions shared by every solver in
`benchmark_suite/external/` and this repository, then attempts to **kill each
architectural hypothesis mathematically before any code is written**.

Three of four hypotheses die below. That is the point: a proof costs minutes, an
experiment costs hours, and a wrong architecture costs a cycle.

---

## 1. Assumption audit

Ten assumptions, checked against source. ✓ = the system makes the assumption,
**✗** = it breaks it.

| # | Assumption | Neal | OpenJij | CP-SAT | SCIP | Ours |
|---|---|---|---|---|---|---|
| A1 | State is a configuration (or set of them), not a distribution | ✓ | ✓ | ✓ | ✓ | ✓ |
| A2 | The objective is fixed for the whole run | ✓ | ✓ | ✓ | ✓ | ✓ |
| A3 | Moves are local **in the given variable basis** | ✓ | ✓ | ✓ | ✓ | ✓ |
| A4 | The interaction graph is static | ✓ | ✓ | ✓ | ✓ | ✓ |
| A5 | Search is guided by energy differences | ✓ | ✓ | ✓ | ✓ | ✓ |
| A6 | Control is a scalar schedule over time | ✓ | ✓ | ✓ | ✓ | ✓ |
| A7 | Replicas/workers are independent solvers of the same problem | ✓ | ✓ | **✗** shared bounds | ✓ | ✓ |
| A8 | Time is discrete and synchronous (sweeps) | ✓ | **✗** `continuous_time_ising.hpp` | ✓ | ✓ | ✓ |
| A9 | Every variable is treated identically | ✓ | ✓ | **✗** branching/pseudocost | **✗** | partly (RC-001) |
| **A10** | **The move set is designed by a human and fixed at compile time** | **✓** | **✓** | **✓** | **✓** | **✓** |

Only three assumptions are broken by anyone, and each by exactly one system.
**A1–A6 and A10 are broken by nobody.**

`ExactInference`, `Approximate` and `Warmstart` have zero implementing operators
in our registry (RC-002 §1.1), and A10 explains why: the move set is a
compile-time constant, so an absent capability stays absent forever.

## 2. Four architectural hypotheses

Each must change the computation model, not add a heuristic.

- **H-A — Entropy as the control variable.** Delete temperature as an input.
  Specify a target entropy trajectory S\*(t) in bits; solve for whatever β
  realises it. Answers "why does temperature exist?" — β is a Lagrange
  multiplier dual to energy under an entropy constraint.
- **H-B — Dynamic dimension.** The variable count shrinks during the run. Run
  exact roof-duality persistency *interleaved with* search, not as presolve;
  proven fixings shrink the problem, which may unlock further fixings.
- **H-C — Runtime move synthesis.** The move set is not designed. The system
  mines its own trajectory for variables that move together and *synthesizes*
  collective moves at runtime. Breaks A10 and A3.
- **H-D — Distribution as state.** State is the marginal field, not
  configurations. Replicas become a sampling representation of it. Breaks A1.

## 3. Mathematical falsification, before implementation

### H-A dies: entropy control is provably a reparametrized cooling schedule

For a Gibbs ensemble, S = β⟨E⟩ + ln Z. Differentiating, using
d(ln Z)/dβ = −⟨E⟩ and d⟨E⟩/dβ = −Var(E):

    dS/dβ = ⟨E⟩ + β·(−Var(E)) − ⟨E⟩ = **−β · Var(E)**

For β ≥ 0 and any non-degenerate ensemble, Var(E) > 0, so dS/dβ < 0 **strictly**.
S(β) is therefore strictly monotone, hence a **bijection**. Specifying S(t) is
mathematically identical to specifying β(t).

An entropy schedule *is* a cooling schedule in different units — explicitly on
the reject list. The only residue is that the bijection is instance-dependent, so
entropy schedules might transfer across instances better; that is a claim about
schedule portability, not a computation model.

**REJECTED.** This is precisely the trap the critique identified: my instinct
produced a thermostat again, and one line of algebra shows it is not an
architecture.

### H-B dies: the loop is either empty or unsound

Roof-duality persistency: variables taking the same value in all optima of the
roof-dual relaxation are persistent — some global optimum agrees with them.
Fixing them and re-solving the roof dual on the residual returns the restriction
of the original relaxation solution: **the relaxation acquired no new
information, so no new persistencies appear.** The interleaving loop is a no-op.

To make it non-empty you must condition on variables that are *not* proved — the
obvious candidate being high-confidence ensemble consensus. But **RC-001 already
refuted exactly that**: freezing consensus won 0 of 132 non-tied pairs
(λ = −2, p ≈ 0), and got worse as the consensus signal got stronger.

**REJECTED** — and rejected by my own prior cycle, at zero cost. This is the
knowledge compounding that was missing before.

### H-D dies: known theory predicts failure on this benchmark

Belief propagation is exact on trees. On loopy frustrated graphs it fails to
converge in the replica-symmetry-broken phase (the de Almeida–Thouless
instability). G-Set instances are frustrated spin glasses squarely in that
regime. Survey propagation repairs this at 1RSB but is built for sparse random
CSPs near threshold, not dense weighted MaxCut.

**REJECTED** for this benchmark on established theory. Worth revisiting only on
sparse, tree-like instance families — which we do not currently benchmark.

### H-C survives — but the ergodic theorem sharpens it into something falsifiable

The naive form of H-C is **not** novel. Learning which variables move together
and mixing them as blocks is linkage learning: LTGA, GOMEA, and the EDA/BOA
family are mature and effective, and they learn linkage from **population
covariance**.

Now the falsification attempt. My proposed twist is to learn linkage from
**temporal** flip correlations along the trajectory instead. By the ergodic
theorem, for an equilibrated ergodic chain

    ⟨δsᵢ δsⱼ⟩_time  =  ⟨δsᵢ δsⱼ⟩_ensemble

so in equilibrium the two are **identical** and my twist is a rename of a known
method. H-C is therefore dead *in equilibrium*.

It survives only where the equality fails — **out of equilibrium**. And an
optimization run is never equilibrated: annealing is by construction a
non-equilibrium process.

**Sharpened hypothesis H-C′:** transient, non-equilibrium flip correlations carry
linkage structure that equilibrium population covariance does not. The move set
synthesized from *dynamics* is therefore different from, and on quenches better
than, the move set synthesized from a *population snapshot*.

This makes a prediction that can refute it cleanly:

> The advantage of dynamics-mined moves over population-mined moves must **shrink
> toward zero as the run approaches equilibrium** (long runs, flat ladder, high
> T) and be **maximal in fast quenches**. If the two move sets perform
> identically at all mixing rates, the ergodic argument holds in practice and
> H-C′ is refuted.

## 4. Why H-C′ is architectural rather than another operator

It breaks A10 — the only assumption no system in the audit breaks. The move set
stops being a compile-time constant and becomes a runtime object the system
builds from its own history. Concretely this means:

- The registry's empty capabilities can be *filled by the running system* rather
  than by a human writing a new file.
- Two runs on different instances end with **different move sets**, and the move
  set is part of the result.
- Operators arise automatically, which is the stated criterion.

The enabler was found in the audit and I did not previously know it:
`runtime.rs:287` uses `ops.get_mut(&step.operator)` — **operators are
instantiated once per run and persist across every step**, so an operator can
accumulate trajectory statistics and mutate its own move set as the run
progresses. No core change is required to host a self-modifying move set.

## 5. Pre-registered design for H-C′

**Architecture.** A synthesizer that maintains, per run:
- a co-flip statistic over the trajectory, updated online;
- a set of synthesized collective moves (variable subsets) mined from it;
- a proposal step that flips an entire synthesized subset as one move, accepted
  by the standard rule.

**Three arms, identical budget:**

| Arm | Move set source | Purpose |
|---|---|---|
| **P** population-mined | covariance across replicas at one instant | the known method (LTGA-like) |
| **D** dynamics-mined | temporal co-flip along the trajectory | H-C′ |
| **N** null | synthesis disabled; single-flip only | must be bit-identical to the baseline operator |

**Swept factor — mixing rate**, which is the whole falsification: fast quench
(few sweeps, cold) → slow anneal (many sweeps, hot). H-C′ predicts D > P at fast
quench and D ≈ P as equilibrium is approached.

**Falsification criteria (fixed now):**
- **H-C′ refuted** if D ≈ P across all mixing rates (bootstrap 95% CI on the
  paired difference inside ±0.1% at every setting).
- **H-C′ refuted** if the D−P gap fails to shrink monotonically with mixing time.
- **Architecture refuted outright** if both D and P fail to beat N — meaning
  synthesized collective moves carry no value at all on this substrate.
- Reporting an energy win below 1% is **not** a success unless the mixing-rate
  prediction is confirmed, since only that pattern distinguishes the architecture
  from a lucky neighborhood.

**Null control:** with synthesis disabled the operator must be bit-identical to
`metropolis_sweep`, unit-tested, exactly as in RC-001.

---

## 6. Results — H-C′ refuted, architecture validated, new law extracted

30 G-Set instances × 8 seeds = **240 paired observations per contrast**, 100
sweeps, 32 replicas. Harness `src/bin/exp_synthesis.rs`, operator
`src/engine_v2/operators/move_synthesis.rs` (4/4 unit tests, incl. null
bit-identity to `metropolis_sweep` and cross-backend equality).

| temp_hi | **P vs N** (known method) | **D vs N** (H-C′) | **D vs P** (the test) |
|---|---|---|---|
| 0.1 | **+0.0568%** p=1.7e-11 | −0.0057% p=0.85 | **−0.0623%** [−0.081,−0.044] p=2.3e-11 |
| 0.5 | **+0.0571%** p=2.3e-3 | +0.0154% p=0.10 | −0.0413% [−0.068,−0.016] p=7.4e-2 |
| 1.0 | **+0.0670%** p=1.8e-3 | +0.0027% p=0.99 | **−0.0639%** [−0.092,−0.036] p=2.2e-4 |
| 2.0 | **+0.0642%** p=1.1e-3 | +0.0168% p=0.28 | **−0.0471%** [−0.074,−0.020] p=2.1e-3 |
| 4.0 | **+0.0580%** p=2.6e-3 | +0.0111% p=0.24 | **−0.0466%** [−0.075,−0.022] p=9.0e-3 |

### 6.1 H-C′ is refuted on both pre-registered criteria

The prediction was D > P at low temperature with the gap **shrinking to zero** as
mixing improves. Observed:

- **Wrong sign.** D is *worse* than P at every temperature.
- **No mixing dependence.** The gap is flat: −0.062, −0.041, −0.064, −0.047,
  −0.047. It does not shrink; it barely moves.

D also fails to beat the plain single-flip baseline anywhere (p = 0.85, 0.10,
0.99, 0.28, 0.24). Temporal co-flip mining is worth nothing.

### 6.2 The architecture itself is validated — but the gain is immaterial

Breaking A10 works. Runtime-synthesized moves beat the fixed single-flip move set
**at every temperature**, p ≤ 2.6e-3 throughout, with a strikingly *constant*
effect of ≈ +0.06%. A move set that did not exist at compile time, mined by the
system from its own run, reliably helps.

It helps by **0.06%**. Under the standing rule that improvements below 1% are
rejected unless they expose a principle, this is **not claimed as a win**. It is
claimed as a working architecture with a negligible payoff at this scale.

### 6.3 The law: mobility is not linkage

The ergodic argument predicted D and P coincide *at equilibrium* and diverge
*away* from it. They diverge **everywhere, by a constant amount**. So the
difference has nothing to do with equilibrium — it is structural, and the ergodic
framing was the wrong lens.

Mechanism: two variables flip in the same sweep mostly when both are **weakly
constrained**, i.e. when local acceptance is high. Temporal co-flip therefore
measures **mobility**, not coupling. And high-mobility variables are exactly the
ones single-flip dynamics already handles well, so bundling them into a
collective move adds nothing — which is precisely the observed null for D vs N.

Cross-replica covariance measures how variable *values* co-vary across
independent samples. That is genuine **linkage**, and it identifies pairs whose
joint state matters, including strongly-constrained pairs that single flips
cannot move.

> **Law RC-003.** Co-movement in *time* is a mobility statistic. Co-variation
> across an *ensemble* is a linkage statistic. They are not interchangeable, and
> only the latter identifies useful collective moves. Any method mining structure
> from trajectory dynamics is measuring which variables are loose, not which are
> bound together.

This retroactively explains a choice that always looked arbitrary: the entire
EDA / LTGA / GOMEA family learns linkage from **populations**, never from
trajectories. It is not a convention. Trajectory mining measures the wrong thing.

## 7. What this cycle changes about the next one

Three cycles now point the same way, independently:

- **RC-002:** what an initialization contributes is ensemble *entropy*, never
  ensemble *energy*.
- **RC-003:** what identifies a useful move is ensemble *covariance*, never
  trajectory statistics.
- **RC-001:** per-site ensemble *consensus* carries real (if small) signal.

In all three the information that matters lives in **cross-replica statistics**,
and in none of them does it live in configurations or trajectories. The replicas
are not N attempts at the answer; they are a sampling representation of a
distribution, and every positive result so far has come from reading that
distribution rather than from moving configurations.

That reopens **H-D (distribution as state)** — which §3 rejected. The rejection
was specifically of *belief propagation*: BP fails in the RSB phase via the
de Almeida–Thouless instability because its fixed-point iteration does not
converge. But a field **estimated from replicas** has no fixed-point iteration
and therefore no AT instability to suffer. The theoretical objection kills
message passing, not distributional state.

**Next architectural hypothesis (RC-004):** make the marginal field the primary
state and configurations a resampling device — a distributional architecture
whose field is Monte-Carlo estimated rather than message-passed. It is the one
survivor consistent with all three laws, and §3's rejection does not reach it.

---

*Three of four hypotheses died in §3 for the cost of a page of algebra and one
recalled prior result. The fourth died in experiment, and paid for itself with a
law that explains a 30-year convention in the linkage-learning literature.*
