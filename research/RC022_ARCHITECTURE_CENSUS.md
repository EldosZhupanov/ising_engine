# RC-022 — Architecture census of the human MaxCut/QUBO corpus

**Status:** closed. Analysis only — no compute, no dependency, no LLM, no run.
**Date:** 2026-08-30
**Preregistered as** `research/EXTERNAL_PROJECTS_BACKLOG.md` §8.A, which fixed
the hypothesis and named all three admissible outcomes before any file was read.
**Corpus:** MQLib (<https://github.com/MQLib/MQLib>, MIT), the library
accompanying Dunning, Gupta & Silberholz, *"What Works Best When? A Systematic
Evaluation of Heuristics for Max-Cut and QUBO"*, **INFORMS J. Computing 30(3),
2018**.
**Coordinate system:** RC-004 §1's seven axes — Σ state · G guide · D domain ·
K control · P population · M moves · T time. RC-004 is immutable and is not
modified by this record.

---

## 1. The question

RC-004 concluded that **all eighteen of our operators occupy one cell**,
`(config, energy, binary, temperature, R, local, sweeps)`, and used that to
explain why three research cycles each produced ≈0.06 %. That conclusion was
drawn **from our own operator library**. §8.A asked whether it is a property of
that library or of MaxCut/QUBO heuristics in general, and fixed the rule that
**the classification may not be tuned to produce new cells** — a collapse is as
valuable as a spread.

## 2. Method

Read-and-classify only. The inventory is MQLib's registry
(`src/heuristics/heuristic_factory.cpp`): **20 named heuristic families,
registered as ~37 command-line variants**, the variants differing in components
(GRASP vs VNS, path-relinking, tabu flavour) rather than in axis values.

Each family is placed from its **published mechanism as evidenced in MQLib's own
headers**, not from memory. Where a header states the mechanism directly it is
quoted. Axis values are RC-004's; `config+memory` is used exactly as RC-004 uses
it for MemComputing.

## 3. The census

Our cell is `(config, energy, binary, temperature, R, local, sweeps)`. **Bold**
marks a value differing from it.

| family | Σ | G | D | K | P | M | T | evidence |
|---|---|---|---|---|---|---|---|---|
| burer2002 | config | energy | **continuous** | **none** | **1** | **gradient** | sweeps | `Rank2Cut`, `std::vector<double>* theta`; header: "gradient descent to minimize f(theta) … followed by Procedure-CUT" |
| deSousa2013 | **field** | energy | **prob** | **none** | **field** | **resample** | sweeps | header: "stopping when the **p vector** has converged" — an estimation-of-distribution algorithm |
| pardalos2008 | **config+field** | energy | binary | temperature | R | **local+resample** | sweeps | `GenerateSolution(x, probs, k)`, "generation probabilities"; global equilibrium search |
| glover1998a | **config+memory** | energy | binary | **none** | **1** | local | sweeps | `tabuR_`, `tabuF_`, `recent_` — recency **and frequency** memory over all critical solutions |
| glover2010 | **config+memory** | energy | binary | **none** | R | **local+synthesized** | sweeps | tabu + population (20 population mentions) |
| lu2010 | **config+memory** | energy | binary | **none** | R | **local+synthesized** | sweeps | memetic + tabu |
| palubeckis2004b | **config+memory** | energy | binary | **none** | 1/R | local | sweeps | multistart / iterated tabu |
| palubeckis2006 | **config+memory** | energy | binary | **none** | **1** | local | sweeps | iterated tabu |
| hasan2000 | **config+memory** | energy | binary | temperature | R | **local+synthesized** | sweeps | GA + SA + tabu hybrid |
| beasley1998 (TS) | **config+memory** | energy | binary | **none** | **1** | local | sweeps | tabu variant |
| beasley1998 (SA) | config | energy | binary | temperature | **1** | local | sweeps | annealing variant |
| alkhamis1998 | config | energy | binary | temperature | **1** | local | sweeps | simulated annealing |
| katayama2000 | config | energy | binary | **none** | R | **local+synthesized** | sweeps | memetic, 9 population mentions |
| katayama2001 | config | energy | binary | **none** | **1** | local | sweeps | k-opt local search |
| merz1999 | config | energy | binary | **none** | R | **local+synthesized** | sweeps | GA: CROSS / GLS / MUTATE variants |
| merz2002 | **partial config** | energy | binary | **none** | **1** | local | sweeps | `QUBOPartialSolution`, `RandomizedGreedy` — construction over **incomplete** assignments |
| merz2004 | config | energy | binary | **none** | R | **local+synthesized** | sweeps | memetic, 11 population mentions |
| lodi1999 | config | energy | binary | **none** | R | **local+synthesized** | sweeps | evolutionary |
| duarte2005 | config | energy | binary | **none** | R | **local+synthesized** | sweeps | scatter/evolutionary |
| festa2002 | **partial config** | energy | binary | **none** | **1** | local | sweeps | GRASP construction + VNS + path-relinking |
| laguna2009 | config | energy | binary | **none** | R | **local+synthesized** | sweeps | cross-entropy / scatter search |

## 4. Findings

**4.1 The taxonomy discriminates. §8.A's first outcome holds.**
Twenty families spread across **at least eight distinct cells**, against one for
our eighteen operators. Every family differs from our cell on at least one axis,
and most on two or three.

**4.2 No family in the corpus sits in our cell.**
Our cell needs `K = temperature` **and** `P = R replicas` **and** `M =
designed-local`. Only four families use temperature at all; three of those are
single-walker (`alkhamis1998`, `beasley1998SA`, and SA inside `hasan2000`), and
the two that combine temperature with a population — `hasan2000` and
`pardalos2008` — also carry memory or resampling, so neither lands in our cell.
**The corpus does not occupy the point our whole library occupies.** That is not
a verdict on the cell; parallel tempering is a legitimate and strong method. It
means the human corpus went elsewhere.

**4.3 Where it went, and what we do not have.**

| mechanism | families | our operator library |
|---|---|---|
| **Memory beyond the configuration** (tabu: recency, frequency, aspiration) | **7 of 20** | **none** — verified by search over `src/engine_v2`: no `tabu`, `recency`, `aspiration` or forbidden-move state in any operator |
| **Synthesized moves** (crossover, path-relinking, recombination) | **9 of 20** | RC-003's runtime synthesizer touched this axis once, for **+0.06 %** |
| **Continuous domain with gradient moves** | 1 (`burer2002`) | none |
| **Marginal/probabilistic state with resampling** | 2 (`deSousa2013`, `pardalos2008`) | none — and `RELATIONAL_PRIMITIVE.md` proves per-variable marginals are **vacuous** on MaxCut by Z₂ symmetry, so this cell is *closed to us by theorem*, not merely unoccupied |
| **Partial (incomplete) configurations** during construction | 2 (`merz2002`, `festa2002`), plus MQLib's shared `QUBOPartialSolution`/`MaxCutPartialSolution` infrastructure | none — every operator acts on a complete assignment |

**4.4 The most-used mechanism in the corpus is the one we have zero of.**
Tabu memory appears in seven of twenty families and is the backbone of the
best-performing entries in the accompanying paper. In `AXIOMS_OF_OPTIMIZATION.md`
terms it is an **Ax2 attack**: the object the algorithm mutates stops being only
a candidate solution and becomes *a solution plus a record of where the search
has been*. §5 of that document names Ax2 as the attackable axiom, and
`RELATIONAL_PRIMITIVE.md` closes the marginal route to it. **Memory is the Ax2
route that remains open.**

A frozen legacy `src/solver/tabu.rs` exists, independent of the operator library
(CLAUDE.md §1). It is not in the registry, the Evolution Engine cannot select it,
and no capability passport describes it — so it does not populate the cell for
any purpose the platform can reach.

## 5. What this licenses, and what it does not

**Licensed.** RC-004's one-cell finding is a property of **our corpus**, not of
the problem class. The seven-axis taxonomy separates twenty independently
designed heuristics into at least eight cells, so it is discriminating enough to
steer generation.

**Not licensed.** Nothing here says an unoccupied cell contains a better
algorithm, that memory would help *our* instances, or that any of these families
beats `UltimateSolver`. This is a map of what exists, not a measurement of what
wins. MQLib's own paper is the authority on relative performance and was not
re-run here.

**Not attempted.** No MQLib code was built, executed, benchmarked or added as a
dependency, per §8.A.

## 6. Consequence for the programme

§8.A's stated purpose was to decide which programme is worth running. The answer
is the first outcome: **cells exist that we do not occupy, and one of them —
memory-carrying state — is both the corpus's most common mechanism and the
surviving route to the axiom our own analysis named as attackable.**

The next question is therefore not "which operator is better" but **"what does
an operator look like whose state is not only the configuration"** — and that is
a question for `EXTERNAL_PROJECTS_BACKLOG.md` §8.B and §8.C, which were gated on
this census.
