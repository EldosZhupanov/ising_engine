# Post-hoc lineage diagnosis — 2026-09-29

This is a **new exploratory record**, not a correction to the immutable
[`RESULT.md`](RESULT.md) or a new preregistered sample. It investigates the
three patch mismatches noticed there. No model-quality or leaderboard verdict
is changed by this analysis.

## Reproduction and observation

[`lineage_replay.py`](lineage_replay.py) compares the original frozen
`20260219_mini-v2.0.0_gpt-5-2-codex` entry with the separately registered
`20260217_mini-v2.0.0_gpt-5-2-high` entry on the **same three preselected task
IDs**. It checks SHA-256 of source listings and case bytes, then emits
[`lineage_evidence.json`](lineage_evidence.json):

```bash
python3 research/experiments/swebench_artifact_intake/lineage_replay.py > /tmp/swebench-lineage.json
cmp /tmp/swebench-lineage.json research/experiments/swebench_artifact_intake/lineage_evidence.json
```

The two public S3 trajectory listings each contain 500 `.traj.json` keys.
**All 500 relative keys have equal S3 ETag and size** across the two entries.
This is listing-level evidence, not 500 independently downloaded SHA-256
comparisons. On the three frozen IDs, the trajectory JSON files are
**byte-identical**, verified by SHA-256. Each identifies its model as
`openai/gpt-5.2-2025-12-11`, while the [Codex registry metadata](https://github.com/SWE-bench/experiments/blob/40f164d5b8f1d249bf95a6df8b74b577fd8e519d/evaluation/verified/20260219_mini-v2.0.0_gpt-5-2-codex/metadata.yaml)
labels that entry `gpt-5-2-codex`; the [high registry metadata](https://github.com/SWE-bench/experiments/blob/40f164d5b8f1d249bf95a6df8b74b577fd8e519d/evaluation/verified/20260217_mini-v2.0.0_gpt-5-2-high/metadata.yaml)
labels the other `gpt-5-2`.

For each of those three IDs, the trajectory's `info.submission` is exactly
the **high entry's** `logs/<id>/patch.diff` and differs from the **Codex
entry's** `logs/<id>/patch.diff` in changed code lines. Thus the previously
unexplained 3/3 mismatch has a concrete source association: the Codex
trajectory path exposes the same three trajectory bytes as the high entry,
whose log patches they match. The 500 ETag/size matches make a copied or
shared trajectory set the strongest explanation for the full directory, but
we have verified byte identity directly only for three files. We cannot infer
whether the reuse was accidental, when it happened, or whether any evaluation
score is wrong.

The [mini-SWE-agent batch runner](https://mini-swe-agent.com/latest/reference/run/swebench/)
normally maps `info.submission` to `model_patch` in its predictions, and the
[SWE-bench evaluation harness](https://github.com/SWE-bench/SWE-bench/blob/main/swebench/harness/run_evaluation.py)
writes `patch.diff` from a supplied `model_patch`. Those are useful pipeline
expectations, not a proof of the exact version or assembly procedure used for
these February submissions. The [SWE-bench experiment instructions](https://github.com/SWE-bench/experiments)
also say traces should reflect the run behind the prediction, including any
selection of multiple attempts. These entries list one attempt in metadata;
the evidence still cannot establish intent.

**Decision:** the three cases should not be used to evaluate claim-based agent
reliability or to audit Codex behavior from their trajectories. Their official
log-side reports can still be assessed separately, but this record does not
regrade them. A deterministic artifact-linkage rule would catch this observed
problem; no AI component is needed for it. This is a narrow data-provenance
finding, not the owner's sought universal product or mathematical breakthrough.
