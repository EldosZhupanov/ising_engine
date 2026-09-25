# CD005-Q2 frozen capability handoff

## 1. Objective

Mode C: qualify the already available CD005 two-spin finishing pass on six
held-out QOBLIB MIS graphs under equal five-second wall-clock budgets. This
tests one production toggle and objective encoding, not general 2-opt utility.

## 2. Strongest Claims

- Confirmatory: 18/18 valid paired cells tied in best set size; 0 wins and 0
  losses. The protocol's practical gate returns **NO-GO**.
- Posthoc but proven: for unweighted MIS with unit vertex reward and edge
  penalty `P > 1`, a complete 1-opt local minimum admits no strictly
  improving two-flip among free variables, even with internally fixed spins.
  The current finishing order therefore makes CD005 inert in this encoding.
- Descriptive: all 4,585 same-seed, within-deadline returned state pairs were
  bit-identical; this was not a registered endpoint.

## 3. Canonical Implementation

The only new Rust executable is
[exp_cd005_mis_qualification.rs](../../src/bin/exp_cd005_mis_qualification.rs).
The [fetcher](fetch.py), [analyzer](analyze.py) and [instrument tests](test_tools.py)
are standalone Python. Existing `UltimateSolver` and both solver families
were unchanged.

## 4. Architecture Decisions

Use the existing opt-in `with_2opt` method; keep graph parsing, timed campaigns
and JSONL in `src/bin`. Keep independent feasibility/energy checks outside
solver code. Preserve the six graph files by pinned upstream Git blobs; commit
only their manifest, not redistributed input data.

## 5. Invariants

Arms have identical model, solver parameters and per-solve seed sequence; only
the two-spin toggle differs. Every counted solution finishes by the deadline,
has binary spins, zero selected edges and QUBO energy `-size`. All 18 cells
are retained, including failures. The algebraic claim additionally assumes a
simple unweighted graph, penalty `P > 1` and full 1-opt descent over the free
variables. Fixed spins are allowed and included in local neighbor counts.
Proof and the presolve/decomposition boundary are in [RESULT.md](RESULT.md).

## 6. Test Evidence

The frozen Q2 instrument at `965dca0` passed 3 Python tests and the required
Rust gates: `cargo check`, `cargo test --release --quiet`,
`cargo build --release --bins`, `cargo clippy --all-targets -- -D warnings`,
`cargo fmt --check`, `git diff --check`. The synthetic analyzer test exercises
all 18 cells. An independent read-only reviewer found no HIGH/MEDIUM issue
before Q2 data access. Posthoc finite corroboration checked 1,099 graphs with
up to five vertices and 36,425 pair moves from 1-opt minima; no counterexample.

## 7. Empirical Evidence

[Raw rows](raw_q2.jsonl), [independent summary](analysis_q2.json) and
[source manifest](source_manifest_q2.json) were frozen at `198eeec`. All
18 cells are valid; 0 candidate wins, 18 ties, 0 losses. Baseline completed
4,631 solves and candidate 4,620. Raw, analysis and manifest SHA-256 hashes
are in [RESULT.md](RESULT.md). Recompute with:

```bash
python3 research/cd005_equal_time/fetch.py --output /tmp/ising-cd005-q-holdout
python3 research/cd005_equal_time/analyze.py \
  --raw research/cd005_equal_time/raw_q2.jsonl \
  --data-dir /tmp/ising-cd005-q-holdout
```

## 8. Assumptions

The six official graphs were selected by names, sizes and blob IDs before
their content was parsed. The Q1 fetch touched bytes but produced no solver
outcome; [Amendment 1](AMENDMENT_1.md) records the metadata correction made
before the Q2 run. The five-second window measures this hardware and this
solver configuration, not a transferable throughput constant.

## 9. Known Weaknesses

There are only six graph names and three campaigns each. No official maximum
independent-set optima were compared. The registered outcome is a practical
pilot decision, not an equivalence test. Source bytes are downloaded on
replay. The algebraic result does not cover weighted MIS, lower edge penalties,
frustrated couplings or wider neighborhoods.

## 10. Simplest Plausible Alternative

Run the existing 1-opt finisher alone. For the same pre-finishing state in
this MIS encoding, the optional strict 2-opt pass has no improving free-pair
move; it therefore leaves that state unchanged. The 18 ties and bit-identical
same-seed pairs agree with this explanation.

## 11. Suggested Kill-Tests

For any proposed next domain, first derive or measure the fraction of 1-opt
minima with a strictly improving 2-flip. If zero, stop before a timed campaign.
If nonzero, pre-register equal-wall-time tests against 1-opt alone and a strong
domain baseline. No new experiment is implied by this handoff.

## 12. Freeze Point

Protocol `0284d48`; metadata amendment `e83b989`; Q2 instrument `965dca0`;
raw and independent summary `198eeec`. Canonical claims: Section 2.
Configuration, seeds, primary gate and source blobs: [protocol](PROTOCOL.md)
plus [amendment](AMENDMENT_1.md). Results and limitations: [RESULT.md](RESULT.md).
Remaining decision: choose a *different objective* with pair-move headroom
before revisiting CD005; preserve the current spin-glass evidence separately.
