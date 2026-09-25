# EXP-007W run artifacts

Protocol: `../../protocol.md`. Inputs: `../../instances.json`. Structural probes: `structural.jsonl`; timed cells: `raw.jsonl`; independent summary: `analysis.json`; environment: `metadata.json`.

From repository root, run `bash research/experiments/exp007_weighted_mis/commands.sh <new-output-directory>` to replay. The run uses wall-clock deadlines, so timing and best-found weights can vary with machine load; independent feasibility and energy checks must always pass.

The two relative paths above were corrected after the run; the frozen runner's
generated README used `../` instead of `../../`. No raw, structural, metadata
or analysis value was changed by this documentation correction.
