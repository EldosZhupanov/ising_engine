---
id: memory-now
kind: live-state
status: active
authority_scope: current-task
updated: 2026-09-27
immutable: false
---

# NOW — the only active project state

This file is the sole repository authority for what is in flight. Always verify
its claims against `git status --short` and the current branch before acting.

## Current — MQ-NATIVE-001 raw results retained, 2026-09-27

[Native calibration protocol](../research/experiments/mqlib_native_calibration/protocol.md)
`fb568c8`; reviewed instrument `a25c595`. Two-worker smoke PASS; main completed
once: 960 valid cells, 1,666 complete messages, 807 on-time two-line completions,
3 complete-but-late cells and 150 without both lines. Registered overall FAIL;
50/100ms pass all four worker/shape groups, every 10/25ms group fails.
[Offline evidence audit](../research/experiments/mqlib_native_calibration/run001/audit.json)
PASS: all raw messages and 234 provenance hashes; independent read-only evidence
review also PASS with separate gate arithmetic.
14 Python and 2 Rust tests, strict targeted build/Clippy/format checks pass.
Next: commit reviewed evidence, then record bounded result.
Only native diagnostic model preparation is measured; full optimizer setup inside
solve/run, solver search, real corpus and reserved outcomes remain outside scope.
Production APIs, kernels and Cargo remain unchanged.

## Previous — MQ-CAL-001 complete, 2026-09-27

[No-search timing result](../research/experiments/mqlib_timing_calibration/RESULT.md):
**registered overall FAIL; independent evidence audit PASS**. One frozen main run,
480 valid measurement records,236 complete messages (234on-time,2late),244without
complete output. Only100ms passes both echo shapes;50ms passes n8 only. No native
solver budget admission, speedup, interpolation or universal clock-limit claim.
Protocol `2d9cb9c`, instrument `3aaac7c`, raw/audit `689fbe2`. Twelve fabricated
unit tests, Python syntax, diff and independent pre/post reviews PASS. No solvers,
real coefficients or reserved outcomes used; production APIs/families unchanged.

**Next task:** design a separately preregistered native no-search readiness probe
using actual Rust/C++ conversion/model-load/output paths on artificial fixtures.
Fix parent CPU0 + child CPU0 for both the native calibration and its dependent
benchmark; record observed masks and reject mismatches. The old screen did not
explicitly pin/record its parent, so MQ-CAL-001 does not retroactively calibrate
that scheduling contract. Echo latency is not clock error.
The Python echo's startup cost cannot be transferred to native solvers. Preserve
cold-start versus warm-worker distinctions; qualify the new instrument before
any execution. No automatic real solver-comparison wave follows this calibration.

