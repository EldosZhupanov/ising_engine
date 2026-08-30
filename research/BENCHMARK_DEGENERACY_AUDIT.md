# What the standard MaxCut benchmarks provably cannot measure

**Date:** 2026-08-30. **Kind:** analysis record. **Compute:** none — every number
below is read off the instance files. No solver was run and no method's quality
is claimed. Tool: `src/bin/benchmark_degeneracy_audit.rs`. Artifacts:
`results/degeneracy_audit/{audit.tsv,verdict.txt}`.

## The question

`memory/OPEN_PROBLEMS.md` §1 recorded a result this project paid for the hard
way. For an instance whose edge weights all share one magnitude `c`, with `V`
violated and `S` satisfied edges:

```
E = c(V − S),   V + S = |E|   ⇒   E = 2cV − c|E|
```

An affine bijection. **Ranking by violation count *is* ranking by energy**, so
energy-guided and constraint-guided search are one method written two ways. No
quantity of additional runs separates them: this is identifiability, not power.

**18,570 recorded experiments in this repository ran on such instances before
anyone noticed.** The property is decidable by reading the file, so it should
have been checked before the first one.

A second axis behaves the same way. A triangle is frustrated iff the product of
its three couplings is positive, so an **all-positive-weight graph is frustrated
by construction**. A corpus with only frustrated triangles has no control group,
and a frustration split there measures whatever else varies — degree, usually.

This audit decides both questions for every instance of the public MaxCut
corpora held here.

## The verdict

242 MaxCut instances (self-loop-carrying QUBO files excluded and counted
separately):

| corpus | MaxCut instances | guide axis **unidentifiable** | frustration has **no control** |
|---|---:|---:|---:|
| **G-Set** | 30 | **30 / 30 — 100 %** | **30 / 30 — 100 %** |
| DIMACS MaxCut | 34 | 32 / 34 — 94 % | 34 / 34 — 100 % |
| Biq Mac | 178 | 70 / 178 — 39 % | 78 / 178 — 44 % |
| **total** | **242** | **132 — 54.5 %** | **142 — 58.7 %** |

- **92 of 242** instances are degenerate on **both** axes at once.
- **Only 60 of 242 — 25 % — are clean on both** and can answer either question.

## G-Set is the worst case, and it is the one everybody uses

**Every one of the 30 G-Set instances is degenerate on both axes.** G-Set is the
standard MaxCut benchmark: it is where Ising-machine, annealer and heuristic
results are conventionally reported. A published G-Set comparison between an
energy-guided and a constraint-guided method is not a weak comparison — it is
**comparing a method against itself in different notation**, and no amount of
compute changes that.

The same holds for DIMACS `sg3dl*` (32 of 34) and for Biq Mac's `g05_`, `pm1*`,
`pw*` and `t*g` families: all 120 of them are degenerate on at least one axis.

## Which instances survive

The audit's useful half. All 60 clean instances live in two Biq Mac families:

| family | instances | distinct weights | frustrated / total triangles |
|---|---:|---:|---|
| `ising2.5-*`, `ising3.0-*` | 30 | 808 – 2367 | ~48 % / ~24 % |
| `w01/05/09-*` | 30 | 21 | mixed |

Nothing else in 242 files qualifies. **If you want to study the guide axis or
frustration on MaxCut, these 60 instances are what the public corpora give you**
— and a study on 60 instances is a very different design from one on 400.

## How to check this yourself

```
cargo run --release --bin benchmark_degeneracy_audit -- --detail
cargo run --release --bin benchmark_degeneracy_audit -- <any rudy-format dir>
```

The tool reads instance files and reports, per corpus and per instance, the
distinct weight count, distinct magnitude count, triangle census with the
frustrated share, mean degree, and the two verdicts. Its four tests are pinned on
hand-checkable fixtures, including K4's exactly four triangles and the ±1 case —
where two distinct *weights* are still one *magnitude*, so the bijection survives
a sign flip. That case is the easiest one to get wrong and is why the audit keys
on magnitudes rather than weights.

## What is claimed, and what is not

**Claimed.** These corpora cannot separate energy-guided from constraint-guided
search on the stated fraction of their instances, and cannot support a
frustration split on the stated fraction, as a matter of arithmetic about the
files. Both are checkable in seconds by anyone with the data.

**Not claimed.** That any published result is wrong. A paper that never made a
guide-axis or frustration claim is untouched by this. Nothing here says anything
about any solver's quality, speed, or ranking — this audit measures **benchmarks,
not methods**. Nothing here is a wall-time claim; this host is
`INSTRUMENT-INVALID` (`research/RC021_C10_DIAGNOSIS.md`) and no clock was used.

**The general point** is the one worth carrying: *a benchmark can be structurally
incapable of answering a question, and the check costs nothing.* It is cheaper to
run this audit than to run one experiment on the wrong corpus — and this project
ran 18,570 of the latter first.
