# Channel Exhaustion — an impossibility result, and the refutation of my own primitive

Outcome **(B)**: not a new primitive, but a rigorous account of why one is not
available for this problem class under the current model. Two of the three
components below refute proposals I made in previous turns.

---

## 1. Theorem 1 — Representation Invariance (kills the relational primitive)

Last turn I proposed *relational state*: work in the quotient of configuration
space by the Z₂ gauge group, holding relations rather than values. Rule 3 says
destroy it before building it.

**Theorem.** Let an algorithm operate on R(x) where R is a bijection, or a
quotient map X → X/G for a symmetry group G. Bijections preserve information
exactly; a quotient removes exactly log₂|G| bits. Therefore the maximum
information-theoretic advantage of the re-representation over working in X is

    **≤ log₂|G| bits.**

**Corollary.** For field-free Ising, G = Z₂ and |G| = 2, so relational state buys
**at most 1 bit — independent of n.**

The relational primitive is therefore **refuted as a source of power**. The
quotient is real, the gauge theorem is real, but 1 bit is not a computational
paradigm. Its second component — "frustration = relational inconsistency" — is on
inspection a *restatement* of MaxCut, not a reduction: deciding which relations to
violate is the original problem verbatim.

This generalizes and kills a whole family: **no change of representation, however
elegant, can alter complexity class.** Ax11 is accidental but powerless.

## 2. Law 2 — the Easy-Information Law (measured; refutes prediction P7)

P7 predicted the emergent covariance residual would concentrate on **frustration
carriers** — edges in frustrated triangles (J_ij·J_jk·J_ki > 0), which are the
facet-defining structures of the cut polytope.

Measured across 30 G-Set instances, ~319,000 edges — very well powered:

| temp_hi | edges in NO frustrated triangle | edges IN a frustrated triangle | lift |
|---|---|---|---|
| 0.1 | n=146,639, mean\|cov\| = **0.5244** | n=172,043, mean\|cov\| = **0.2649** | **0.505** |
| 0.5 | 0.5313 | 0.2713 | 0.511 |
| 1.0 | 0.5163 | 0.2694 | 0.522 |
| 2.0 | 0.4749 | 0.2611 | 0.550 |
| 4.0 | 0.4185 | 0.2425 | 0.579 |

**P7 is refuted, and the sign is inverted.** Frustrated edges carry *half* the
covariance of unfrustrated ones, consistently at every temperature.

In hindsight the mechanism is obvious and I should have predicted it: an edge in no
frustrated triangle can be satisfied consistently, so the ensemble converges on its
relation and the covariance is high. An edge inside a frustrated triangle is one the
ensemble *cannot* settle, so its covariance is suppressed.

> **The Easy-Information Law.** The extractable statistical information in an
> ensemble is concentrated on the sub-problem that is already easy, and is depleted
> exactly on the frustrated core that determines hardness.

## 3. This closes the central puzzle

The session's outstanding question was: emergent, non-redundant pairwise signal at
2× the noise floor (RC-004 §8.6), yet converting it into collective moves gained
**+0.06%** (RC-003).

Earlier candidate explanations, both now dead:
- *Redundancy with J* — refuted (η² = 0.24; 76% of variance is emergent).
- *Wrong channel (Ax10)* — partially true but insufficient.

The actual explanation is Law 2. The signal is real, emergent, and
non-redundant — **and it is about the part of the problem that was never hard.**
Learning it well changes almost nothing, which is precisely a 0.06% gain.

## 4. The four channels, and why each is closed here

An algorithm can only beat brute force by:

| Channel | Mechanism | Status for field-free Ising |
|---|---|---|
| **Acquisition** | queries to the objective | the only channel the SA lineage uses; bounded by query complexity |
| **Retention** | memory carried forward | available (poly-size state) — **but what is retainable is the easy information (Law 2)** |
| **Deduction** | sound inference from what is known | roof-duality persistency is idempotent after proved fixings (RC-003); odd-cycle inequalities are the known proof system, obstructed by separation cost |
| **Representation** | change of coordinates | **≤ log₂\|G\| = 1 bit (Theorem 1)** |

The result is stronger than "the channels are blocked." Retention is *open* — and
useless, because the information available to retain is information about the easy
sub-problem. That is why no amount of better statistics, better linkage learning,
or better move synthesis moves the needle.

**Historical breakthroughs, re-read as channel unlocks:** CDCL unlocked deduction
(SAT has resolution); DP unlocked retention (optimal substructure makes retained
values reusable); Simplex unlocked deduction via LP duality; MCTS optimized
acquisition allocation; Transformer and diffusion unlocked retention *across*
instances. Each succeeded because its problem class had an open channel. Field-free
Ising has none that Law 2 does not drain.

