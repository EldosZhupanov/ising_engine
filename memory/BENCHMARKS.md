# memory/BENCHMARKS.md — competitive standing

*Authored here — no other file records the honest competitive position.*
Raw data: `../experiments/results/index.json`. Parsers and statistics:
`../src/benchmark/`. Instance conventions: validated against published optima
(`instances.rs`).

Last updated: 2026-07-27

## Every recorded head-to-head

| Experiment | Engine | vs | n | win frac | Wilcoxon p |
|---|---|---|---|---|---|
| EXP-0000 | `ultimate_v1` | `openjij_sa` | 3 | 1.0 | 0.25 |
| EXP-0000 | `neal_sa` | `openjij_sa` | 3 | 1.0 | 0.25 |
| EXP-0001-smoke | `sparse_colorsweep_v01` | `openjij_sa` | 2 | 1.0 | 0.5 |
| **EXP-0001** | **`sparse_colorsweep_v01`** | **`openjij_sa`** | **23** | **1.0** | **2.66e-05** |

(EXP-0000 rows appear twice in the file — the same two comparisons recorded
across two runs.)

**Exactly one result is statistically strong.** EXP-0001 at n=23 is real. Every
other row is n=2–3 at p=0.25–0.5, which is the smallest p a sign test can
produce at that sample size — i.e. no evidence, not weak evidence. Determinism
checks pass (`ultimate_v1`: cut_a == cut_b == 564.0).

## What has never been benchmarked against

`../benchmark_suite/external/` contains seven cloned repositories:

```
OpenJij  PySCIPOpt  dimod  dwave-neal  dwave-ocean-sdk  or-tools  scip
```

Only **OpenJij** and **dwave-neal** have ever been run against. **CP-SAT
(`or-tools`) and SCIP are sitting on disk, unused.** They are the strongest
available baselines and their absence is the single largest gap in the
competitive record.

## Missing measurement infrastructure

The field's standard currency for solver papers, none of which we produce:

- **time-to-target distributions** and ECDFs (a TTS(0.99) harness exists in
  `solver/tts.rs` but is not used for the competitive record)
- **performance profiles** across an instance family
- **multiple-comparison correction** across the 18,570-run hypothesis history
- comparison against **published best-known G-Set values** in the reported
  figures (baselines are currently internal)

## Scale honesty

Validated up to **n ≈ 2,000** (G-Set). Nothing above that has been tested.
Read [OPEN_PROBLEMS.md](OPEN_PROBLEMS.md) §1 before designing any new
benchmark: G-Set provably cannot distinguish energy-guided from
constraint-guided search, and cannot separate frustration from clustering.

## Standing internal result

`UltimateSolver` **beats evolved research plans 5/5** at equal budget and
identical seeds. No superiority is claimed for the research engine over
production — A/B against `UltimateSolver` is the only production arbiter.
