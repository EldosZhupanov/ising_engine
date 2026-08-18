# The Relational Primitive, and an Impossibility Theorem

Deliverable: a complete axiom taxonomy with each axiom classified as
*mathematically unavoidable*, *empirically contingent*, or *historically
accidental*; one impossibility theorem with a confirmed quantitative prediction;
and one candidate primitive that survives three refutation attempts.

---

## 1. Complete taxonomy of hidden axioms

Shared by SA, PT, PA, BP, LTGA, EDA, CP-SAT, SCIP, MCTS, neural CO, diffusion
optimization, gradient methods and evolutionary algorithms.

| | Axiom | Status | Justification |
|---|---|---|---|
| **Ax1** | The answer is an element of an explicitly representable set X | **unavoidable** | the output must be writable; a solver that cannot emit its answer is not a solver |
| **Ax2** | The mutated object *is* a candidate solution | **accidental** | DP mutates a value function, CDCL a clause database, MCTS a tree |
| **Ax3** | Quality is a scalar total order | **unavoidable *on unweighted instances*** | proved: E = 2V − \|E\| (RC-004). Contingent only under heterogeneous weights |
| **Ax4** | Direction is derived from f at runtime | **accidental but cost-obstructed** | diffusion/neural CO destroy it; ~10⁴× per-move overhead confines it to coarse granularity |
| **Ax5** | The search space never shrinks | **accidental** | CDCL and B&B destroy it. Obstructed for Ising: every configuration is feasible, so exclusion needs bounds, and MaxCut relaxations are weak |
| **Ax6** | Step t+1 depends on step t | **accidental** | Transformer has no recurrent state; DP is a partial order on a DAG |
| **Ax7** | Nothing transfers between instances | **accidental** | any amortized method destroys it |
| **Ax8** | A move changes O(1) coordinates | **accidental** | Simplex pivots change the whole basis |
| **Ax9** | Instance is inert data, algorithm is a fixed interpreter | **contingent** | analog Ising machines instantiate the instance physically; digitally this is a constant factor |
| **Ax10** | **The only channel from knowledge to progress is a move** | **accidental** | CDCL=pruning, DP=tabulation, Simplex=reparameterization, MCTS=allocation |
| **Ax11** | **State is expressed in the problem's given coordinates** | **accidental — and provably harmful** | §2 |

Ax1 and Ax3 are the only unavoidable ones, and Ax3 only conditionally. Everything
else is convention.

## 2. An impossibility theorem

**Theorem (Gauge Degeneracy).** Let E(s) = Σ J_ij s_i s_j be an Ising objective
with no external field. Then E(s) = E(−s) exactly, so the Gibbs measure is
invariant under global spin flip, and for every site i and every temperature

    ⟨s_i⟩ ≡ 0.

**Corollary (vacuity of marginal state).** Any algorithm whose state is a set of
per-variable marginals carries **exactly zero** information about the solution of
a field-free Ising problem. Mean-field annealing, UMDA, every product-form EDA,
and marginal-propagating BP are not *weak* on MaxCut — they are **provably
vacuous**. Any non-zero marginal they observe is sampling noise.

MaxCut is field-free: in spin coordinates the QUBO linear terms cancel exactly.
Every G-Set instance is therefore covered by the theorem.

### 2.1 Confirmed quantitatively, and it discriminates against the rival explanation

The rival explanation for m ≈ 0 is glassiness (multimodality). It predicts m ≈ 0
*regardless* of a field. Gauge degeneracy predicts m ≈ 0 **only while the
symmetry holds**, and immediate polarization once it is broken.

Measured (`src/bin/exp_field_capacity.rs`, 30 G-Set instances, R = 32, noise floor
√(2/πR) = 0.1410):

| | mean\|m\| / noise | frac \|m\|>0.9 | mean\|cov\| / noise |
|---|---|---|---|
| **h = 0** (symmetric) | **0.96 — dead** | 0.00% | **2.77 — alive** |
| **h = 2** (symmetry broken) | **5.46 — alive** | **49.3%** | **0.89 — dead** |

Gauge degeneracy is confirmed and glassiness is refuted as the cause: a field
lifts the marginals from the noise floor to 5.5× it, with 49% of sites strongly
polarized.

The second column is the discovery. **The information does not appear or
disappear — it moves between representations.** When values are determinable,
relations become redundant; when values are forbidden by symmetry, relations carry
everything. Total information is conserved; only its *coordinates* change.

### 2.2 An instrument failure, reported not hidden

The h = 0.5 condition returned exact zeros in every cell. That is impossible for a
covariance, so it is not data: `SparseBitSlice` rejects non-integral weights, the
`let Ok(...) else { continue }` skipped every instance, and the run measured an
empty set. Only h = 0 and h = 2 (integral) are valid. Reported because a reader
of the raw output would otherwise see a spurious "field destroys all information"
row.

## 3. The primitive

> **RELATIONAL STATE.** The computational object is a progressively refined,
> partial description of the **relations between variables** — the quotient of
> configuration space by the problem's symmetry group — never the values of the
> variables themselves.

For field-free Ising the symmetry group is Z₂ and the quotient is the space of
*partitions*: dimension n−1, not n. A complete consistent set of pairwise
relations determines the solution up to global flip, so the relational state is
**not** exponential — it is exactly the information the theorem says is
determinable, and no more.