## 5. Predictions that distinguish this from all current theory

No existing optimization theory predicts Law 2. Three consequences, all measurable
before any implementation and none dependent on benchmark tuning:

- **P8 (cross-method).** EDAs, linkage learning, neural CO and diffusion
  optimization must all extract information disproportionately from the
  unfrustrated sub-structure. Their advantage should scale with the *unfrustrated
  fraction* of an instance, not with its size. Refuted if any statistical method
  shows advantage concentrated on frustrated cores.
- **P9 (structural, not tuned).** As frustration density rises from 0 to maximal,
  the relative advantage of every statistics-based method over plain local search
  must decay monotonically to zero. Frustration density is a property of the
  instance, so this is untunable. Refuted by any non-monotone or flat response.
- **P10 (planted instances).** On instances with a planted, frustration-free
  backbone, statistical methods should show *large* gains — far above the 0.06%
  observed here. Refuted if planted structure does not lift them.

P9 is the sharpest: it predicts a *quantitative decay curve* for an entire
methodological family, from a structural parameter, with no free parameters.

## 6. Honest status

**Not a new primitive.** The relational primitive I proposed last turn is refuted
by Theorem 1 (≤ 1 bit) and its operational half is a restatement. I record that
plainly rather than defending it.

**An impossibility-flavoured result instead**, at mixed rigour: Theorem 1 is a
proof; Law 2 is a strong measurement (n ≈ 319k, five temperatures, consistent
sign and magnitude) with an obvious mechanism; the four-channel decomposition is a
framework, not a theorem, and I do not claim otherwise.

**What would overturn it.** Law 2 is the load-bearing claim. If the ensemble
statistics of some *other* sampling process — not Metropolis — concentrated on
frustrated cores instead, the argument collapses and retention reopens. That is
the one experiment worth running next, and it is a question about sampling
processes rather than about operators.

---

# 7. SELF-REFUTATION: Law 2 is dead

Section 6 named Law 2 as load-bearing and invited attack. The attack succeeded in
one run. Two facts, either of which is fatal.

## 7.1 The control group is empty — the test never had a control

Splitting edges three ways instead of two:

| group | n | mean \|cov\| | mean local degree |
|---|---|---|---|
| in no triangle | 146,639 | 0.5244 | **26.3** |
| **in triangles, NONE frustrated** | **0** | — | — |
| in ≥1 frustrated triangle | 172,043 | 0.2649 | **69.3** |

**There is not one unfrustrated triangle in the entire G-Set corpus.** The reason
is structural: most G-Set instances have all-positive weights, so every coupling is
antiferromagnetic and *every* triangle is frustrated by construction; the ±1
instances (G11–G13, G32–G34) are toroidal grids with *no* triangles at all.

So "in a frustrated triangle" and "in any triangle" denote **the same set of
edges**. The P7 measurement could not distinguish frustration from
triangle-membership even in principle.

## 7.2 The degree confound fully explains the effect

Triangle edges have mean local degree **69.3** against **26.3** — a 2.6× density
difference, forced by the definition (a triangle edge has common neighbours). The
measured covariance ratio was 0.505 ≈ ½. Covariance declining roughly inversely
with local degree is ordinary constraint competition and has nothing to do with
frustration.

**Law 2 was a density effect wearing a frustration costume.** Withdrawn.

## 7.3 What survives, and what falls with it

**Survives** (proofs, unaffected by this):
- **Theorem 1, Representation Invariance** (≤ log₂|G| bits) — still refutes the
  relational primitive.
- **Gauge Degeneracy** and its corollary: marginal-state methods are provably
  vacuous on field-free Ising.

**Survives** (measurements, unaffected):
- marginals at 0.96× the noise floor; covariance at 2.77×; η² = 0.24, so ~76% of
  pairwise covariance is not explained by J.

**Falls:**
- Law 2, the Easy-Information Law.
- The §3 resolution of the central puzzle, which rested on it.
- The §4 claim that retention is "open but drained". Retention is simply **open,
  and unexplained**.

**The central puzzle is reopened.** Emergent, non-redundant pairwise signal at 2×
the noise floor yields +0.06% when exploited, and I no longer have an explanation
for why.

## 7.4 A benchmark-degeneracy finding — the second one

This is now the second hypothesis G-Set is structurally unable to test:

