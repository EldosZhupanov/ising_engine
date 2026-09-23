# LAYA-001 frozen capability handoff

Date: 2026-09-23. This prospective clarification supplements [RESULT.md](RESULT.md)
without changing that historical report or the binding [protocol](PROTOCOL.md).

## 1. Objective

Mode B: implement and exercise semantic probabilities → verified discrete
reconciliation → residual conflict graph inspection. Success means a working,
reproducible adapter with verified energies and reduced optima; this exploratory
pilot cannot establish novelty, production accuracy, or a speed advantage.

## 2. Strongest Claims

- C1, capability, supported: the installed English Laya checkpoint supplies
  probabilities that the isolated adapter reconciles against exclusion and
  implication constraints. All 24 tiny instances pass exhaustive energy checks.
- C2, descriptive, supported only on this corpus: raw accuracy is 162/192,
  greedy repair 175/192, exact/Ultimate/reduced exact 181/192. Reconciliation
  corrects 19 raw errors and damages zero correct bits; 26 violations become zero.
- C3, descriptive, supported: existing full presolve fixes all 192 variables.
  There are no residual components to search. This provides no evidence of
  annealing/CD005 benefit or runtime acceleration.

## 3. Canonical Implementation

[run_pilot.py](run_pilot.py) creates the frozen corpus and invokes installed Laya;
[pipeline.py](pipeline.py) builds penalties and verifies returned results;
[laya_bridge.rs](../examples/laya_bridge.rs) calls existing production presolve,
decomposition, and UltimateSolver; [analyze.py](analyze.py) recomputes summaries.
No production API or solver implementation was changed for this capability.

## 4. Architecture Decisions

Keep model inference and JSON in research orchestration. Reuse existing solver
and presolve through an example instead of adding Laya dependencies to production.
The exhaustive oracle is limited to small cases. Ultimate already invokes
presolve, so there is no claimed comparison with an unreduced Ultimate baseline.
Map and implementation plan: [intake](HYPODIVE_BUILDER_INTAKE.md).

## 5. Invariants

- Direct likelihood plus penalties equals QUBO energy for all 256 states/group.
- Returned states are strictly binary and energies finite, with tolerance 1e-8.
- Exact and lifted reduced optima have equal energy; tied assignments may differ.
- M = 1 + sum(abs(unary coefficients)) makes constraint violations dominated
  by an available feasible assignment. Semantic correctness is not guaranteed.
- Bridge stdout/stderr and failures are persisted before parsing or validation.

Python tampering tests and Rust exhaustive/reduction tests enforce these boundaries.

## 6. Test Evidence

[verification.json](verification.json) records the exact pre-run commands:
5 Python tests, 4 Rust adapter tests, and 24 production regression tests passed;
scoped check/build/clippy/rustfmt and diff checks passed. No root src edits were
made, so these were scoped research gates, not a claim to rerun unrelated gates.
Independent read-only reviewer repeated the Python and Rust adapter tests,
verified all 77 result hashes and all 24 provenance chains, and recomputed the
complete summary using source commit 96b2ace: PASS, no HIGH/MEDIUM findings.

Full-worktree memory validation has a pre-existing failure from 29 uncatalogued,
untracked Markdown files under fundamental_ai and rc027. The intended-source
snapshot passed `GIT_OPTIONAL_LOCKS=0 bash
/tmp/laya-001-memory-snapshot/scripts/check_memory_docs.sh 265555b`:
251 Markdown files, 71 immutable files. Unrelated material is preserved.

## 7. Empirical Evidence

Raw corpus, prompts, probabilities, solver requests/responses, environment and
completion marker: [results/run001](results/run001/).
Aggregate: [summary.json](results/run001/summary.json).
Integrity: [sha256.json](results/run001/sha256.json), 77 hashed files.

```bash
python3 research/laya_semantic/analyze.py research/laya_semantic/results/run001
```

This analysis needs no model inference. The pilot ran once from clean detached
source 96b2ace. Seeds and generator are frozen in the protocol and corpus.
All arms use the same 24 groups; no inferential or generalization claim is made.
Observed inference time: 130.0278 s, CPU, two threads; not a speed comparison.

## 8. Assumptions

The synthetic truth is feasible by construction. Eight literal manufacturing
status messages per group use shared templates. Laya sees each message, not
the graph or ground truth. The experiment tests one English checkpoint with
SDK laya 0.3.7; it does not test multilingual or typed variants.
Snapshot and file hashes are in [environment.json](results/run001/environment.json);
dependency versions are in [runtime_versions.json](results/run001/runtime_versions.json).

## 9. Known Weaknesses

The corpus is small and presolve-saturated. There is no real-domain holdout,
calibration study, training, equal-cost comparison, or hard-search result.
The Brier score is descriptive; it alone does not establish calibration.

Prospective corrections to RESULT.md: the run identifies the English snapshot,
not a verified model called "Laya-1B". CD005 was configured but its contribution
was not isolated, and full presolve explains the result without residual search.
Zero residual enumerated states does not mean zero computational work.
The observed 0.45997 s total bridge time includes process starts and all three
solver/oracle paths; it is not isolated UltimateSolver timing.
The concurrent RESULT.md publication describes this same run, not replication.

## 10. Simplest Plausible Alternative

Greedy feasible repair already reaches 175/192. Existing full presolve alone
explains the stronger 181/192 result. No new solver mechanism is needed to
explain this pilot. Structured inference itself is established methodology.

## 11. Suggested Kill-Tests

Next, separately preregister a real annotated corpus with imperfect rules and
negation/ambiguity: a failure to improve over greedy at controlled damage would
refute practical usefulness. For graph reduction, first choose instances with
nonempty residual components; then compare exact optima before/after reduction.
Persistent complete elimination would show the corpus cannot test search benefit.
Any mismatched lifted optimum would refute reduction correctness for that case.
These are suggestions, not authorization to run another experiment.

## 12. Freeze Point

- Branch: feat/solver-research-upgrades; clean evaluated detached commit: 96b2ace.
- Protocol freeze: 26d4c0a; results freeze: f28d12a.
- Claims: C1–C3 above. Source and tests: canonical paths in sections 3 and 6.
- Config/seeds: protocol, corpus.json, environment.json; hashes: sha256.json.
- Replay: analyze command above; model rerun requires the recorded local weights.
- Concurrent SK/QOBLIB commits after the evaluated source are not pilot evidence.
- Remaining: a separately scoped real-data/residual-graph study; no production
  integration, record-breaking experiment, or quantum dynamics engine is implied.
