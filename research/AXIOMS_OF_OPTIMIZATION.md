# The Hidden Axioms of Optimization

The loop `State → Evaluation → Action Selection → New State` is not a law. It is a
set of eight unstated commitments. This document names them, tests the naming
against history, determines which are theorem-protected and which are merely
conventional, and identifies the one that is both unbroken in this field and
actually attackable.

---

## 1. The axioms

The standard loop presupposes:

| | Axiom | What it assumes |
|---|---|---|
| **Ax1** | **Representability** | The answer is an element of an explicitly representable set X. |
| **Ax2** | **Solution-carrying state** | The object the algorithm mutates *is* a candidate solution (or a set of them). |
| **Ax3** | **Scalar total order** | Quality is a scalar f(x) inducing a total order on X. |
| **Ax4** | **Runtime-derived direction** | The next move is computed *from f*, during the run. |
| **Ax5** | **Space invariance** | X is the same at step 10⁶ as at step 0. Nothing is ever permanently excluded. |
| **Ax6** | **Sequential dependence** | Step t+1 depends on step t. |
| **Ax7** | **Instance isolation** | Nothing learned on one instance persists to the next. |
| **Ax8** | **Locality** | A move changes O(1) coordinates. |

## 2. The framework's test: does it explain the breakthroughs?

A taxonomy that cannot explain known history is worthless. Each named
breakthrough should map to destroying a *specific, different* axiom.

| Breakthrough | Axiom destroyed | The new computational object |
|---|---|---|
| **Simplex** | **Ax8** (+Ax1) | a *basis*, not a point — vertex optimality turns a continuum combinatorial |
| **Dynamic programming** | **Ax2** | a *value function* over subproblems; Ax6 weakens to a partial order on a DAG |
| **SAT / CDCL** | **Ax5** | a *clause database* — monotone, sound exclusion; the assignment is scratch, the clauses are the product |
| **MCTS** | **Ax2, Ax3** | a *tree of statistics*; value is a posterior, not an evaluation |
| **Diffusion models** | **Ax4, Ax7** | a *learned score function* — direction comes from data, never from evaluating the objective |
| **Transformer** | **Ax6** | *no recurrent state at all* — all positions resolved in parallel |

The framework passes. Six breakthroughs, six distinct axioms, and in every case
the move was the same shape: **the thing being updated stopped being a candidate
solution.**

## 3. Where the field actually stands

| System | Axioms destroyed |
|---|---|
| CP-SAT | Ax5 (clause learning), partly Ax2 |
| SCIP | Ax5 (B&B + 18 presolvers) |
| Belief propagation | Ax2 (object is marginals) |
| LTGA / GOMEA | Ax8 (collective moves) |
| Neural combinatorial optimization | Ax4, Ax7 |
| Simulated annealing, PT, PA, SQA | **none** |
| **Our 18 operators** | **none** |

Every operator in this repository satisfies all eight axioms. That is a sharper
statement of RC-004's "one cell" finding, and it is the real reason three cycles
of careful work each returned ~0.06%.

**The SA lineage is amnesiac by construction.** Ax5 means the search space is
{±1}ⁿ forever: an SA run at step 10⁶ has excluded nothing that it had not
excluded at step 0. CDCL's space shrinks monotonically. That asymmetry, not
implementation quality, is why SAT solvers reached millions of variables and
annealers did not.

## 4. Which axioms are theorem-protected

Destroying an axiom is only worth attempting where no theorem forbids it. Two are
already settled by proofs from earlier cycles, and one is defended by a
mathematical obstruction:

- **Ax3 is PROTECTED on unweighted instances.** RC-004 proved E = 2V − |E| for
  J ∈ {±1}: energy and violation count are the same object under an affine
  bijection. On G-Set and `dimacs_maxcut` the scalar total order cannot be
  removed, because the "constraint multiset" *is* the scalar. Only `biqmac`
  (145 distinct weights) can even pose the question.
- **Temperature-elimination-by-reparametrization is PROTECTED.**
  dS/dβ = −β·Var(E) < 0 makes S(β) a bijection, so an entropy schedule is a
  cooling schedule renamed.
- **Ax5 is OBSTRUCTED for Ising, though not by a theorem.** Sound exclusion needs
  bounds. In MaxCut every configuration is feasible, so nothing is excluded by
  infeasibility — only by a bound, and MaxCut relaxations are weak enough that
  exact B&B loses to heuristics at n = 2000. CDCL's trick does not transfer,
  because there is no analogue of an unsatisfiable core.
- **Ax4/Ax7 are COST-OBSTRUCTED at fine granularity.** Metropolis decides a move
  in O(deg) ≈ 20 flops. Any learned proposer needs 10³–10⁶ flops. A learned
  direction must therefore be ~10⁴× better *per move* to break even, which the
  neural-CO literature does not deliver on speed-normalized comparisons. It
  survives only if applied at coarse granularity — per region or per restart,
  never per flip.