1. **RC-004:** all instances unweighted ⇒ E = 2V − |E| ⇒ energy-guided and
   constraint-guided search are provably identical; the guide axis is
   unidentifiable.
2. **Here:** zero unfrustrated triangles ⇒ frustration and clustering are
   perfectly confounded; the frustration axis is unidentifiable.

Thirty instances, 18,570 recorded runs, and two distinct research questions that
this corpus cannot answer *in principle*. The frustration test is possible — it
needs instances with mixed-sign weights **and** triangles, i.e. `biqmac` — but not
here.

## 7.5 Honest final position

Neither Outcome A nor Outcome B has been reached, and claiming either would be the
real failure.

**Not A:** no new primitive. The relational primitive is refuted by Theorem 1.

**Not B:** no complete impossibility theorem. Channel exhaustion needed Law 2 to
close the retention channel, and Law 2 is gone. Representation is provably closed
(≤1 bit); deduction is obstructed but not proved closed; **retention is open**.

**What is actually established:** two theorems — Representation Invariance, and
Gauge Degeneracy with its corollary that an entire family of algorithms is provably
vacuous rather than merely weak — plus a reproducible measurement that emergent
non-redundant pairwise information exists and resists exploitation for reasons now
unknown.

---

# 8. Answering the central puzzle: why 2.7× signal → +0.06% gain

Two diagnostics, both attacking the *measurement* rather than the algorithm.

## 8.1 Q1 — the signal lives only on directly-coupled pairs

Every covariance number reported in this session was measured on **graph edges**,
i.e. pairs that are directly coupled. Correlation there is not emergent — it is
what J *does*. Emergent structure, if any, must appear between variables that are
**not** coupled.

| pair type | n per cell | mean \|cov\| | ratio vs noise floor |
|---|---|---|---|
| adjacent (graph edges) | ~319,000 | 0.32 – 0.39 | **2.29 – 2.77** |
| **non-adjacent** | 120,000 | **0.165 – 0.185** | **1.17 – 1.31** |

Between uncoupled variables the correlation is barely above noise. **Essentially
the entire 2.7× lives on pairs whose correlation is dictated by the coupling.**

## 8.2 Q2 — most of the rest is local degree, also trivially known

| model | η² (variance of edge covariance explained) |
|---|---|
| sign(J) alone | 0.214 – 0.241 |
| **sign(J) × local degree** | **0.417 – 0.552** |

Adding one more quantity readable directly from the instance file more than
doubles the explained variance.

## 8.3 Variance budget, and a correction to my own earlier claim

With R = 32 the sampling noise on each covariance estimate has variance ≈ 1/R =
0.031. Total variance of measured covariance ≈ 0.289²/(1 − 0.241) ≈ 0.110. So:

| component | share of variance |
|---|---|
| local structure (sign J × degree) | ~55% |
| sampling noise (R = 32) | ~28% |
| everything else | **~17%** |

**Correction.** In RC-004 §8.6 I reported η²(sign J) = 0.24 and called the
remaining 76% "emergent". That was wrong. It was 76% *unexplained by one
variable*, and adding a second trivially-known variable absorbs half of it, with
sampling noise accounting for most of the remainder. It was never emergent — it
was under-modelled.

## 8.4 The answer

**The 2.7× is not information about the solution. It is a restatement of the
instance's own local structure.**

The ensemble's pairwise correlations are, to a good approximation, a smooth
function of three things — whether an edge exists, its sign, and how dense the
local neighbourhood is — all readable from the instance file in O(1) per edge
without running any solver.

An algorithm that mines those correlations therefore spends compute rediscovering
what it already had. The +0.06% is the value of the ~17% residual, and it is small
because that residual is what is left after the problem statement has explained
itself. This also explains why the effect was so *stable* across temperatures in
RC-003 (+0.057 to +0.067%): a quantity determined by fixed graph structure does not
vary with temperature.

This is *not* the retracted Law 2. Law 2 claimed the information was real but
concerned the easy sub-problem; that claim died with the empty control. The claim
here is different and directly measured: most of the information is not about the
problem's solution at all, and what is not local structure is largely sampling
noise.

## 8.5 Falsifiable consequence

**P11.** Predict each edge's covariance *analytically* from (edge sign, endpoint
degrees) with no Monte Carlo, and drive RC-003's move synthesis from that
prediction instead of from measured statistics. It should recover the great
majority of the +0.06%. If a zero-sample analytic surrogate matches a
32-replica measured one, the sampling was demonstrably not the source of the
value. Refuted if the analytic version performs substantially worse.