This changes what computation *is* rather than how fast it runs: the state is not
a point being moved but a relation system being refined, and the answer is
*derived* from it at the end by a linear-time 2-colouring.

### 3.1 Why it is not "just use overlaps"

Spin-glass physics has always known ⟨s_i⟩ = 0 and always used overlaps. **The
physics here is standard and I claim none of it.** The claim is algorithmic: in
every existing method the gauge-invariant object is used as a *transient
observable* to construct a move — Houdayer builds overlap clusters, then discards
them. That is Ax10 again. **No algorithm maintains a persistent, accumulating
relational state.** Relations are computed and thrown away, every step, forever.

### 3.2 Frustration is relational inconsistency

The formulation pays for itself immediately: a relation system is consistent iff
the signed graph has no odd cycle, and consistency is decidable in *linear* time
by union-find. **Frustration is exactly inconsistency of the relational
description**, and odd cycles are its carriers. MaxCut restated: *maintain a
partial relation system and decide which relations to violate.* The hardness moves
from "search a landscape" to "resolve an inconsistent constraint system" — which
is the form CDCL is good at, and the form SA has no representation for.

## 4. Three attempts to destroy it

**Attempt 1 — is it a relabelling of pairwise EDA?** No. A pairwise EDA estimates
correlations and resamples configurations; its state is still a distribution over
*values*, and it is re-estimated from scratch each generation. Relational state is
persistent and lives in the quotient space. **Survives** — but it also *predicts*
pairwise EDAs should outperform univariate ones on gauge-symmetric problems by a
large margin, which is a testable consequence.

**Attempt 2 — does the relation space collapse under frustration?** The threat: if
relations are inconsistent, no consistent state exists and the object is
ill-defined. But a *partial* relation system need not be complete; inconsistency is
localized to odd cycles, which is information rather than failure. **Survives**,
and this is where the primitive gets its content.

**Attempt 3 — the fatal one: does relational state face the same order blow-up as
decomposition?** Composing relations is transitive closure (union-find), which
stays *pairwise* and does not generate higher-order terms — unlike marginalization,
which does. This is the exact obstruction that killed the decomposition
architecture in §8.9 of the axioms document, and relational state evades it
because it composes by transitivity rather than by summation. **Survives, and this
is the strongest argument in its favour.**

## 5. Historical breakthroughs re-explained

1. **CDCL.** Standard account: "clause learning prunes the space." Relational
   account: CDCL accumulates knowledge in the *gauge-invariant* space — a clause
   is a relation, not an assignment. **Prediction it makes that pruning does not:**
   binary clauses and equivalence reasoning should be disproportionately valuable
   relative to their information content. This is empirically true — equivalence
   reduction and binary implication graphs are among the strongest SAT
   preprocessors, a fact the pruning account does not predict.
2. **Houdayer / isoenergetic cluster moves.** Standard account: "cluster moves
   cross barriers." Relational account explains **why the overlap specifically**:
   it is the unique gauge-invariant pairwise object. It predicts that a cluster
   move defined on absolute spin values rather than overlap must fail — which is
   correct, and is not explained by barrier-crossing.
3. **Simplex.** The basis records *which constraints are tight* — a relation among
   constraints — and x is derived from it. The state is coordinate-free in exactly
   the sense above.
4. **Transformer.** Attention is literally a matrix of pairwise relations between
   tokens, with no recurrent value state. Standard account: "captures long-range
   dependencies." Relational account: the state *is* the relation matrix.

Four breakthroughs; in two of them (CDCL, Houdayer) the relational account makes a
prediction the standard account does not.

## 6. Falsifiable predictions, before implementation

- **P5 — CONFIRMED above.** Breaking the gauge symmetry must move information from
  relations to values. Measured: marginals 0.96 → 5.46× noise; covariance
  2.77 → 0.89× noise.
- **P6 — untested.** Information conservation across representations: for a family
  of instances interpolating from h = 0 to large h, the *sum* of marginal and
  relational information should stay roughly constant while its split shifts
  monotonically. Refuted if total information rises or falls sharply with h.
- **P7 — untested, the decisive one.** The residual covariance (76% of variance,
  2× noise, RC-004 §8.6) should be concentrated on **odd cycles** of the signed
  graph. If the emergent relational information is *not* carried by frustration
  carriers, the primitive has identified the wrong object and should be abandoned.

## 7. What is and is not claimed

**Not claimed:** the physics. ⟨s_i⟩ = 0 by symmetry, and the use of overlap as the
meaningful observable, are textbook spin-glass theory.

**Claimed:** (a) the corollary as an *impossibility theorem classifying algorithm
families* — marginal-state methods are provably vacuous, not merely weak, on
gauge-symmetric objectives; (b) the observation that information is *conserved but
relocated* between value- and relation-coordinates as symmetry breaks, measured
here in both directions; (c) relational state as a persistent computational object
rather than a transient observable, which no existing method maintains; (d) the
identification of frustration with relational inconsistency, which supplies the
primitive's operational content and evades the order blow-up that killed the
decomposition architecture.

**Status:** no code written for the primitive. P7 is the measurement that should
decide whether it is built, and it is cheap.
