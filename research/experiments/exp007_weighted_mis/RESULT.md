---
id: exp007w-result
kind: research-result
status: closed
authority_scope: EXP-007W weighted-MIS CD005 qualification
created: 2026-09-25
immutable: true
---

# EXP-007W: pair-move headroom, no equal-time best-result gain

## Registered outcome

This is one execution of the frozen [protocol](protocol.md) on six
deterministic vertex-weighted Erdős–Rényi graphs. **H1, structural headroom:
SUPPORTED on this corpus.** After production `full_presolve` and complete
1-opt descent, 58/60 seeded states had a strictly improving free edge pair.
Presolve fixed zero vertices in all six graphs; the independent analyzer
recomputed every single- and pair-flip delta from the graph data.

**H2, practical equal-time advantage: NO-GO.** With two seconds per arm in
each of 60 graph-campaign cells, the candidate `with_2opt(true)` won 0,
tied 60 and lost 0 on best feasible vertex-weight sum. All 60 cells and all
60 structural rows were valid. The registered one-sided sign-test p-value is
`1.0` because all primary differences are zero. The gate requiring at least
10 wins across three graphs and at most three losses was not met. A tie does
not prove equivalence on other graphs or budgets.

| Graph | n | Edges | Improving probes / 10 | Best weight in each of 10 campaigns, both arms | Completed solves, baseline / candidate |
|---|---:|---:|---:|---:|---:|
| `er_n64_p08_g0` | 64 | 158 | 9 | 142 | 1,684 / 1,703 |
| `er_n64_p16_g1` | 64 | 325 | 10 | 128 | 1,345 / 1,350 |
| `er_n96_p08_g2` | 96 | 392 | 10 | 190 | 931 / 942 |
| `er_n96_p16_g3` | 96 | 709 | 9 | 143 | 777 / 789 |
| `er_n128_p08_g4` | 128 | 646 | 10 | 252 | 691 / 696 |
| `er_n128_p16_g5` | 128 | 1,282 | 10 | 183 | 569 / 571 |

Totals: baseline 5,997, candidate 6,051 counted solves. There were 59
baseline and 60 candidate late completions, excluded as registered. Actual
arm elapsed times ranged from 2,000.003 to 2,034.444 ms because an already
started solve could finish after the deadline; such solutions were not counted.
Among 5,934 same-seed solves completed in both arms, candidate weight was
higher in 268, equal in 5,666, lower in 0. This is a **secondary descriptive
comparison**, conditioned on both solves finishing. It establishes that the
finisher sometimes improves one returned state, while the preregistered
best-of-time outcome showed no gain. It is plausible that many restarts
already reached the same observed best weight within two seconds, but exact
optima were not established. No SOTA, world-record or generic MWIS superiority
claim follows.

## Claim audit and boundary