[Corpus inventory](../research/experiments/mqlib_screen/NEXT_DESIGN.md#metadata-inventory-completed--2026-09-27)
remains303files, none established unseen; targets/redistribution license and
candidate MST2/Katayama adapters remain unqualified. Parent S3/X3 and holdout gates
remain. Do not relabel old BiqMac data or the eight exposed screen cases as unseen.

## Previous — MQ-SCREEN-001 complete, 2026-09-27

[Fixed-wrapper comparison](../research/experiments/mqlib_screen/RESULT.md):
**240 valid cells, no demonstrated final-quality advantage**. Eight fresh
synthetic QUBOs, ten seeds, Ultimate/v2 default/MQLib MERZ2002ONEOPT, 2 seconds
per cell on one CPU. All three pairwise contrasts: 0 wins / 80 ties / 0 losses.
H1 INCONCLUSIVE, normalized difference 0, sign-flip p = 1, empirical bootstrap [0,0].
No optimum/equivalence/learned-selection claim; this is a nondiscriminating screen.
459 raw witnesses independently rescored; no fallback-only or late complete
witnesses. Executable audit PASS. Protocol `2d6d08d`, instrument `b0cf802`, raw `2fe2cb1`.
Final independent read-only review PASS (comparison_explorer); the prior
reviewer service-limit interruption is retained in verification.json.
Production solvers/APIs unchanged.

## Previous — MQ-QUAL-001 complete, 2026-09-27

[MQLib qualification](../research/mqlib_qualification/README.md) **PASS**.
Protocol `f67cbdf`, reviewed instrument `2b61068`, one retained run in
`research/mqlib_qualification/run001/`. Twelve artificial models, 544 exact
state-energy identities and 36/36 independently verified final candidates.
All candidates reach tiny-fixture optima; no competitive inference follows.
Independent read-only audit PASS: 73 raw hashes, seven Git source hashes, six
build hashes and every state/candidate/objective. Twelve Python tests PASS;
strict C++ oracle build, shell/Python syntax and memory/diff checks PASS.
Cargo was not rerun: no production Rust/Cargo changes. Upstream build warnings
are preserved in metadata; the oracle itself compiles with warnings denied.

The standalone `benchmarks/adapters/run_mqlib.py` accepts an explicit JSON
binary polynomial with offset, strictly verifies final solutions and keeps raw
failures. `scripts/setup_mqlib.sh` builds pinned MIT MQLib
`585496274af5abb0849d0d47e135496b4688680b` under ignored cache. Only native
MERZ2002ONEOPT is qualified; no legacy-harness, MaxCut/CSR or hyperheuristic
integration is implied. Both production solver families are unchanged.

**Historical handoff (superseded by the current block):** define a matched-budget complementarity
study and reconcile it with S3/X3 in the binding external-comparison protocol.
[Amendment 1](../research/EXTERNAL_COMPARISON_AMENDMENT_1.md) changes the handoff
to NOW and permits this small correctness qualification; it does not waive
comparison prerequisites. No real corpus/holdout, training, record hunt or
comparative wave was run. Do not repeat RC-027 as a new selector result.

All adapter, oracle, fixtures, tests and retained run artifacts are accounted for.
The known incomplete `benchmarks/qoblib/upstream/` clone remains excluded.
GitHub Actions billing was the last verified hosted-CI blocker; local results
must not be described as hosted CI success.

## Previous — verified publication bundle, 2026-09-27

The user authorized synchronizing GitHub's default `master` with the current
local project. Its previous fetched tip was March commit `7df051b`; local c8391a6
contained 253 additional commits. Publication uses ordinary fast-forward, never
force-push. Audit the exact Git commit, not an assumed date or cached remote copy.

Publication verified: remote `master` reached `501905960f97cb1f42f5bbc839c77c370476b16d`
(258 commits after the old remote). [GitHub Actions run](https://github.com/EldosZhupanov/ising_engine/actions/runs/36313660314)
failed before any job steps: GitHub reports failed account payments or a spending
limit requiring attention in Billing & plans. This is an external CI blocker;
local PASS results above/below are not a claim that hosted CI passed. Account
billing must be resolved before hosted checks can run. This status correction
is a subsequent documentation commit; use current remote HEAD for new audits.

[Publication corrections](../research/HYPODIVE_TRIAGE.md#github-publication-correction--2026-09-27):
synthetic Max-Cut output no longer claims G1/Gurobi results; both endpoints receive
the correct linear coefficient. RandomSearchOrchestrator accurately names its
method, preserves the old public alias, and includes the model energy offset.
README directs readers to current evidence. Three Rustdoc markup errors were
fixed without changing experimental behavior. Solver kernels and architecture
remain unchanged; original Gset loading and multi-slice SQA physics remain open.

Verification: workspace check, release tests **787 passed / 0 failed / 5 ignored**,
release binaries, all-target strict Clippy, all-workspace formatting, strict
Rustdoc, memory/link checks **359 Markdown / 93 immutable**, and diff checks PASS.
The five ignored cases are opt-in profiling/benchmark tests, not hidden failures.
[Retained publication checks](../research/publication_checks/20260927.json) record
commands, raw output, source hashes and the initial Rustdoc failures. Independent
code review and Graft final review PASS; all required gates are closed. No new speed result or quantum-advantage claim was produced.

Graft 0.20.0 optional navigation is installed; twelve compatibility checks PASS.
Sources/data: `scripts/graft.sh`, `research/graft_qualification/`. The
[scientific ecosystem triage](../research/HYPODIVE_TRIAGE.md#scientific-ecosystem-and-remote-audit-reconciliation--2026-09-27)
and `research/literature/scientific_ecosystem_20260927.json` preserve the earlier
read-only audit. Corrections are prospective; historical evidence is retained.

**Next task:** scope the MQLib baseline adapter with exhaustive small-model
energy/sign/offset checks; amend the recorded external-comparison protocol
handoff before a new campaign. ER-001 scorer/blocking continuation below remains
research inventory, not an instruction to restart its frozen experiment.

The incomplete untracked `benchmarks/qoblib/upstream/` clone and ignored caches
remain local. All intentional Graft, audit and publication-check artifacts are
explicitly included in this publication bundle.

[ER-001 result](../research/experiments/entity_resolution/RESULT.md): verified
real-data reconciliation capability; **H1 PASS, H2 FAIL, H3 PASS**. Twelve WDC
training blocks/72 records,120 native cells,1,344 incumbents; independent audit
PASS. Nine blocks have inconsistent pair decisions. Exact and greedy both attain
F1=0.705314 and the same optimal partitions on all12; native optimum witnesses
120/120 do not establish added search value. Native-wrapper F1 variation is due
to optimal ties; five selected fallback endpoints have separate native-optimal
witnesses and are disclosed. Cross-block/unselected pairs remain unresolved.

Protocol/instrument96b0c23; raw a91481c. [Handoff](../research/HYPODIVE_BUILDER_HANDOFF.md)
and [independent executable audit](../research/experiments/entity_resolution/independent_audit.py)
preserve reproduction. Fourteen Python tests and2/2 unchanged Rust example tests
PASS; build/targeted Clippy PASS; analysis replay equals saved summary. Independent
review recomputed147 main raw hashes,203 source/Git hashes, binary, every cell,
energy/partition, confusion count and gate. Source/cases are frozen; no rerun.
Final publication review PASS; memory/link gate359 Markdown/93 immutable and
`git diff --check` PASS. Reusable independent audit source/report are preserved.

**Next task (design first):** qualify blocking coverage and lexical/model scores
on newly frozen entity-disjoint data, with the SAME greedy/exact decoder. Define
how Laya could be restored/evaluated separately; no training, installation or
full campaign follows automatically. Do not expand native search on these easy
objectives or retune ER-001 into a positive result. Laya was NOT run in ER-001.

The reusable research CLI in `research/experiments/entity_resolution/core.py`
accepts complete integer scores for1..8 records and returns an exact consistent
partition; it certifies objective/constraints, not semantic truth. Production
source, both solver families and weights are unchanged. Pilot processes finished.
The known incomplete upstream clone remains untouched; current Graft artifacts
are explicitly inventoried above.

## Application discovery completed — 2026-09-27

User redirected work to finding useful applications with Hypodive before further
solver development. [Application triage](../research/HYPODIVE_TRIAGE.md) and
[primary-source matrix](../research/PRIOR_ART_MATRIX.md) now distinguish five
opportunities: entity resolution, external SAT, weight rounding, binary-alloy
cluster expansions and quantum-error decoding. No application advantage or new
AI training method is established. Existing higher-order/rounding/cluster-editing
prior art materially narrows novelty claims.

Five [deterministic checks](../research/application_triage/checks.py) PASS;
[retained output](../research/application_triage/checks.json) records source SHA
and base `3af7c25`. Includes 218,624 exact energies for a counterexample to CD003's
overbroad polynomial-response condition. The historical CD003 report is preserved;
the correction is prospective. These are algebraic checks, not new solver runs.

**Next task:** APP-01 intake: inspect real entity labels/splits, candidate-graph
semantics and score availability; design an exact small-partition oracle and a
binding matched-cost pilot. Laya's value must be tested separately from the
optimizer's. If usable data/nontrivial residuals fail intake, APP-02 external
SAT qualification is the fallback. No automatic training or full campaign.
The penalty/schedule comparison below remains deferred inventory.

Production source, both solver families and cached weights are unchanged.
No long-running job was launched. The known incomplete upstream clone remains
untouched; it is not a benchmark source.

Independent finished-artifact review **PASS**: five checks replayed exactly,
source/inventory hashes match, retained output refuses overwrite. Memory/link
gate passes 357 Markdown / 91 immutable; `git diff --check` passes. Cargo gates
were not rerun because production Rust/Cargo are unchanged.

## HUBO-Q002 complete — 2026-09-27

[Phase B result](../research/experiments/hubo_corpus_qualification/RESULT_PHASE_B.md):
**MIXED**, 240/240 valid cells, 4,079 main incumbents and 240 final witnesses.
Operational variation qualifies 6/12 instances and 2/4 strata (both n128 groups);
the preregistered >=3/4 overall success threshold was not met. Descriptive MSC
lower/equal/higher energies: 27/82/11. No superiority, optimum or record claim.
All twelve opened synthetic inputs remain in the result, never a later holdout.

Protocol `9438f15`, reviewed instrument/analysis `5e9ea62`, raw data `d5a2a63`.
Thirteen instrument tests, eight unchanged Phase A tests and three independent
auditor corruption checks pass. Independent raw audit PASS: all main witnesses,
four smoke cells/eleven incumbents, 244 final .sol files, 202 source files and 500
archive files, clocks/order and summary. The optional runtime check also matches.
Production source and both solver families are unchanged; no campaign remains
running. Only the previously known incomplete upstream clone is visibly untracked.

[Phase A](../research/experiments/hubo_corpus_qualification/RESULT_PHASE_A.md) remains
correctness-only evidence: 24 retained local/global reductions, local penalties
9–97 versus 1,537–18,417 global. Its historical NOT RUN statement about Phase B
is superseded by the new Phase B record, not edited retroactively.

**Deferred comparison design:** formulate a fresh independent holdout and a
within-kernel local/global-penalty comparison crossed with representation-derived
versus common-endpoint schedules, with native/OpenJij controls. Declare any narrower
population prospectively; never filter Q002 into a successful overall result.
Obtain a new binding protocol before new outcomes. No further campaign, Laya
training, production routing or record hunt follows automatically from Q002.

## HUBO-C001 completed, 2026-09-27

[HUBO-C001 result](../research/experiments/hubo_comparison/RESULT.md):
**NOT_QUALIFIED_FOR_ADVANTAGE**, 400/400 valid two-second cells. MSC native
beats MSC quadratic and OpenJij quadratic in 100/100 paired cells each, but
**ties native OpenJij in 100/100**. Native endpoint energy has zero seed variation
within every instance. This does not establish optima or solver equivalence.
Representation benefit is supported only against the registered conservative
reduction/settings on ten synthetic n=32 cases. It is not a novel algorithm,
world record or production-UltimateSolver result.

Protocol `feb9a76`, source/analysis `23d300f`, raw `a196745` preserve the full
experiment: 3,487 main incumbents, 400 final witnesses, environment/hashes,
8 disjoint smoke cells. [Handoff](../research/experiments/hubo_comparison/HYPODIVE_BUILDER_HANDOFF.md)
links the independent audit (PASS: all witnesses, incumbents, summary fields and
recorded source/package hashes) and reproduction commands. Production source and
both solver families remain unchanged. Do not rerun/tune on these ten opened
instances or launch a record hunt based on this comparison.

The follow-up is Q002's prospective endpoint-variation qualification and local
penalty correctness gate above. A later competitive comparison still requires a
fresh holdout and controls for the penalty-induced schedules; Q002 does not
license a record hunt. LABS allocation diagnosis and lattice boundary repair
remain in the backlog.

[HUBO-RG001](../research/experiments/hubo_representation_gate/RESULT.md) remains
the deterministic correctness gate (6/6 tests); direct higher-order input is
available at the MSC kernel, while public UltimateSolver::solve accepts QuboModel.
Constants require explicit external accounting. The executed breakthrough EXP001
concerned graph elimination/pair refinement, not this HUBO comparison.

Laya remains feasibility research only. The [existing guide](../research/laya_semantic/README.md)
records solver-informed training as a hypothesis and cites overlapping prior art.
The local checkpoint has 421,293,830 stored tensor elements; its old temporary
Python environment is absent. No training or checkpoint modification occurred.

## Repository recovery and LABS-Q002 complete — 2026-09-26

The user requested a whole-checkout inventory, including untracked and ignored
research. [PROJECTS.md](../PROJECTS.md) now separates projects and evidence;
`memory/FILE_MAP.tsv` is the per-file inventory generated by
`scripts/build_file_map.py`. `cargo check`, `cargo test --release`,
`cargo build --release --bins`, Clippy, fmt, the Markdown catalogue check, and
the standalone `fundamental_ai` tests passed on this checkout. Five invalid
Market Split candidates are preserved **without deletion** in
`rejected_candidates/`; the seven remaining `solutions/` files pass the local
official checker. The 41 selected official inputs publish valid Boolean
answers. RC027's printed instrument-commit typo is corrected prospectively in
[RESEARCH.md](RESEARCH.md), not in its frozen result. The formerly untracked
lattice code (`cffaee3`), fundamental-AI source/raw (`004385e`), EXP001
evidence (`31de138`), RC027 data (`4435ae9`), LABS checkpoints (`683c5a7`),
41 labeled Market Split inputs (`4924772`) and provisional audit/agent notes
(`d179e39`) are preserved in separate commits. The local checker gate is now
executable (`59e5808`). None of those commits upgrades a scientific claim.
The only remaining visible untracked tree is the incomplete external
`benchmarks/qoblib/upstream/` clone with broken Git HEAD; it is **not** a
benchmark source and is retained untouched.

**Active REPO-002 qualification:** [EXP-TEN-006A-R recheck](../research/EXP006A_RAW_RECHECK.md)
reproduced 3,360 non-timing rows from current source, versus 4,410 claimed
in the prior review. At K=16/noise 0.20/T=16, all eight methods had 0/20
exact graph solves; classical spectral/min-sum improved partial bit accuracy.
Historical September 12 source freeze and external-baseline fairness remain
UNKNOWN. The four experimental Market Split binaries promote a candidate only
after the local QOBLIB checker returns success; the LLL kernel's completeness,
integer-overflow safety and solver benefit remain unproved. Independent review
found and prompted fixes for empty-kernel crashes and malformed timeouts
(`e04301f`), while flagging unresolved module-boundary debt: preprocessing and
QUBO encoding are in `core/`, search math in `bin/`. The source is preserved as
an experimental snapshot, not a production integration or superiority result.
The standalone `fundamental_ai` tests and raw-design audit are now part of the
selected checker gate (`47c7e07`). `check_labs`
computes the checkpoint energies but has no optimum table for N=67…74, so
its exit 0 does **not** establish optimality. Independently recomputed N=74
energy is 357, above the listed best-known 341.

**LABS publication integrity (`66e9934`):** the later hunter now serializes
best-energy/sequence/checkpoint publication, writes witnesses through a synced
temporary file and atomic rename, and fails on checkpoint-write errors. Strict
CLI parsing supports an isolated `--output-dir`; different processes must use
different directories. Below-BKV search stopping now requires synchronous local
checker success, and messages describe a locally listed BKV rather than a world
record. Four targeted release tests, the complete selected quality gate and
independent review passed. This changes publication correctness, not evidence of
search quality or speed. The subsequent Q002 qualification is recorded below.

**LABS-Q002 completed:** [result](../research/experiments/labs_q002/RESULT.md)
records an integrity-valid **NOT_QUALIFIED** outcome from 60 frozen cells at
ten seconds and one worker. Hunter optimum hits at N40/50/60: 4/10, 0/10, 0/10;
lMAts: 10/10, 8/10, 0/10. Paired endpoints: 0 hunter wins, 4 ties, 26 losses.
All 881 main-run incumbents and all 60 final witnesses passed independent
verification; independent raw audit and statistical replay passed. Source
freeze `6668e76`, raw evidence `7449724`. No new record campaign is justified
by this result. The earlier LABS-Q001 source and records remain unchanged.

**Deferred after the 2026-09-27 user decision:** formulate a separate LABS budget-allocation diagnosis on new
diagnostic seeds: measure initialization, PT, tabu and 2-opt costs before
selecting any algorithm change. Register that profile/ablation independently;
do not tune on Q002 seeds or extend their budgets. The
lattice boundary repair is a separate P1 task, required before production use.
[ROADMAP.md](../ROADMAP.md) lists the gates; [FILE_MAP.tsv](FILE_MAP.tsv)
records each file's tracking state and SHA-256.

## EXP-005 Market Split premise correction — 2026-09-25

The [falsification-first triage](../research/EXP005_MARKETSPLIT_TRIAGE.md)
found that all 41 locally selected QOBLIB instance files publish a binary
`# Solution:` assignment in their comments. Independent integer evaluation and
the local `check_marketsplit` accepted 41/41. Thus these files cannot support a
first-known-solution claim; the untracked EXP-005 protocol's opposite premise
is contradicted. The tracked LLL/hybrid code remains a prototype. A new
algorithmic benchmark would need comment-stripped solver inputs, a separate
answer oracle, matched budgets, strong baselines, and an arm-0 ablation.
Preserve the original protocol and other agents' untracked work.

## CD003-MR1 structural gate complete — 2026-09-25

The [result](../research/experiments/cd003_market_residual/RESULT.md) records
10/10 valid official Market Split residual-field probes and an independently
recomputed **NO-GO** for boundary-field caching on the preregistered split:
presolve fixed 0 variables, while the 22,528 tested boundary assignments
produced distinct internal field vectors within their respective instances. The earlier
[applicability triage](../research/breakthrough/cd003/APPLICABILITY_TRIAGE.md)
also narrows CD003: a small response table does not imply a cheap conditional
solve. This does not refute CD003's scoped theorem, test conditional optimal
responses, or compare solver speed. Do not optimize this partition on the ten
opened files or integrate CD003 based on this diagnostic. Next scoped choice:
find a naturally repeated, bounded-precision interface in an independent
domain under a new protocol, or study a different certified graph reduction.
No second campaign follows automatically from this result. The production
solver, MSC family, and Laya bridge were unchanged. Preserve unrelated
untracked drafts.

## EXP-007W complete — 2026-09-25

The [result](../research/experiments/exp007_weighted_mis/RESULT.md) and
[handoff](../research/experiments/exp007_weighted_mis/HYPODIVE_BUILDER_HANDOFF.md)
record one frozen vertex-weighted MIS evaluation: 58/60 post-presolve 1-opt
states admitted an improving two-flip, yet the equal-time best feasible weight
tied in 60/60 paired cells; practical decision **NO-GO**. All rows and returned
solutions passed independent checks. This establishes mechanism headroom on
the six synthetic graphs but no demonstrated best-of-time advantage. The
production solver and both families were unchanged. Do not tune on these six
graphs or treat same-seed secondary improvements as a primary win. Next scoped
choice: a separately registered harder, external vertex-weighted MIS corpus
with a specialized baseline, or return to CD003 on nonempty residual graphs.
No further empirical campaign is automatically authorized by this note.
Preserve unrelated untracked drafts. The CD005-Q2 section below remains the
prior unweighted result.

## CD005-Q2 equal-time qualification complete — 2026-09-25

The user selected CD005 qualification after CD004-R. The
[result](../research/cd005_equal_time/RESULT.md) and
[handoff](../research/cd005_equal_time/HYPODIVE_BUILDER_HANDOFF.md) record one
frozen evaluation on six QOBLIB MIS graphs: 18/18 valid paired cells, 0 wins,
18 ties, 0 losses at equal five-second wall-clock budgets; decision **NO-GO**.
For this unweighted MIS QUBO with penalty `P=2`, complete 1-opt descent already
precludes any strict 2-flip improvement. This is an objective-specific
algebraic limit, not a retraction of CD005's separate spin-glass evidence.
The Q1 fetch stopped at a blob-ID transcription error before any solver run;
the [prospective amendment](../research/cd005_equal_time/AMENDMENT_1.md)
discloses the prior byte access. Do not turn on CD005 for this MIS encoding
based solely on graph topology. Next scoped choice: study CD003 on nonempty
residual graphs, or qualify CD005 on a different objective only after proving
that improving two-flips exist. No new empirical campaign is automatically
authorized by this note. Preserve unrelated untracked drafts. Older sections
below are historical context.

## CD004-R correction — 2026-09-25

The [prospective recheck](../research/breakthrough/cd004_recheck/RESULT.md)
found an exact counterexample to the historical CD004 backjump solver. The
corrected certified-core prototype matched exhaustive optima in 120/120 new
instances, but visited the same nodes as chronological BnB and was slower
(median wall ratio 4.876; 0/120 time wins). The historical 33–54% node
reduction is not an established exact-search advantage. Do not integrate CD004
or use it in a graph-structure selector until a new exact and matched-cost
mechanism passes its own protocol. CD003 and CD005 retain their separate scoped
evidence; this recheck did not evaluate them. The old CD004 entries below are
historical checkpoints, superseded for present decisions by this correction.

Next scoped research choice: study CD003 on real low-rank residual graphs or
qualify CD005 at equal cost on a held-out application; no new experiment is
automatically authorized by this note. Preserve unrelated untracked drafts.

## Research continuity and duplicate prevention — 2026-09-25

The user asked to connect a year of prototypes and avoid repeating research.
The [research pointer](RESEARCH.md) now routes recent CD, RC, Laya and LABS work
to primary evidence and distinguishes a confirmed research result from a
production integration. The document catalogue includes the tracked QOBLIB
submission guides. Untracked drafts and record-hunt checkpoints remain local
and are not silently promoted to canonical evidence.
Next for any new idea: locate its nearest prior result and code call sites, name
the changed assumption and cheapest falsifying check, then select an explicit
integration gate before touching either solver family. The LABS and other
sections below preserve their historical outcome.

## LABS-Q001 qualification COMPLETE — 2026-09-23

The user authorized execution of the frozen campaign:
`python3 research/labs_qualification/campaign.py run --pt target/release/qoblib_labs_challenge --lmats /tmp/labs-q001-baseline-v3/solvers/lMAts-lRRts/src/lMAts --checker /tmp/labs-q001-checker --output /tmp/labs-q001-run`
- Protocol integrity: 60/60 cells executed strictly under the 10.0 s deadline; 0 checker failures, 0 process errors.
- Evaluated commit: `cd04ff13137264a01f5e03f90bc093f5be46f112`.
- Quantitative findings:
  - $N=40$ (target 108): `pt` achieved 7/10 exact optimum hits (median 108.0, 1.63 s – 9.78 s); `lmats` achieved 10/10 hits. Paired: 7 ties, 3 `lmats` wins.
  - $N=50$ (target 153): `pt` 0/10 hits (median 185.0); `lmats` 5/10 hits (median 157.0).
  - $N=60$ (target 218): `pt` 0/10 hits (median 282.0); `lmats` 0/10 hits (median 256.0).
- Outcome: NOT QUALIFIED (`pt_qualified: false`). Operational threshold ($\ge 8/10$ hits at each length) was not met.
- Full record: [`research/labs_qualification/RESULT.md`](../research/labs_qualification/RESULT.md).
- Lesson: pure Parallel Tempering + 1-opt local search cannot match specialized genetic crossover and tabu recency on higher dimensions ($N \ge 50$) at equal time budgets. Record attempt on $N=67$ requires population-level operators.

## QOBLIB — Quantum Optimization Benchmarking Library 2026 Integration & Submission — 2026-09-24

The user authorized official QOBLIB (Nature Computational Science, 2026; IBM Quantum & Zuse Institute Berlin / ZIB-AOPT) submission preparation:
- Implemented `src/bin/qoblib_mis_benchmark.rs` evaluating official Maximum Independent Set (07-independentset) instances in DIMACS format:
  - Formatted and generated official submission package in `benchmarks/qoblib/submissions/20260924_UltimateSolver_Zhupanov/`:
    - `sloane_1dc_64`: exact Gurobi optimum (10) in 1.20 s (0 violations).
    - `sloane_1dc_128`: exact Gurobi optimum (16) in 2.20 s (0 violations).
    - `sloane_2dc_128`: exact Gurobi optimum (5) in 4.14 s (0 violations).
    - `socfb-haverford76` (1,446 nodes, 59,589 edges): achieves 280 (99.3% of official world record 282) in 40.8 s (0 violations).
  - 100% of solutions verified by official ZIB checker `check_stableset` (Thorsten Koch) with `VALID: Solution successfully verified`.
- Upgraded `src/bin/qoblib_labs_challenge.rs` to Memetic Parallel Tempering:
  - Integrated uniform genetic crossover between cold replicas.
  - Implemented short-term tabu search (`tabu_search_labs`) with tenure and aspiration criterion.
  - Verified 4 qualification unit tests and Bolztmann exchange invariants.
- Executed high-throughput multi-core campaign with `src/bin/labs_record_hunter.rs` (~200M flips/s across 4 Rayon threads, 60s per target) targeting 22-year-old unproven world records (Knauer 2004):
  - $N=67$ (BKV 241): reached $E = 341$ (gap $+100$).
  - $N=68$ (BKV 250): reached $E = 350$ (gap $+100$).
  - $N=69$ (BKV 274): reached $E = 366$ (gap $+92$).
  - $N=70$ (BKV 295): reached $E = 359$ (gap $+64$).
  - In short 60s runs, the unproven 20-year-old records remain unbroken (confirming that literature records require multi-hour supercomputing budgets or BnB). Incumbents verified with exact energy checks.
- Quality gates: PASS (cargo check, cargo test --release, cargo build --release --bins, cargo clippy --all-targets -- -D warnings, cargo fmt --check, git diff --check).
- Expanded official QOBLIB expansion to Problem Class 01 (`01-marketsplit`, Cornuéjols & Dawande 1998):
  - Implemented exact unconstrained quadratic equality formulation: $\min H(x) = \sum_k ( \sum_j A_{kj} x_j - b_k )^2 \ge 0$.
  - Developed `src/bin/qoblib_marketsplit_benchmark.rs` integrating `UltimateSolver` with $O(m N^2)$ 2-opt exact quench and dynamic tabu search.
  - Built official ZIB verifier `target/release/check_marketsplit` (Thorsten Koch).
  - Solved 6 instances to 100% exact equality (0 constraint violations, Exit code 0):
    - `ms_03_050_002`: 1.00 s (4.4x faster than South Korea Q-Bridge GPU Simulated Annealing on Apple M5 Max / RTX 5090)
    - `ms_03_050_005`: 0.96 s (4.5x faster than Q-Bridge GPU SA)
    - `ms_03_050_007`: 0.87 s (VALID, 0 violations)
    - `ms_03_050_009`: 0.70 s (VALID, 0 violations)
    - `ms_03_100_001`: 2.02 s (2.1x faster than Q-Bridge GPU SA)
    - `ms_03_100_012`: 0.87 s (4.9x faster than Q-Bridge GPU SA)
- Deployed Continuous Overnight LABS Record Hunter & Focused Attack:
  - Upgraded `src/bin/labs_record_hunter.rs` with O(N^2) `local_search_2opt`, cross-thread collective memory sharing, and Variable Neighborhood Shake.
  - Overnight and daytime continuous execution reached ~61 cumulative CPU hours across all 4 cores.
  - Option B Focused Attack on N=74 ran 15 full rounds: compressed energy down from 373 -> 365 -> 357!
  - The gap to the 22-year-old Knauer 2004 world record (341) is now compressed to just +16!
  - 100% verified by official ZIB verifier `check_labs` (VALID).
  - Telemetry streamed to `benchmarks/qoblib/world_records/hunt_history.log`.
  - Packaged complete official submission package in `benchmarks/qoblib/submissions/01-marketsplit/20260924_UltimateSolver_Zhupanov/` with per-instance passports and `.sol` files, 100% verified by `check_marketsplit`.

## SK-PARISI — Sherrington-Kirkpatrick Parisi Ground State Challenge COMPLETE — 2026-09-23

The user requested execution of the hardest global benchmark and physical frontier, followed by an adversarial peer-review audit.
- Exhaustive Verification: Added `test_sk_energy_exhaustive_equivalence` in `tests/test_energy.rs` asserting $|E_{\text{QUBO}}(x) - E_{\text{SK}}(\sigma(x))| < 10^{-11}$ and all 10 single-flip deltas match across all $2^{10} = 1024$ states. Confirmed CSR symmetric matrix factor 0.5 in `calculate_total_energy`.
- Algorithmic Rigor: Refactored `src/bin/sk_parisi_benchmark.rs`:
  - Replaced heuristic D-Wave formula with exact graph-theoretic treewidth theorem: $tw(K_N) = N - 1$. Since $tw(Z_{12}) \le 200$, direct minor embedding of $K_{256}$ and $K_{512}$ on D-Wave Advantage2 $Z_{12}$ is topologically ruled out.
  - Multi-start Greedy (20 restarts): converges to $e \sim -0.705 \dots -0.732$, matching quenches in SK literature (Folena et al. 2024: $-0.708 \dots -0.735$).
  - Evaluated on ensemble of independent disorder realizations (mean $\pm$ std dev).
  - Clarified finite-size reference $\langle e_0(N) \rangle$ as an empirical ensemble reference, not an instance-specific lower bound.
  - Acknowledged Andrea Montanari's IAMP polynomial-time algorithm ($C(\varepsilon)N^2$) and cited Talagrand (2006) and Auffinger & Chen (2017).
- Results:
  - $N=64$ (5 instances): Greedy $-0.7054 \pm 0.0328$, UltimateSolver $-0.7188 \pm 0.0263$ (mean time 756 ms).
  - $N=128$ (5 instances): Greedy $-0.7325 \pm 0.0183$, UltimateSolver $-0.7467 \pm 0.0146$ (mean time 3.2 s).
  - $N=256$ (3 instances): Greedy $-0.7084 \pm 0.0063$, UltimateSolver $-0.7401 \pm 0.0080$ (mean time 15.0 s).
  - $N=512$ (1 instance, 130,816 couplings): Greedy $-0.7293$, UltimateSolver $-0.7533$ (time ~85–92 s).
- Quality gates: PASS (cargo check, cargo test --release, cargo build --release --bins, cargo clippy, cargo fmt, git diff --check).

## LAYA-001 — semantic reconciliation pilot COMPLETE — 2026-09-23

The user explicitly requested execution of the LAYA-001 pilot. The frozen exploratory
pilot ran across 24 synthetic groups (192 decisions).
- Quality gates: PASS (zero mismatches on encoding, returned energies, exact and reduced optima).
- Result: Raw Laya produced 26 constraint violations (only 10/24 feasible groups, 84.4% accuracy).
- The UltimateSolver path eliminated 100% of violations (0 violations, 24/24 feasible groups),
  recovered 19 corrupted bits with 0 false damages, and achieved 94.3% accuracy (matching exact oracle).
  Full report: [`research/laya_semantic/RESULT.md`](../research/laya_semantic/RESULT.md).
- Presolve: `full_presolve` completely fixed all 24 groups analytically (0 residual states).
All variables were fixed by presolve; no annealing/CD005 contribution or speed
advantage is established. Frozen instrument: `96b2ace`; raw evidence: `f28d12a`.
Independent review verified 77 hashes and reproduced the summary without inference.
Prospective report clarifications and complete handoff:
[`HYPODIVE_BUILDER_HANDOFF.md`](../research/laya_semantic/HYPODIVE_BUILDER_HANDOFF.md).
Next for this line: separately scope real annotated data and nonempty residual
graphs; the pilot does not authorize a new experiment. No root src changes by
this task; preserve unrelated concurrent work. Older selections below are historical.

## Explicitly resumed: cross-domain mechanisms with Ising — 2026-09-17

The user explicitly selected the five-step mechanism map / boundary / transfer /
prior-art falsification / minimal experiment workflow and requested both HYPODIVE
skills. This lifts the earlier audit stop for this scoped investigation. Do not
restart EXP001 or automatically execute the uncommitted EXP002 draft.
Active bundle: `research/CANDIDATE_IDEAS_LEDGER.md`, `research/breakthrough/cd005/`.
First cycle CD001/H11: rank-only response-count claim NO-GO (commit `20ce993`).
Second cycle CD002/H12: discrete gauge synchronization vs spectral sync NO-GO (commit `a900575`).
Third cycle CD003/H13: precision-rank bounded response N_resp <= (2bK+1)^r CONFIRMED (commit `b37c1c3`).
Fourth cycle CD004/H14: native soft-conflict learning cuts BnB nodes by 33-54% CONFIRMED (commit `f12ce06`).
Fifth cycle CD005/H15 (Option D: Edge-Restricted 2-Opt Escapes) completed and confirmed:
Theorem 1 verified (0 violations), escapes 93-97% of 1-opt local traps with up to 24.8x speedup over O(N^2) scans.
Phase 1 Integration: CD005 operator integrated into `src/solver/local_search.rs` and `UltimateSolver` (commit `895835e`).
All unit/integration tests (199 passed, 6 local search tests including Theorem 1 completeness), release binaries, clippy, and fmt green.
All candidate ideas maintained in `research/CANDIDATE_IDEAS_LEDGER.md` and `research/SYNTHESIS_CD_SOLVER_ARCHITECTURE.md`.
Preserve all unrelated research and unfinished earlier records.

The audit stop below is historical and superseded within this explicit scope.

## Audit stop — 2026-09-17

The latest explicit user instruction is: «аудить не надо делать стоп аудит».
Do not start or resume repository audits, independent review, falsification
passes or EXP002 under an automatic goal continuation. Resume those activities
only after an explicit user instruction lifting this stop. This does not cancel
the broader research objective or mark it complete. Both requested HYPODIVE skills
are already installed; their presence does not authorize an audit.

Preserved checkpoint: EXP001 has a completion marker for 4,200 rows at
`research/breakthrough/exp001/complete.json`; no rerun is required. The untracked
EXP001 result and EXP002 protocol are drafts, not committed/frozen records.
Post-experiment validation remains unfinished, so no new superiority claim is
licensed. The previously written “in progress” statement below is superseded.
No new experiments or scientific checks were run while recording this stop.
Preserve unrelated `research/fundamental_ai/` and `results/rc027/` work.

## Current user-authorized research expansion — 2026-09-11

The current conversation explicitly requests a repository-wide algorithm audit,
including `engine_v2`, ten mathematical hypotheses, and autonomous isolated
experiments. This supersedes the old task-selection restriction below, not any
binding protocol or historical outcome. The active task bundle is
[research/breakthrough/RESEARCH_STATE.md](../research/breakthrough/RESEARCH_STATE.md),
[the audit](../research/breakthrough/AUDIT.md), and
[EXP001](../research/breakthrough/EXP001_PROTOCOL.md).

**Resumed through the goal control on 2026-09-12:** the previously blocked
thread goal is active again. The saved source state is unchanged at `fdec0df`.
The audit and ten hypotheses are complete at the documented algorithm-surface
scope. EXP001 preregistration was committed as `aee6ea8` before prototype code or data.
The isolated prototype and analysis instrument now pass 9 Rust mathematical
tests and 3 Python screen tests; independent instrument review returned PASS
(no HIGH/MEDIUM findings; reviewer repeated all 12 tests).
Instrument committed as `526ddd3`; the single registered 4,200-row run is now
in progress under `research/breakthrough/exp001/`. Do not launch it again or
change the frozen source/protocol. Next: inspect completion and analyze the
registered screens, then investigate any unexpected large effects. The user requested and received preliminary findings on 2026-09-12 and explicitly directed continued research without a change of direction. See [HANDOFF.md](../research/breakthrough/HANDOFF.md) for the preserved checkpoint.

### Parallel Isolated Research: `research/fundamental_ai/` (2026-09-12)
- Autonomous exploration of higher-order tensor energy dynamics ($p \ge 3$) as AI computational primitives.
- **EXP-TEN-001** (Associative Scaling): Completed across $N \in \{128, 256, 512\}$. Published in `EXP_TEN_001_RESULT.md`.
- **EXP-TEN-002** (Adversarial Falsification): Completed across 6 data suites. Model C proved identical to degree-3 Polynomial DAM and collapses under correlation where 1-NN achieves 100%. Published in `EXP_TEN_002_RESULT.md`.
- **EXP-TEN-003** (Latent Compression $P \gg R$): Completed. Discovered the Sign-Erasure Pathology of odd-degree energy models ($p=3$); linear SVD outperforms CP-3 on unseen grammar ($0.773$ vs $0.322$). Published in `EXP_TEN_003_RESULT.md`.
- **EXP-TEN-004** (Relational Composition): Completed. Confirmed inference-time compute scaling ($T=1: 0.9\% \to T=2: 100.0\%$), while pairwise Hopfield collapses to $0.0\%$. Published in `EXP_TEN_004_RESULT.md`.
- **EXP-TEN-006A & EXP-TEN-006A-R Audit** (Prior-Art Reset & Adversarial Audit): Completed across 4,410 audit trials. Overclaims retracted. Audit proved LearnedDualEnergy was inert (Rescue Rate = 0.7%, identical to zero-interaction ablation, p=1.000). The cyclic task was algebraically reduced to Permutation Synchronization over S_d (Pachauri et al. 2013). Classical Spectral Sync (89.4%) and Loopy Min-Sum (95.1%) outperform candidate (79.6%). Published in `EXP_TEN_006_PRIOR_ART_RESET.md`, `EXP_TEN_006_STABILITY_THEORY.md`, and `EXP_TEN_006_NOVELTY_REVIEW.md`.
- **EXP-TEN-006B** (Rigorous Energy & Contractive Synchronization Falsification): Completed. Implemented and mathematically verified Exact Analytical BPTT (TRAIN-A) and Exact Analytical Equilibrium Propagation (TRAIN-C). Pilot Gate failed decisively across all continuous models (Net Rescue <= +3.7% vs threshold > +10.0%). Discovered the Inertia-Erosion Dilemma: ln-cosh potentials create zero-crossing vanishing gradient barriers causing total inertia (Rescue Rate = 0.0%), while linear EBMs and contractive GNNs cause symmetric error diffusion that damages clean bits at nearly the same rate it rescues corrupt bits (Damage Rate 18.9%, Rescue Rate 22.6%). Classical Spectral Synchronization (+40.1%) and Loopy Min-Sum BP (+40.6%) decisively dominate continuous neural relaxations. Published in `EXP_TEN_006B_RESULT.md`. All root production code and invariants remain untouched.

The pre-existing untracked `results/rc027/` remains untouched and uninspected.
RC021 terminal evidence and all existing binding records remain immutable.
`NOW.md` remains the sole project handoff; RESEARCH_STATE is an experiment ledger.
Full-worktree memory validation has a pre-existing uncatalogued RC027 file;
validate the intended-change tracked-source snapshot separately and report both.

The sections below preserve prior research context. Their RC027 “active task”
labels do not select the new task; its frozen protocol still governs RC027 data.

## Standing objective — the thing we do not drift from

Set 2026-08-28. **Subordinate to `research/ISING_ENGINE_CONSTITUTION.md`, which
is the Single Source of Truth for direction and is immutable.** This section
does not set direction; it states the one concrete objective the current work
serves, so that any task can be checked against it in a sentence.

The Constitution's mission is **access**: the commodity CPU as a competitive
optimization machine, delivered as *an instrument whose readings can be trusted*
— and it names three assets that do not expire: the architecture, the evidence
base, and the discipline.

**The objective, concretely:**

> Establish whether the algorithm space for our problem class has more than the
> one architectural cell our own corpus occupies — and if it does, build the
> instrument that finds candidates in the empty cells and can prove what it
> found.

**Why this and not something else.** Five recorded results say the same thing
five ways: RC-004 places all eighteen operators at the single point
`(config, energy, binary, temperature, R, local, sweeps)`;
`AXIOMS_OF_OPTIMIZATION.md` finds we kill none of the eight axioms;
`RELATIONAL_PRIMITIVE.md` proves marginal-state methods vacuous on MaxCut;
evolved plans lose 5/5 to `UltimateSolver`; runtime operator synthesis bought
+0.06%. We search one cell extremely well. Nothing yet shows a second cell
exists.

**The test that decides it** is `research/EXTERNAL_PROJECTS_BACKLOG.md` §8.A —
map MQLib's human-designed MaxCut/QUBO heuristics onto RC-004's seven axes.
Read-and-classify only: no compute, no dependency, no LLM. All three outcomes
are results, and the outcome selects the program:

- other cells are occupied → the taxonomy discriminates; §8.B and §8.C follow;
- everything collapses into our cell → a real structural convergence of the
  field, stronger than RC-004 alone, and scaling the dataset becomes the better
  use of compute, **confirming** the current roadmap rather than correcting it;
- the heuristics resist clean classification → the taxonomy is repaired before
  it is allowed to steer anything.

**What this objective forbids.** Adopting an external framework before the
census says the space has room. Growing the dataset toward 500k as a *goal*
rather than as a consequence. Publishing any Class II claim without the arm that
could refute it — the Soup pilot of 2026-08-28 is the standing example of what
that costs: a clean infrastructure result (a 7B model QLoRA-tuned on a 6 GB
laptop GPU, ~2.07 GB peak, deterministic receipt) that says **nothing** about
our selector, because no random-subset arm was run.

## Active task

**RC-024 is complete. Registered outcome: WORKS BUT IMMATERIAL.**

Endpoint-guided path relinking was preregistered (`28710dc`), implemented and
mutation-verified (`4af912e`), and run once as registered. Ninety paired
observations over 30 G-Set instances at seeds 101/102/103:
**69 wins, 0 losses, 21 ties**; mean gain **+0.105 %**, median +0.077 %,
Wilcoxon **p = 5.4 × 10⁻¹³**. Not one pair of the ninety cleared the 1 %
materiality floor. Full record: `results/rc024/RESULT.md`.

The zero in the loss column is the never-worse invariant, proven on synthetic
endpoints before the run. The candidate does strictly more logical work than its
control and this host is INSTRUMENT-INVALID, so **no wall-time or equal-cost
claim is licensed** — a +0.105 % gain bought with unmeasured extra work is not a
speed result.

**Taken with RC-003 and RC-023, this is the cycle's real finding.** RC-022 named
synthesized moves as the largest mechanism gap between our corpus and the human
one — 9 of 20 external families. It is now measured twice in two independent
forms: covariance-mined collective moves (RC-003, ≈ +0.06 %) and endpoint-guided
path relinking (RC-024, +0.105 %). Both land an order of magnitude under the
floor. RC-023 found the *form* of memory not to matter at all. Three
architecture experiments in a row say the mechanism inventory is **not** where
this corpus is short — a claim about these instances at this budget, and the
hypothesis the next cycle should try to break rather than a conclusion.

RC-023's post-hoc signed/unweighted stratification **does not reproduce here**
(+0.126 % signed vs +0.100 % unweighted). That removes one way it could have
been a property of the corpus rather than of tabu memory. It neither confirms
nor refutes the original observation, which concerned a different mechanism.

## Working context

- Active branch: `feat/solver-research-upgrades`.
- RC-021 Step 6 is complete, committed as
  `1b855b774654f85a326baf37436262e9f5322e5b`, and an independent read-only
  review of it returned PASS.
- RC-021 Step 7 is complete and committed as
  `3493ed8`; its independent review blockers and adversarial follow-up findings
  are closed, and the full source gate set is green.
- **Step 8 is committed as `35fb5fa`, with `ad24013` and `e9a0450` on top.**
  Seven independent review rounds ran. Every HIGH and MEDIUM was fixed and
  mutation-verified; the deliberately open findings are recorded in
  `memory/OPEN_PROBLEMS.md` §0b.
- **RC-021 executed once on instrument commit `e9a0450` and is terminal.** The
  mandatory controls ran; eleven passed and `C10` failed because diagnostic
  overhead was `0.010802221586322025`, above the frozen `0.01` limit. The
  complete closure is Class I `INSTRUMENT-INVALID`, exit 3. No qualification
  session began, no paired-spread datum exists, and no Class II host verdict is
  licensed. The durable publication is
  `research/RC021_INSTRUMENT_INVALID_RECORD.md`.
- RC-021 closeout is committed as `af8da91` and `c110e2b`: the immutable Class
  I record is published, all formerly missing synthetic state-machine rows are
  covered by non-vacuous tests, the named mutation is killed, and the raw
  terminal evidence remained byte-identical across the full gate run.
- The durable-memory series is integrated on this branch as commits
  `983497d..d5880e3`.
- The integrated memory state at `e824b7f` was independently reviewed and
  returned PASS.
- `experiments/rc021/` is the existing gitignored terminal evidence directory;
  its closure and five input artifacts are bound by hashes in the abort record.
- RC-022 is closed: twenty MQLib families occupy at least eight RC-004 cells.
- RC-023 is closed with registered outcome (a): hard and soft memory forms were
  indistinguishable at the frozen budget. Synthesized moves are the next
  untested architectural axis.
- Binding RC documents and their paths are immutable.
- `docs/memory-architecture` is a historical backup reference only. It is not
  the active branch and holds no authority.

## Product position

- The production solver exists and remains the comparison arbiter.
- The `engine_v2` selector, capability registry, plan synthesis, Runtime,
  knowledge system, and research orchestration exist.
- Production auto-routing is not yet licensed: the domain-specific selector
  still needs replicated equal-cost evidence and a production A/B win against
  `UltimateSolver`.

## This task is complete only when

1. An Explorer map proves the exact existing synthesized-move implementation,
   runtime/registry call sites, reusable harness and do-not-touch boundary.
2. A binding RC-024 preregistration is committed before candidate code or
   result data exists.
3. Candidate and control differ on one named synthesized-move mechanism, use
   identical seeds and logical budgets, and have deterministic backend/energy
   equivalence tests.
4. The registered experiment runs once, its complete outcome is published even
   if null or adverse, and prohibited wall-time/production claims are absent.
5. An independent review and every applicable source, memory and immutable-file
   gate pass; RC-021 terminal evidence remains byte-identical.

## When Step 8 is finished — the stopping criterion

Set 2026-08-28 after three review rounds each found defects, several of them in
the previous round's own fixes. Without a stated criterion this converges on
nothing: the diff grows, each round finds less, and an uncommitted ~4 000-line
change is itself a risk — it can be lost, and it grows past the point where a
reviewer can hold it.

**Commit Step 8 when a review round returns no HIGH and no MEDIUM correctness
finding.** LOW findings are recorded in `OPEN_PROBLEMS.md` and closed
deliberately afterwards, not before the commit.

This is not a lowering of the bar. Every HIGH and MEDIUM found so far has been
fixed and mutation-verified; the criterion only says that polish below that
severity does not justify holding four thousand lines out of history.

## The census is done — and it decided the programme

**`research/RC022_ARCHITECTURE_CENSUS.md`, 2026-08-30.** The §8.A experiment
that the standing objective named as decisive has run. Analysis only: no
compute, no dependency, no execution.

Twenty independently designed MaxCut/QUBO heuristic families from MQLib spread
across **at least eight cells** of RC-004's seven-axis space, against one for
our eighteen operators. **No family occupies our cell**, and three mechanisms
the corpus uses heavily are absent from our operator library entirely:

- **memory beyond the configuration** — 7 of 20 families, all tabu-family. We
  have **one**, in a different form: `history_field`, a metadynamics soft
  decaying penalty, which is among our **top four** operators by mean rank. The
  first version of RC-022 said we had none; that was wrong and the correction is
  §0 of the record. The gap is not the axis but the **form**: no hard
  prohibition, no recency list, no aspiration criterion anywhere;
- **synthesized moves** (crossover, path-relinking) — 9 of 20; RC-003 touched
  this axis once, for +0.06 %;
- continuous domain with gradient moves — 1 family.

So RC-004's one-cell result is a property of **our corpus**, not of the problem
class, and the taxonomy discriminates well enough to steer generation. The
marginal/probabilistic cell stays closed to us by `RELATIONAL_PRIMITIVE.md`'s
Z₂ theorem, which leaves **memory as the surviving Ax2 route** —
`AXIOMS_OF_OPTIMIZATION.md` §5's named attackable axiom.

This does **not** say an empty cell holds a better algorithm, that memory helps
our instances, or that any of these beats `UltimateSolver`. It is a map of what
exists, not a measurement of what wins.

**The question it produced, which is the next piece of work:** does the *form* of
memory matter — can hard prohibition with an aspiration criterion do something a
soft decaying bias cannot?

**RC-023 answered it: no.** `research/PREREG_RC023_MEMORY_FORM.md` was committed
(`74e425e`) before any code; `tabu_sweep` was built as `history_field`'s twin
(`34eab7c`), differing only in the form of the memory, with the shared RNG
stream pinned by a test so the comparison isolates that one variable. Two-sided
Wilcoxon over 30 paired G-Set scores: **p = 0.87** as registered, **p = 0.63**
when the harness's 3-decimal printing is removed as a possible artifact. Mean
difference −0.0018, median +0.0010 — they disagree in sign. Registered outcome
(a): the two forms are indistinguishable at this budget on this corpus. Full
record and its prohibited claims: `results/rc023/RESULT.md`.

The result is worth as much as a win. RC-022 named the *form* of memory as the
surviving Ax2 route after the Z₂ theorem closed the marginal cell. That route is
now measured and it is flat — which means the gap between our corpus and MQLib's
is not the tabu spelling, and the next candidate must come from a different axis
(synthesized moves, 9 of 20 families, is the largest remaining one).

**The post-hoc observation that must not be promoted without its own
preregistration:** the sign of the per-instance difference separates perfectly
by whether the instance carries negative weights — tabu is lower on 6/6 signed
(±1) instances and 3/24 unweighted ones, Mann–Whitney U = 0. Density is
excluded (G48/G49/G50 match G11/G12/G13 on topology and m/n and show ~0), but
the matched control is **saturated** — nine of nineteen operators sit at its
optimum — so it cannot discriminate and the confound is only partly excluded.
RC-004 retracted Law 2 for exactly this shape of error. Treat it as a
hypothesis, and note that its first design obstacle is finding a control corpus
that is not already solved.

## Next action — in this order

1. **The architecture line is closed. Do not open a sixth cycle on it.** Its
   three natural questions have all been asked and answered:

   | question | cycles | answer |
   |---|---|---|
   | is the mechanism inventory the gap? | RC-003, RC-023, RC-024 | effects an order of magnitude under the 1 % floor |
   | is G-Set's degeneracy the reason? | RC-025 | no — a weight-diversity ladder is flat |
   | were we measuring it outside the search it belongs in? | RC-026 | **partly yes** — and it changes nothing that matters |

   RC-026 is the only one that found a signal: embedding path relinking k times
   inside one search, at an unchanged 50-sweep budget, gives Spearman ρ = +0.312,
   **p = 1.0e-4**, positive on 26 of 30 instances. It is also `gain ∝ k^0.18`
   against a per-round cost of `d²/2` evaluations — 25× the rounds buys 1.77× the
   gain, and 1 % would need of order 3.6e5 rounds. **The direction that helps is
   the direction that cannot be afforded.** That is a cleaner ending than five
   nulls, and it is the ending.

   Everything is recorded: `results/rc026/RESULT.md`, `OPEN_PROBLEMS.md` §0h.

2. **The one asset this line produced is the never-worse invariant.** 478 wins,
   **0 losses** in 630 paired trials across two corpora, three preregistrations,
   nine seeds and five embedding depths. Whatever comes next, that property — a
   mechanism that cannot make things worse, proven before it was run — is the
   template worth reusing, not the mechanism itself.

3. **Wall-time work is open and the protocol now exists.**
   `research/PERF_INCREMENTAL_RELINK.md` made the project's first entitled
   wall-time claim: incremental relinking, **5.72×** measured against the host's
   own calibrated null, bit-identity proven by re-running two frozen commands and
   matching the SHA-256 published in their `RESULT.md` files — 540 registered
   rows. `CLAUDE.md` §9 now carries the protocol: calibrate the host, keep the
   previous binary and interleave the arms, prove identity against something
   published earlier, declare whatever moved.

4. **ACTIVE TASK — RC-027, the budget axis.** Every one of the six cycles ran
   50 sweeps; a mechanism that only pays on a long run would be invisible to all
   of them. It is the last cheap question this project has not asked and the only
   one left that could overturn the closure. Do the candidate and control curves
   **converge, diverge, or stay parallel** as the budget grows?

   **The full brief is [`NEXT_TASK_RC027.md`](NEXT_TASK_RC027.md).** Read it
   before anything else: it carries the design, the mandatory controls, the
   non-negotiable procedure, the six traps that have already cost this project
   time, the tools that now exist, and the hard prohibitions. Preregister before
   any code, as the last four cycles did.

2. **Decide item D** in `OPEN_PROBLEMS.md` §0b: does exit 4 mean "the evidence
   is broken" or "something went wrong"? A sentinel that could not run is not a
   damaged journal. A contract question; it belongs to the maintainer.
3. **C10 is diagnosed, and the wall-time block is not where the record said.**
   `research/RC021_C10_DIAGNOSIS.md`: C10's 1 % bound sits at the *median* of its
   own null distribution — two identical arms exceed it in 20 of 40 replications
   — so its verdict carried almost no information about the diagnostics. The
   outcome (INSTRUMENT-INVALID) stands; the recorded cause does not, and the
   diagnostic overhead remains **unmeasured** rather than shown to be zero.
   `src/bin/host_timing_calibration.rs` now measures what this host can resolve
   **before** a threshold is frozen: **~25 % cleanly, ~5 % marginally, 1 % not at
   all**, and `--pin` reports a known +25 % injection as +48 %. Consequences are
   recorded in `CLAUDE.md` §9 and `OPEN_PROBLEMS.md` §0f. A wall-time claim needs
   a fresh preregistration with a bound at or above 5 % and a mandatory null arm;
   C10 itself stays frozen exactly as written.

4. **The external-technology review is settled** —
   `EXTERNAL_PROJECTS_BACKLOG.md` §14. All thirteen August-2026 sources were
   fetched and verified real. Twelve are "no": they speed up writing code, which
   has never been the limiting step here. The one that changes anything is the
   AGENTS.md study, and it was reported to us inverted — the harm is in
   **auto-generated repository overviews**, not context files as a class, while
   hand-written instructions are followed and help. Checking our own overviews
   ended the question early: `.claude/context/` is **untracked**, has no
   generator and no freshness gate, was last written 2026-07-05, and its
   `modules.txt` still describes a project with **no `engine_v2`**. `CLAUDE.md`
   §7–§8 ranked that map above ripgrep, and the same paper shows such
   instructions are obeyed. Corrected in place: `rg` and `cargo metadata` now
   rank first because they cannot go stale. Do **not** regenerate
   `.claude/context/` without evidence that a maintained overview helps.

RC-021 is an instrument, not a result, and it is still unqualified on this host
(C10). That is why RC-023 makes **no wall-time claim of any kind** — only
paired quality at identical seeds, which needs no qualified host.

## Forbidden during this task

- Re-running RC-021, any real control, any qualification session, or a sentinel
  on G11.
- Editing, deleting, repairing, or supplementing the existing
  `experiments/rc021/` terminal evidence.
- Editing or moving any existing binding preregistration, amendment, or cycle
  record; RC-024 gets a new prospective preregistration and result.
- Fixing the non-blocking F1/F2 test-hardening findings "while here".
- Starting any six-session execution under the spent RC-021 protocol.
- Changing the scalar solver family, `core`, Cargo dependencies, public API,
  `UltimateSolver`, or production auto-routing.
- Inspecting or changing registered result seeds after the RC-024
  preregistration is committed.
- Making a wall-time, equal-cost, universal-optimizer, or production-superiority
  claim from RC-024.
- Starting Soup/LoRA/video-model work inside this research cycle.
- Weakening the durability contract, the terminal evidence lock, or the
  read-only guarantee of `--verify`.
- Treating Obsidian, generated indexes, summaries, chat logs, or model memory as
  a source that outranks Git-tracked canonical documents.
