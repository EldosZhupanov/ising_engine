---
id: cd005-q2-result
kind: research-result
status: closed
authority_scope: CD005-Q2 equal-time MIS qualification
created: 2026-09-25
immutable: true
---

# CD005-Q2 result: no MIS finishing benefit

## Decision and scope

The frozen [protocol](PROTOCOL.md), corrected prospectively by
[Amendment 1](AMENDMENT_1.md), returned **NO-GO** for an equal-wall-time
advantage from enabling the existing CD005 two-spin finishing operator on this
unweighted maximum-independent-set (MIS) QUBO. In 18/18 paired cells, the best
feasible set size tied; candidate wins 0, losses 0. All 18 cells passed the
independent spin, collision and energy checks. This does not retract CD005's
separate spin-glass result or establish equal performance on other objectives.

The stronger, *post-result* explanation is algebraic: for the particular
unweighted MIS encoding used here, any completed 1-opt descent already reaches
a 2-opt local minimum **over the free variables**. The 2-opt finishing pass is
structurally unable to improve the state, regardless of graph topology or
fixed-spin values. This is a scoped algorithmic invariant, not a novelty claim
about MIS theory.

## Frozen comparison

The evaluated instrument was frozen at commit `965dca0`. Both arms called the
same `UltimateSolver::new(2.5, 0.05, 10, 5, Some(seed))` with identical seed
streams; only `with_2opt(false/true)` differed. Each arm received 5.000 seconds
per cell. Only solves finishing before the deadline counted. Three campaigns
on each of six previously unused official QOBLIB graphs gave:

| Graph | Vertices | Edges | Best sizes in campaigns 0, 1, 2 | Baseline / candidate completed solves |
|---|---:|---:|---|---:|
| `C125-9.gph` | 125 | 787 | 34, 34, 34 / same | 530 / 532 |
| `brock200-2.gph` | 200 | 10024 | 12, 12, 12 / same | 118 / 120 |
| `hamming6-4.gph` | 64 | 704 | 12, 12, 12 / same | 785 / 779 |
| `sloane_1zc_128.gph` | 128 | 1120 | 18, 18, 18 / same | 474 / 477 |
| `johnson8-4-4.gph` | 70 | 1855 | 5, 5, 5 / same | 478 / 488 |
| `football.gph` | 35 | 118 | 16, 16, 16 / same | 2246 / 2224 |

Totals: 4,631 baseline and 4,620 candidate completed solves; 18 late
completions in each arm were excluded as specified. Of 4,585 same-seed
completed-solve pairs, all had equal set size. A *posthoc* direct comparison
of the saved bitstrings found 4,585/4,585 identical states. This last state
identity check was not a preregistered endpoint; it is descriptive corroboration
of the theorem below. No claim of reaching the official optimum is made.

## Why the operator cannot help in this encoding

For a simple undirected graph and binary selection vector, the instrument uses

\[
E(x)=-\sum_i x_i + P\sum_{\{i,j\}\in E}x_i x_j,\qquad P=2.
\]

More generally take `P > 1` and allow an arbitrary set of fixed spins. For
each *free* selected vertex, 1-opt forces `k=0` selected neighbors, including
fixed ones: removing it with `k >= 1` would change energy by `1-Pk < 0`. For
each *free* unselected vertex, 1-opt forces `k >= 1`: adding it with `k=0`
would change energy by `-1`. If no spins are fixed, these facts say that the
selected vertices form a maximal independent set. Fixed-spin collisions, if
any, contribute a constant and do not change the free-move argument.

Every possible two-bit move involving *two free variables* has nonnegative
energy change:

1. Remove two selected vertices: `delta = +2`.
2. Replace selected `u` by unselected `v`: with `k_v >= 1` selected neighbors
   before the move, `delta = P(k_v - a_uv) >= 0`, where `a_uv` is 1 if they
   are adjacent, else 0. An exact tie is possible but the operator accepts
   only strictly improving moves.
3. Add two unselected vertices: with `k_u,k_v >= 1`,
   `delta = -2 + P(k_u+k_v+a_uv) > 0`.

