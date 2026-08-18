# RC-004 — The Architecture Space, and two impossibility proofs

Cycles 001–003 each produced a ~0.06% effect. This document explains why that was
structurally inevitable, and defines the space in which architectures — not
operators — can be enumerated, compiled, refuted and evolved.

---

## 1. Why every cycle produced 0.06%

An architecture is a choice on each of seven axes, each axis being one of the
eliminative questions.

| Axis | The question | Values |
|---|---|---|
| **Σ** state | *Can you search without state?* | configuration-ensemble · marginal-field · none (amortized) |
| **G** guide | *Why is there a cost function at all?* | scalar-energy · violation-multiset · none (feasibility) |
| **D** domain | *Why binary variables at all?* | binary · continuous · probabilistic |
| **K** control | *Can you search without temperature?* | temperature · acceptance-target · none |
| **P** population | *Can you search without replicas?* | single · R replicas · field |
| **M** moves | *Can you search without flips/neighbors?* | designed-local · synthesized · resample · none |
| **T** time | *Must updates be synchronous?* | sweeps · event-driven |

Known algorithms are points in this space:

| System | Σ | G | D | K | P | M | T |
|---|---|---|---|---|---|---|---|
| Simulated annealing | config | energy | binary | temp | 1 | local | sweeps |
| Parallel tempering | config | energy | binary | temp | R | local | sweeps |
| Population annealing | config | energy | binary | temp | R | local+resample | sweeps |
| Simulated quantum annealing | config | energy | binary | temp | R (Trotter) | local | sweeps |
| **WalkSAT / focused walk** | config | **violation** | binary | **none** | 1 | local | event |
| **Belief propagation** | **field** | energy | **prob** | none | field | resample | sweeps |
| **LTGA / GOMEA** | config | energy | binary | **none** | R | **synthesized** | sweeps |
| **Simulated bifurcation** | config | energy | **continuous** | none | R | ODE | sweeps |
| **MemComputing** | config+memory | energy | **continuous** | none | 1 | ODE | **event** |
| **All 18 of our operators** | config | energy | binary | temp | R | local | sweeps |

**Eighteen operators occupy one cell.** Metropolis, Gibbs, Houdayer, ICM,
extremal optimization, replica exchange, population resampling, my consensus
thermostat, my move synthesizer — every one is
`(config, energy, binary, temperature, R, local, sweeps)`. The library's apparent
diversity is entirely *intra-cell*.

That is the structural reason for 0.06%. Cycles 001–003 searched inside a single
cell; the ceiling of a cell is small. RC-003's synthesizer was the only one to
touch a second axis (M = synthesized) and it was the only one to produce a
consistent effect at every temperature — which is a weak but real signal that
axis-changes matter more than value-changes.

## 2. Two impossibility proofs — before any implementation

### 2.1 On unweighted instances, the cost function CANNOT be eliminated

The most radical question — *why is there a cost function at all?* — has a
provable answer on our benchmark.

For Ising energy E(s) = Σ_{(i,j)∈E} J_ij s_i s_j with **J_ij ∈ {±1}**, call an
edge *violated* when J_ij s_i s_j = +1. With V violated and S satisfied edges:

    E = V − S,     V + S = |E|     ⟹     **E = 2V − |E|**

Energy and violation count are related by a **fixed affine bijection**. They are
the same object up to an additive constant.

Therefore a constraint-directed search that ranks configurations by violation
count is *provably ranking them by energy*. On unweighted instances,
`G = violation-multiset` and `G = scalar-energy` are **not two architectures.
They are one architecture written two ways.**

**Verified against the actual data:**

| Family | distinct edge weights | energy ≡ violation count? |
|---|---|---|
| G-Set (all 30 used in RC-001/002/003) | {−1, +1} | **yes — indistinguishable** |
| `dimacs_maxcut/sg3dl051000.mc` | {−1, +1} | **yes — indistinguishable** |
| `biqmac/be100.1.sparse` | **145 distinct**, −100…+100 | **no — genuinely different** |

### 2.2 A benchmark-validity finding

Every one of the 18,570 recorded experiments ran on unweighted instances. On
those instances the energy-guided and constraint-guided computational models are
provably identical. **The corpus could not have detected a difference between
them even if one existed.** This is not a statistical power problem; it is an
identifiability problem, and no number of additional G-Set runs fixes it.

