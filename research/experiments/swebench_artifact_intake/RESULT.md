# SWE-bench public artifact intake — bounded result (2026-09-29)

**Decision: NOT QUALIFIED for the proposed false-`DONE` corpus.** This is a
source-admission result, not a verdict on agent reliability or a new product.
The frozen selection and prospective S3 amendment are in
[`HYPODIVE_TRIAGE.md`](../../HYPODIVE_TRIAGE.md#swe-bench-artifact-source-intake-selection-freeze--2026-09-29).
The original repository-asset transport failed before instance inspection;
this result concerns only the amended anonymous-S3 transport.

## What was checked

The two frozen 2026 SWE-bench Verified entries yielded the same three
hash-ranked task IDs. [`replay.py`](replay.py) downloads the six pinned
trajectories and, where `metadata.yaml` advertises logs, the four pinned log
artifacts per task. It fails on any changed SHA-256 or mismatched task ID and
regenerates [`evidence.json`](evidence.json). Run from repository root:

```bash
python3 research/experiments/swebench_artifact_intake/replay.py > /tmp/swebench-intake-evidence.json
cmp /tmp/swebench-intake-evidence.json research/experiments/swebench_artifact_intake/evidence.json
```

The pinned registry commit is `40f164d5b8f1d249bf95a6df8b74b577fd8e519d`.
The raw 2026 directory listing and two metadata hashes are recorded in the
selection freeze. The S3 XML listing SHA-256 values used after the amendment
were `0246259a25da122721290a23a65ce6822fa2a4da459703017920e7471cbb7ad1`
(GPT-5.2 Codex, 500 `.traj.json` task IDs) and
`4fcceaa7e9dc690a6d266324d90b63e7b659d9a612bb74cee0f64fe5c32b9bc7`
(Claude 4.5 Haiku, 500 `.traj.json` task IDs). The Claude listing also contains
`sb-cli-reports/` and non-trajectory files, excluded by the amendment's
"trajectory instance ID" rule. The three SHA-ranked IDs were
`sympy__sympy-13798`, `pytest-dev__pytest-5631`, and `sympy__sympy-17318`.
No outcome was used to pick them.

| Frozen field | GPT cases | Claude cases | Interpretation |
|---|---:|---:|---|
| C: explicit **final** success/completion claim | 0/3 | 0/3 | Manual classification of the original trajectories. `Submitted` and a patch are not success claims. Claude has intermediate comments about local tests, but its last recorded assistant text announces patch submission, not task success. |
| P: nonempty submitted patch equals trajectory exit payload | 3/3 | 3/3 | Task patch is present. |
| O: official verdict **and regrader-sufficient test output** | unverified | unavailable by advertised metadata | The three GPT cases have report and nonempty test-output artifacts; regrader sufficiency was not checked. Claude metadata has `assets.logs: null`. |
| I: task ID and source entry join | 3/3 | 3/3 | ID is consistent in trajectory and URL. |
| L: public fetch / stated reuse terms | 3/3 / unknown | 3/3 / unknown | Anonymous HTTP access works; reuse terms were not established. |

**Admission: 0/6 have all C/P/O/I, below the frozen 4/6 threshold.** O was not
promoted on mere file presence; full O remains unverified for the GPT cases.
The source is unfit for this particular false-final-claim experiment. It is not
evidence that false completion claims never occur in other workflows. An
official report is an evaluation artifact, not fresh execution by this audit;
the [official SWE-bench experiment instructions](https://github.com/swe-bench/experiments)
describe `swebench submit verify` as regrading recorded test output.

## Separate artifact-lineage observation

In **all three GPT cases**, `info.submission` exactly matches the trajectory's
`exit` patch, but differs from `logs/<task>/patch.diff`. This is more than
header formatting: each pair has changed code lines exclusive to each patch.

| Task | Lines only in trajectory submission | Lines only in log patch | Log report `resolved` |
|---|---:|---:|---|
| `sympy__sympy-13798` | 15 | 15 | false |
| `pytest-dev__pytest-5631` | 10 | 1 | true |
| `sympy__sympy-17318` | 7 | 2 | false |

For the latter two, `job-output.json`'s `diff_before` equals `patch.diff`
byte-for-byte. For the first, the Git index-header abbreviation differs, while
the changed code lines are the same. In all three, `diff_before == diff_after`
and `diff_changed == false` within the job output. This corroborates internal
consistency of the **log-side** patch, but does not establish why it diverges
from the trajectory submission. Possibilities include separate artifact
generation paths or an association error. We did **not** independently rerun
SWE-bench, prove which patch was actually applied, or inspect the entire 500-case
entry. Do not infer an incorrect leaderboard score or general prevalence from
these three selected cases.

The [mini-SWE-agent output specification](https://mini-swe-agent.com/latest/usage/output_files/)
calls `info.submission` the agent's final output/patch. That gives a precise
candidate invariant to test: a published verdict should be traceable to the
same patch bytes or a documented transformation. A simple hash/linkage check
is the first baseline; no new AI verifier is justified by this finding.

**Protocol deviation disclosed:** after seeing Claude's `assets.logs: null`,
an exploratory query of a *guessed* S3 `logs/` prefix showed objects, but their
contents were not fetched or used to promote O. The frozen metadata-following
rule remains in force. The initial S3-ID parsing script briefly split on `.`;
it was corrected before any case content was read, yielding the IDs above.

**Next cheap falsifier:** ask whether the entry's `all_preds.jsonl` and public
submission/evaluation pipeline explicitly document a patch transformation or
different provenance for trajectory and log assets. If yes, the apparent
lineage problem may be explained. If not, reproduce the mismatch on a clean
copy with a direct patch-hash check before framing it as a benchmark issue.
No implementation or product claim follows from this intake.
