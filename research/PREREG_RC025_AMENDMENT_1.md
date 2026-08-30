# PREREG RC-025 — Amendment 1: the family names carry a size suffix

**Status:** binding. Committed **before any RC-025 datum exists**.
`research/PREREG_RC025_CORPUS_OR_MECHANISM.md` remains immutable and is not
edited; this amendment is read together with it.

## What happened

The registered command was run once. The harness **refused before measuring
anything**:

```
RC025_INSTRUMENT_INVALID  pm1s yielded 0 files, not the 10 PREREG §2 froze
```

`results/rc025/raw.tsv` was empty and no experiment executed. The refusal is the
§5.5 and §2 guard working: the harness matches a family by name followed by a
separator, so it cannot quietly take the wrong files.

## The error

§2 named the Biq Mac families without their size suffix. The files are named
`pm1s_100.0`, not `pm1s.0`. The correct family names are:

| §2 wrote | actual family | files | n | edges | measured weights |
|---|---|---:|---:|---:|---:|
| `pm1s` | `pm1s_100` | 10 | 100 | 495 | 2 |
| `pw01` | `pw01_100` | 10 | 100 | 495 | 10 |
| `w01` | `w01_100` | 10 | 100 | 495 | 21 |
| `g05_100` | `g05_100` — already correct | 10 | 100 | 2475 | 1 |
| `pw05` | `pw05_100` | 10 | 100 | 2475 | 10 |
| `w05` | `w05_100` | 10 | 100 | 2475 | 21 |
| `pm1d` | `pm1d_100` | 10 | 100 | 4901 | 2 |
| `pw09` | `pw09_100` | 10 | 100 | 4455 | 10 |
| `w09` | `w09_100` | 10 | 100 | 4455 | 21 |

**Every other registered fact is confirmed as written.** All nine families exist,
each holds exactly the 10 files §2 froze, every instance has `n = 100`, and each
family's measured weight diversity is exactly the value §2 registered. The
ladder, the densities, the mechanisms, the budget, the seeds, the estimand and
the four frozen outcomes are unchanged. This amendment corrects **file names and
nothing else**.

## Why the suffix is load-bearing rather than cosmetic

`pm1s`, `pm1d` and `g05` each exist at more than one size in this collection —
`pm1s_80` and `pm1s_100`, `g05_60`, `g05_80` and `g05_100`. A family name
without its size does not denote one family. A matcher that accepted a bare
prefix would have drawn `pm1s_80` and `pm1s_100` into the same rung and mixed two
problem sizes inside a comparison that holds size constant. It would have
produced ten files and a plausible number, and nothing would have complained.

That is the reason the harness refuses on a count mismatch instead of taking
whatever it finds, and it is why this error surfaced as a refusal rather than as
a result.

## Scope

The only permitted change is the nine strings in the harness's `PRIMARY` table,
from the §2 spelling to the actual family name. No instance is added or removed,
no rung is re-assigned, no threshold moves. The registered command in §7 is
unchanged. After the correction the run proceeds exactly once, as §7 froze it.