Exactly one weighted family is available locally: **biqmac**. Any experiment
about the guide axis must run there or it is testing nothing.

### 2.3 Temperature cannot be eliminated by reparametrizing to entropy

Restated from RC-003 §3: for a Gibbs ensemble,
`dS/dβ = −β·Var(E) < 0` whenever Var(E) > 0. S(β) is strictly monotone, hence a
bijection, so specifying an entropy trajectory *is* specifying a temperature
trajectory. `K = none` cannot be reached by re-labelling `K = temperature`.

It can only be reached by removing the acceptance rule itself — which is what
`G = violation-multiset` does, since with no scalar objective there is nothing
for a temperature to weight. **The guide axis and the control axis are coupled:
G = none or violation ⟹ K = none.** That is a typing rule, not a heuristic.

## 3. Typing rules — what makes this a compiler rather than a list

The seven axes give 3·3·3·3·3·4·2 = **1944 tuples**, most of them incoherent. The
rules below reject the ill-typed ones so that the remainder can be generated
mechanically:

1. `G ∈ {violation, none}` ⟹ `K = none` *(no scalar ⇒ nothing to temper)*
2. `Σ = none` ⟹ `M = none ∧ P = single` *(amortized inference has no search loop)*
3. `Σ = marginal-field` ⟹ `M = resample ∧ D = probabilistic`
4. `M = synthesized` ⟹ `P = R` *(needs an ensemble to mine — RC-003's law)*
5. `D = continuous` ⟹ `M ≠ designed-local` *(a "flip" is a binary operation)*
6. `P = field` ⟹ `Σ = marginal-field`
7. `G = violation` is **identifiable only on weighted instances** *(§2.1)*

Rule 7 is unusual and important: it is a constraint linking an architecture to
the *benchmark* required to test it. An architecture generator that ignores it
will produce experiments that are provably uninformative — which is exactly what
would have happened had I implemented focused search on G-Set this cycle.

## 4. RC-004 — pre-registration

**Architecture under test:** `(config, violation-multiset, binary, none, R,
designed-local, sweeps)` — the focused / constraint-directed cell. It answers two
eliminative questions simultaneously: there is no cost function, and there is no
temperature, no acceptance rule, no comparison of totals. A violated constraint
is selected and repaired; magnitudes never enter.

**Why this cell:** it is empty in our library, it is the historical origin of a
genuine algorithm class (focused random walk / WalkSAT), and — uniquely — §2.1
gives it a **built-in validity condition** that makes the experiment
self-checking.

**The negative control is the strongest part of the design.** §2.1 *proves* the
architecture must be indistinguishable from energy-guided search on unweighted
instances. So:

- **On G-Set (unweighted): the prediction is NO DIFFERENCE**, and the bootstrap
  95% CI must fall inside ±0.1%. If focused search differs significantly on
  G-Set, the proof is wrong or the implementation is buggy — either way the
  instrument is invalid and the weighted result must be discarded. This is a
  condition whose answer is known a priori, which is what makes it a control.
- **On biqmac (145 distinct weights): the difference, if the architecture has
  content, must appear here and only here.**

**Falsification:**
- Architecture refuted if it differs from energy-guided search on **neither**
  family — it is then a renaming, not a computational model.
- Instrument invalid if it differs on the **unweighted** family (violates §2.1).
- A result under 1% is reported as a principle, never as a win.

**Prediction, stated plainly:** focused search will *lose* to energy-guided
search on biqmac, because treating a weight-100 violation as equal to a weight-1
violation discards real information. If so, the law will be that the scalar
objective is not an arbitrary convention but a *weighting* that heterogeneous
instances require — and the cost function is thereby explained rather than
eliminated.

---

## 5. Status

Delivered this cycle: the architecture space and its typing rules (architectures
are now mechanically enumerable), two impossibility proofs, and a
benchmark-validity finding that invalidates one class of experiment across the
entire recorded corpus.

Not yet built: the focused operator and the biqmac harness. The proofs in §2
changed what that experiment must look like — on the benchmark I would have
reached for by default, it was guaranteed to measure nothing.