Consequently a strict free-pair improvement is impossible after complete
free-variable 1-opt descent. `UltimateSolver` invokes
`steepest_descent_1opt` before the optional `steepest_descent_2opt_escapes`
at both finishing call sites in `src/solver/ultimate.rs`. Although the
experiment passes `&[]` as user clamps, `full_presolve` can derive internal
fixed spins. The proof above includes those spins in each `k` and matches
`local_search.rs`, which scans only free pairs. When `UltimateSolver` splits
free-variable connected components, `extract_component` folds fixed-1 edge
penalties into the local linear fields; each subproblem is the same conditional
energy up to a constant. Across components, pair energy change is the sum of
two nonnegative single-flip changes. The energy values are integral, so the
implementation's `1e-12` improvement tolerance does not affect this
conclusion. A posthoc exhaustive checker
covered all 1,099 simple graphs with 1–5 vertices, their 33,866 states, 3,725
1-opt minima and 36,425 two-flip checks without a counterexample. The proof,
not that finite check, establishes the claim for arbitrary graph size.

The argument depends on equal vertex reward, simple graph edges, `P > 1`,
complete 1-opt descent over free variables and strict improvement. It does not
cover weighted MIS, different penalties, frustrated spin glasses or larger
multi-vertex moves.

## Reproducibility and provenance

Source: pinned `ZIB-AOPT/QOBLIB` commit
`2b400f43c197bb0eb9bc9802efa2b28b818ab63c`; file Git blob and SHA-256
checks are in [source_manifest_q2.json](source_manifest_q2.json). The graph
bytes are fetched by [fetch.py](fetch.py) and are not copied into this repo.
The first fetch attempt, CD005-Q1, stopped on a transcribed blob ID before any
solver run. It accessed bytes from two files and held a third in memory;
[Amendment 1](AMENDMENT_1.md) discloses this and corrects the metadata before
the first complete evaluation, CD005-Q2. No graph content or result was
inspected or used to tune the instrument between the attempts.

Frozen raw outcomes: [raw_q2.jsonl](raw_q2.jsonl), SHA-256
`249543548042a097d4d31a1951d22a6a90add774122ba5d235a53c362baaf889`.
Independent [analysis_q2.json](analysis_q2.json), SHA-256
`133b82129bda925c23f485f3c8292d957f34c17f428782ddb8aa69f096975e47`.
Manifest SHA-256:
`c7769cdf96c830b6ab32a1d1d830c254cb9de9f5e3cbebb140bd13c76b567498`.
Results were frozen at commit `198eeec`. Instrument source hashes are:
`fetch.py` `cc5027e6ccb486ffd006a2186336d14b671b7cd9160e21ac4467ce8f9d43745d`,
`analyze.py` `6beb618268e0b61a09e3d4aa91f19256ac07e6507146bc737db57637c3e4eb20`,
and `src/bin/exp_cd005_mis_qualification.rs`
`475d6ffcfc3fc70da5e59c45d1f5280398e9d9a74a3c29b396a29320c165ee90`.

Replay independent validation from the committed raw data after fetching the
pinned graphs:

```bash
python3 research/cd005_equal_time/fetch.py --output /tmp/ising-cd005-q-holdout
python3 research/cd005_equal_time/analyze.py \
  --raw research/cd005_equal_time/raw_q2.jsonl \
  --data-dir /tmp/ising-cd005-q-holdout
```

The original campaign was run once with:

```bash
target/release/exp_cd005_mis_qualification \
  --data-dir /tmp/ising-cd005-q-holdout \
  --output research/cd005_equal_time/raw_q2.jsonl
```

Environment: Linux x86-64, AMD Ryzen 7 170 with Radeon Graphics; Rust release
binary. Each sequential arm elapsed 5,000–5,079 ms including its mandatory
late completion; only within-deadline completions entered the primary outcome.
Before the Q2 data run, Python instrument tests and all required Rust gates
passed: `cargo check`, `cargo test --release --quiet`,
`cargo build --release --bins`, `cargo clippy --all-targets -- -D warnings`,
`cargo fmt --check`, `git diff --check`. A read-only independent reviewer found
no HIGH or MEDIUM issue in the frozen instrument and amendment.

## Next research decision

Do not apply a graph-topology selector that turns on this particular 2-opt
finisher for unweighted MIS with this encoding. First test whether the
*objective and preprocessing* leave any improving pair moves; graph structure
alone cannot create them here. A new weighted, frustrated or non-MIS use case
would need a separate frozen protocol and matched-cost comparison.