## 5. The attackable axiom is Ax2 — and my own data has been pointing at it for three cycles

- **RC-001:** per-site ensemble consensus carries real signal; configurations do not explain it.
- **RC-002:** what an initialization contributes is ensemble **entropy**, never ensemble **energy**. Quality is erased; diversity never is.
- **RC-003:** what identifies a useful move is cross-replica **covariance**; trajectory statistics measure mobility, not linkage.

Three independent cycles, three different questions, one answer: **the
information lives in the ensemble's statistics, and not in the configurations.**
And in all three I responded by writing another operator that mutates
configurations. The data was rejecting Ax2 and I kept re-assuming it.

### The new computational object

> **The algorithm's state is a distribution over configurations, held as
> sufficient statistics — per-site marginals and pairwise correlations — and
> updated directly. Configurations exist only as a sampling device for
> re-estimating those statistics. The answer is read out at the end by rounding
> the converged distribution.**

The loop is no longer

    State → Evaluate → Select action → New state

but

    Statistics → Update statistics → (resample only to re-estimate) → Statistics

with no candidate solution anywhere in it. Ax2 is gone; Ax6 weakens, because
statistics compose rather than sequence.

### Why this is not belief propagation

BP also destroys Ax2 and is the obvious objection. But BP updates its marginals
by a **fixed-point iteration**, and in the replica-symmetry-broken phase that
iteration does not converge — the de Almeida–Thouless instability. G-Set is
squarely in that phase, which is why BP is not competitive there.

A field estimated by **Monte Carlo from replicas has no fixed-point iteration**,
so there is no instability to suffer. The AT theorem kills message passing; it
does not reach a sampled field. This was RC-003's own conclusion and it is the
one route into Ax2 that the known obstruction does not close.

## 6. Falsification, before implementation

Stated now so it cannot be adjusted later:

1. **Sufficiency.** If per-site marginals plus pairwise correlations are truly
   the state, then reconstructing an ensemble by *sampling from those statistics*
   and discarding the configurations must lose nothing. Refuted if a
   statistics-only cycle degrades against a configuration-carrying control at
   matched work.
2. **The collapse risk, and it is the likely failure.** A product-form marginal
   field cannot represent a multi-modal distribution: a spin glass has
   exponentially many competing ground states, and the mean-field marginal of
   that mixture is m_i ≈ 0 everywhere — which encodes *nothing*. This is the same
   pathology that makes BP fail, arriving by a different door. If it occurs, the
   law is that **pairwise statistics are insufficient state for a glassy
   landscape**, and Ax2 is protected for a reason rather than by convention.
3. **The honest prior.** Failure mode 2 is more likely than success. That is
   worth running anyway: it would establish *why* Ax2 survives, which is a result
   about the whole SA lineage rather than about an operator.

## 7. What this document claims and does not claim

It claims: the eight axioms are real, they explain six historical breakthroughs,
our engine destroys none of them, two are provably protected in this setting, and
Ax2 is the one both unbroken in the Ising field and reachable by a route the
known obstruction does not close.

It does not claim a new class of computation has been built. It identifies which
one is worth building and states in advance what would refute it. Cycles 001–003
each produced 0.06% because they optimized inside all eight axioms; the value of
this analysis is that it stops the fourth from doing the same.

---

# 8. The theory's predictions, and the first one tested

A framework that makes no testable prediction is philosophy. Four predictions
follow from §1–§5; the second was run immediately because it decides whether the
proposed architecture should be built at all.

## 8.1 What is *fundamentally impossible* inside the eight axioms

Two capability gaps, not speed gaps:

**(i) Certification is impossible.** Ax5 says nothing is ever permanently
excluded, so no Ax5-respecting algorithm can ever prove optimality — only report
the best it saw. Forty years of simulated annealing has never certified an
optimum, and this is why. CDCL certifies UNSAT; branch-and-bound certifies by
bound-meets-incumbent. Both destroyed Ax5 to get there.

**(ii) Information accumulation is bounded — the amnesia bound.** Under Ax2 + Ax5
+ Ax7 the algorithm's only memory is its state. Total retained information about
the instance is therefore ≤ |Σ| = O(nR) bits, **independent of the number of
evaluations T**. An annealer may perform 10⁹ energy evaluations and still carry
O(nR) bits of what it learned. CDCL's clause database grows with T; DP's value
function grows with T. The SA lineage's does not.