| Claim | Evidence | Status |
|---|---|---|
| Unequal vertex rewards can create an improving two-flip after 1-opt | Two-vertex exact witness in [protocol](protocol.md); 58/60 independent structural checks in this run | `SUPPORTED` for possibility and this corpus |
| Enabling CD005 improves the best feasible result at equal time on these six graphs | 60 valid paired cells, 0 wins / 60 ties / 0 losses | `FALSIFIED` for the protocol's practical GO criterion; wider domains `UNVERIFIED` |
| Weighted MIS exchange is a new algorithmic discovery | Existing [weighted-MIS local-search literature](https://drops.dagstuhl.de/storage/00lipics/lipics-vol338-sea2025/html/LIPIcs.SEA.2025.22/LIPIcs.SEA.2025.22.html); no novelty test here | `UNVERIFIED`, not claimed |

The earlier [unweighted CD005-Q2 theorem](../../cd005_equal_time/RESULT.md)
remains valid for unit vertex rewards. Varying **vertex rewards** changes its
swap inequality; merely varying sufficiently large edge penalties need not.
The observed local headroom did not translate into an equal-budget best-result
gain on these fairly small synthetic graphs. Strong specialized weighted-MIS
baselines and real weighted graph families were not tested.

## Independent validation and provenance

Inputs and generator were committed before the instrument at `08f24a9`;
the protocol file is immutable and listed in `memory/BINDING_SHA256`.
Instrument and analyzer were frozen at `f25e2c3` after calibration on a
separate two-vertex graph, full Rust quality gates, three synthetic analyzer
tests and independent pre-data read-only review. The one held-out execution
used the canonical [commands.sh](commands.sh) and evaluated full commit
`f25e2c38c2480fd7b7f127bdb47738d26347437b`. Raw outputs were frozen at
`5b9b391` before this report. Its [metadata](results/run001/metadata.json)
records WSL2/Linux, four logical CPUs, AMD Ryzen 7 170 with Radeon Graphics,
9,373,351,936 RAM bytes, `rustc 1.95.0`, no `RUSTFLAGS` or
`RAYON_NUM_THREADS` override, UTC start/end, seeds, evaluated binary SHA-256
and source-path cleanliness. `git_dirty=true` reflects unrelated pre-existing
untracked files; every experiment source path matched the evaluated commit.

The [independent analyzer](analyze.py) reconstructed all solution weights,
edge collisions and QUBO energies directly from the frozen
[instances.json](instances.json). It rechecked every counted bitstring and
seed, compared the harness-recorded elapsed times to the deadline, and
checked all 60 1-opt states and every free edge-pair delta. The analyzer
cannot independently reconstruct the run's wall clock.
It reported zero invalid cells or probes. Re-running it on the committed
[raw cells](results/run001/raw.jsonl) and
[structural rows](results/run001/structural.jsonl) reproduces
[analysis.json](results/run001/analysis.json) byte-for-byte. Input SHA-256 is
`3c901eaaa0098cc273427e6942227fb5c0c5010f459423e5686819d94878e520`;
evaluated binary SHA-256 is
`d4210eef16ef61a865aca4571a516e6db80d01524afd837190d52d8e3e967e03`.
The post-run [SHA256SUMS](results/run001/SHA256SUMS) covers all committed
run artifacts; key values are structural
`7d7fa5fbda715f836d62670e1eaa07502c89b9c98f788f063bd9e6f3a4f9ccd7`,
timed raw
`b473b215c5c0477ec178f2aeb4401196285b9f3c88f4006f14ddd11e00636ab7`,
analysis
`c119b3abaeeef8c94bbb1bc4cf6c20ef47a19ac04743ae673e8bc61a8733cd05`.

The protocol asked for hashes in the third, raw-results commit. They were
published in this *subsequent* reporting commit instead; the frozen raw,
structural, analysis and metadata files were not changed. The generated
run README also had two incorrect relative links (`../`); its paths were
corrected after the run, and the correction is disclosed in that README.
Neither documentation discrepancy changed an input, algorithm, outcome or
registered decision. No rerun was performed.

Recompute independent validation without running the solver:

```bash
python3 research/experiments/exp007_weighted_mis/analyze.py \
  --structural research/experiments/exp007_weighted_mis/results/run001/structural.jsonl \
  --raw research/experiments/exp007_weighted_mis/results/run001/raw.jsonl \
  --data research/experiments/exp007_weighted_mis/instances.json
```

The new Rust binary passed `cargo check`, `cargo test --release --quiet`,
`cargo build --release --bins`, `cargo clippy --all-targets -- -D warnings`,
`cargo fmt --check`, and `git diff --check` before its freeze. Python tests
passed 3/3. An independent reviewer found no remaining HIGH/MEDIUM issue in
the frozen instrument, and independently reproduced the result summary and
artifact hashes after the run.

## Research decision

CD005 has **mechanistic headroom** on vertex-weighted MIS, unlike this
project's unweighted MIS encoding, but the measured default 2-second campaign
does not justify turning it on for this graph family. A future study would
need a separately frozen corpus with harder, externally sourced vertex-weighted
graphs and a strong specialized baseline. This result does not authorize
retuning the present six graphs or treating the 268 same-seed improvements as
the primary outcome.
