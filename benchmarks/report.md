# Benchmark Report

Solvers run on identical instances, seeds, and computational budget. Gap is the mean per-run optimality gap vs. the per-instance reference (verified best-known when supplied in `best_known.csv`, else the best objective found by any solver). TTS(0.99) is in per-run wall-clock ms with 95% bootstrap CIs.

**Skipped (unavailable in this environment):** OpenJij, dwave-neal. Their Python bridge scripts are in `benchmarks/adapters/`; install the libraries and rerun to include them.

## Summary

| family | solver | inst | p_success | mean gap | gap CI95 | TTS(0.99) ms | mean ms |
|---|---|---|---|---|---|---|---|
| chimera | PT | 4 | 0.94 | 0.0019 | [0.0000,0.0054] | 0.4518 | 0.27 |
| chimera | SA | 4 | 0.88 | 0.0046 | [0.0004,0.0094] | 0.3063 | 0.14 |
| chimera | Ultimate | 4 | 1.00 | 0.0000 | [0.0000,0.0000] | 232.9817 | 232.98 |
| dense | PT | 4 | 0.03 | 0.0888 | [0.0614,0.1141] | 888.9663 | 6.13 |
| dense | SA | 4 | 0.62 | 0.0020 | [0.0008,0.0037] | 13.4767 | 2.87 |
| dense | Ultimate | 4 | 1.00 | 0.0000 | [0.0000,0.0000] | 373.1322 | 373.13 |
| random | PT | 4 | 0.00 | 0.0877 | [0.0629,0.1116] | inf | 2.34 |
| random | SA | 4 | 0.34 | 0.0019 | [0.0006,0.0034] | 14.5749 | 1.33 |
| random | Ultimate | 4 | 1.00 | 0.0000 | [0.0000,0.0000] | 296.3227 | 296.32 |
| sk | PT | 4 | 0.00 | 0.0965 | [0.0686,0.1228] | inf | 3.16 |
| sk | SA | 4 | 0.53 | 0.0016 | [0.0006,0.0031] | 16.6794 | 2.74 |
| sk | Ultimate | 4 | 1.00 | 0.0000 | [0.0000,0.0000] | 377.6393 | 377.64 |
| sparse | PT | 4 | 0.00 | 0.0801 | [0.0597,0.1008] | inf | 0.66 |
| sparse | SA | 4 | 0.34 | 0.0022 | [0.0010,0.0037] | 8.4272 | 0.77 |
| sparse | Ultimate | 4 | 1.00 | 0.0000 | [0.0000,0.0000] | 257.3920 | 257.39 |

## Significance (Ultimate vs. baseline, per family)

| family | vs | Wilcoxon p | t-test p | Ultimate better |
|---|---|---|---|---|
| chimera | PT | 1.0000 | 0.3910 | yes |
| chimera | SA | 0.3711 | 0.1931 | yes |
| dense | PT | 0.1003 | 0.1602 | yes |
| dense | SA | 0.1003 | 0.1434 | yes |
| random | PT | 0.1003 | 0.1560 | yes |
| random | SA | 0.1003 | 0.3229 | yes |
| sk | PT | 0.1003 | 0.1530 | yes |
| sk | SA | 0.1003 | 0.1026 | yes |
| sparse | PT | 0.1003 | 0.1282 | yes |
| sparse | SA | 0.1003 | 0.0918 | yes |

## Artifacts

- `results_raw.csv` — every run.
- `summary.csv` — per (family,solver) metrics.
- `results.json` — machine-readable summary + significance.
- `tables.tex` — LaTeX booktabs tables.
- `plot_success.svg`, `plot_gap.svg` — figures.