**Prediction P1 — the Amnesia Law.** If retained information is independent of T,
then at fixed total work W, performance must be invariant to how W is partitioned
into independent restarts (beyond burn-in τ): best-of-k runs of length W/k ≈ one
run of length W, for all W/k ≫ τ. A genuinely learning architecture violates this
— one long run beats many short ones. **Testable with existing operators, no new
code.**

## 8.2 What becomes solvable if Ax2 falls

Not "MaxCut, faster". A different question becomes askable at all. A
single-configuration algorithm structurally cannot answer:

- *How many optimal solutions are there?* (counting, #P-type queries)
- *What is P(sᵢ = +1) over the optimal set?* (marginal inference over optima)
- *Which solution stays good under perturbation?* (robust optimization)
- *Give me k maximally-different near-optima.* (solution diversity)

These are capability gaps, and all four are real industrial requirements that the
entire annealing lineage cannot serve at any speed.

## 8.3 Prediction P2 — tested, and it refutes half the architecture

**Predicted before writing any architecture code:** on a glassy landscape a
product-form marginal field must collapse, because the mean-field marginal of a
multi-modal distribution is ≈ 0 everywhere. Quantitative null for a
noise field of R replicas: E|m| = √(2/πR) = **0.1410** at R = 32.

Measured (`src/bin/exp_field_capacity.rs`, 30 G-Set instances, randomized init
per RC-002, ~46k sites and ~100k edges per cell):

| temp_hi | mean\|m\| | ratio vs null | frac\|m\|>0.9 | mean\|cov\| | ratio vs null | frac\|cov\|>0.5 |
|---|---|---|---|---|---|---|
| 0.1 | 0.1347 | **0.95** | **0.00%** | 0.3843 | **2.72** | 26.7% |
| 0.5 | 0.1350 | 0.96 | 0.00% | 0.3910 | **2.77** | 28.3% |
| 1.0 | 0.1395 | 0.99 | 0.00% | 0.3830 | 2.72 | 27.7% |
| 2.0 | 0.1353 | 0.96 | 0.00% | 0.3595 | 2.55 | 25.2% |
| 4.0 | 0.1394 | 0.99 | 0.00% | 0.3235 | 2.29 | 19.7% |

**First moment: dead.** 0.95–0.99× the noise floor at every temperature. Of
~46,000 sites, **zero** are polarised above |m| = 0.9. The prediction was
confirmed to two significant figures.

**Second moment: alive.** Pairwise covariance runs at **2.3–2.8× the noise
floor**, with ~27% of edges above |cov| = 0.5.

This is the textbook Edwards–Anderson signature — ⟨sᵢ⟩ → 0 while
⟨sᵢsⱼ⟩ − ⟨sᵢ⟩⟨sⱼ⟩ stays finite — measured directly in our own ensemble.

**Consequences.** The product-form architecture is refuted by measurement, not by
argument: mean-field annealing, UMDA, and every marginals-only EDA would be
encoding literally nothing on these instances. It also explains BP's failure
concretely — BP propagates marginal information, and the marginals here are
noise. And it explains RC-003 retroactively: covariance-mined moves worked
because ~27% of edges carry real pairwise signal, while temporal co-flip mining
failed because it measured mobility instead.

**Cost of this refutation: one measurement, minutes.** The alternative was
implementing the architecture and discovering it over a cycle.

## 8.4 What the theory must reproduce as special cases

The distributional architecture is only credible if the known methods fall out of
it:

| Special case | Reduction |
|---|---|
| Simulated annealing | R = 1; the distribution degenerates to a point mass |
| Population annealing | statistics = the empirical distribution of R configurations |
| **UMDA / product-form EDA** | marginals only + full resampling |
| Mean-field annealing | marginals only, correlations dropped, deterministic update |
| Belief propagation | field updated by fixed-point message passing instead of MC |

**This is a genuine consistency check, and it passes in an unexpected way.** The
theory predicts the marginals-only special cases (UMDA, mean-field annealing) are
degenerate on glassy instances — and §8.3 measures exactly that. A theory whose
degenerate special case is independently known to be a weak method, and which
then predicts the *quantitative* degree of that degeneracy correctly, has earned
some credit.

The residue — pairwise correlations, Monte-Carlo estimated — is the only part
that is not one of the above.

## 8.5 Prediction P3 — the next test, and I expect it to hurt

The 2.7× covariance signal may be **trivially explained**. For a strongly coupled
edge, cov(i,j) ≈ −tanh(β J_ij): the correlation is a direct restatement of the
coupling, already known from the problem statement, carrying **zero** new
information. Only the *residual* after regressing out the direct-coupling
prediction is emergent structure.

**P3:** decompose measured covariance into the J-predicted component and the
residual. If the residual sits at the noise floor, then pairwise statistics carry
nothing beyond the instance itself, Ax2 is fully protected, and the distributional
architecture is dead in both moments.

There is already indirect evidence for this pessimistic outcome: RC-003 exploited
population covariance and got **+0.06%**. A 2.7× signal yielding a 0.06% gain is
the signature of a statistic that is real but redundant. P3 would explain that
gap, and a confirmed P3 is a stronger result than a working architecture — it
would show the SA lineage's amnesia is not a design oversight but a consequence
of there being nothing local left to remember.

## 8.6 P3 — tested, and **REFUTED**

Grouping every edge by sign(J_ij) and decomposing the covariance variance:

| temp_hi | η²(sign J) | residual sd | residual / noise floor |
|---|---|---|---|
| 0.1 | 0.2409 | 0.2889 | **2.05** |
| 0.5 | 0.2390 | 0.2909 | **2.06** |
| 1.0 | 0.2341 | 0.2825 | **2.00** |
| 2.0 | 0.2253 | 0.2624 | 1.86 |
| 4.0 | 0.2141 | 0.2375 | 1.68 |

**Only ~24% of the covariance is explained by the coupling.** The remaining ~76%
is residual, and that residual sits at **1.7–2.1× the noise floor** — far above
sampling error.

My pessimistic prediction was wrong. The ensemble is **not** redundant with the
problem statement. There is genuine emergent pairwise information that is *not*
a local function of J, and Ax2 is therefore **not** protected.

### 8.7 The puzzle this creates is now the central fact

Two measurements that must be reconciled:

- Emergent, non-redundant pairwise information exists: residual at 2× noise, 76%
  of variance unexplained by the instance.
- RC-003 converted exactly that statistic into collective moves and gained
  **+0.06%**.

Redundancy is now excluded as the explanation. Something else destroys the value
between "the information exists" and "the algorithm benefits".

### 8.8 Ax10 — the axiom this exposes

> **Ax10 (Move-channel monopoly).** Every algorithm assumes the only channel from
> *knowledge* to *progress* is a move in configuration space. Whatever is learned
> must be cashed out as a proposed transition.

Our 18 operators obey it absolutely. RC-003 obeyed it: it mined real structure and
then, having no other channel available, converted it into a flip.

The historical breakthroughs are precisely the ones that opened a *different*
channel:

| Breakthrough | Channel used instead of a move |
|---|---|
| CDCL | **pruning** — a learned clause excludes a region; it never proposes an assignment |
| Dynamic programming | **tabulation** — knowledge becomes a reusable subproblem value |
| Simplex | **reparameterization** — a pivot changes the basis, i.e. the coordinates |
| MCTS | **allocation** — knowledge redirects future sampling effort, not the current state |

Four breakthroughs, four non-move channels. The SA lineage has only ever had one.

For Ising, pruning is obstructed (weak bounds, §4) and tabulation needs optimal
substructure that a frustrated graph lacks. The remaining channel is
**decomposition**: use the information to change *what the variables are*, not to
move them.

### 8.9 Minimal model, and an honest refutation attempt

If the residual covariance identifies tightly-bound blocks, the correct use is not
to flip a block but to **eliminate** it — solve it exactly and replace it with one
meta-variable. The state becomes a *hierarchy of problems*, not a configuration;
Ax1 and Ax8 both fall, and the variable set itself changes during computation.

**Refutation attempt, and it partly lands.** Marginalizing a block out of an Ising
model generates **higher-order interactions among its boundary**: the coarse-grained
problem is no longer quadratic. Repeated elimination grows the interaction order
without bound. This is the classical obstruction that makes real-space
renormalization fail for spin glasses, and truncating back to pairwise is exactly
the uncontrolled approximation that is known to fail there. Conditioning instead
of marginalizing avoids the blow-up but is no longer exact, and degenerates into
large-neighborhood search — an Ax10-obeying method again.

So the model is **not refuted, but it is obstructed at a nameable point**, and the
obstruction is quantitative rather than philosophical: it depends on how fast
interaction order grows, which depends on block boundary size.

### 8.10 The next measurement, stated before building anything

**P4 — modularity of the residual.** Compute the residual covariance graph
(covariance with the sign(J) component regressed out) and measure its modularity:
is there a partition into blocks with high internal and low boundary coupling?

- If modularity is high **and boundary sizes are small**, the interaction-order
  blow-up is bounded and the decomposition architecture is viable.
- If the residual is high-magnitude but *unstructured* (low modularity), then the
  emergent information is genuinely global and non-decomposable — which would
  explain the 2× signal / 0.06% gain gap completely, and would establish that
  **NP-hardness here manifests as information that exists but is not localizable
  into any channel a bounded-order model can use.**

The second outcome is the more likely one and the more interesting one. It would
be a statement about why the problem is hard, not about why an operator is slow.
