# MQ-SCREEN-001 — no final-quality separation

Date: 2026-09-27. Status: **INCONCLUSIVE for H1**.

[Preregistration](protocol.md) commit `2d6d08d`; instrument `b0cf802`;
[retained data](run001/) commit `2fe2cb1`. No outcome-based tuning or reruns.

## Result

Eight fresh synthetic weighted QUBOs, ten seeds per instance, three configured
wrappers, two seconds per cell on CPU0 with one worker thread: **240/240 valid
cells**. In every instance/seed pair all three arms return the same final energy:

| Instance | Ultimate | v2 default | MQLib MERZ2002ONEOPT |
|---|---:|---:|---:|
| q00, n64, p0.1 | -161 | -161 | -161 |
| q01, n64, p0.1 | -181 | -181 | -181 |
| q02, n64, p0.7 | -400 | -400 | -400 |
| q03, n64, p0.7 | -501 | -501 | -501 |
| q04, n128, p0.1 | -487 | -487 | -487 |
| q05, n128, p0.1 | -408 | -408 | -408 |
| q06, n128, p0.7 | -1398 | -1398 | -1398 |
| q07, n128, p0.7 | -1307 | -1307 | -1307 |

Each entry is identical over all ten seeds (zero within-arm final-energy spread).
Every pairwise comparison is **0 wins / 80 ties / 0 losses**. No fallback-only
cells. Primary mean normalized Ultimate advantage = 0, exact sign-flip p = 1,
instance-bootstrap 95% interval [0,0]. The frozen H1 criterion is not met.
The degenerate empirical interval does not prove population equivalence.
Post-hoc oracle improvement over the best fixed arm is also zero; this is not
an evaluated learned selector.

These are feasible binary states with independently verified objective values,
not certificates of global optimality. No known optimum, optimum success rate,
TTS99, record, SOTA or generalization claim is available.

## Verification and limitations

[Independent executable audit](independent_audit.py) checks **459 complete
main-run witnesses**, 80 v2 configuration events, 223 committed source hashes,
10 additional instrument/binary hashes, dataset hashes, exact raw-polynomial
energies, receipt deadlines, endpoint reconstruction and all summaries.
[Audit output](run001/independent_audit.json): PASS; no late complete witnesses.
Raw SHA256: `513d9b54c57e69117a0c38346c8e9e100e6f9901906dd45f60430c52be03c135`.

The preceding [smoke](smoke001/) had 24 valid cells and 24 independently checked
witnesses, with no fallback-only cases. Design, instrument and smoke received
independent read-only reviews before main execution. The main auditor was
written by that reviewer and executed by the primary agent; the reviewer's
next turn encountered a service usage limit. A second independent reviewer
(`/root/comparison_explorer`) then personally reran the read-only audit and
reviewed protocol/result/current-state alignment: **PASS**, no blockers.
The [verification record](verification.json) preserves both review events.

[Verification logs](verification.json) retain the initial compile failure and
subsequent fixes as well as passing checks: two Rust representation/parser tests,
ten Python deadline/validation/statistics tests, strict example Clippy. Formatting,
C++ strict bridge compilation and memory/diff checks also passed. The production
solver families, public APIs and Cargo dependencies are unchanged. The full
production test suite was not rerun for this isolated research instrument.

This compares restarted configured Ultimate/v2 wrappers against one native MQLib
heuristic. It does not compare tuned maxima, MQLib's portfolio/hyperheuristic,
learned engine_v2 or continuous Ultimate search with mid-solve callbacks. Budget
includes conversion, launch, search and delivery; only complete lines received
before cutoff qualify. Host scheduling, wrapper overhead and setup are part of
this estimand. No timing speedup is claimed from different delivery latencies.

## Reproduction

Use a clean checkout containing this result (the instrument source hashes must
match the recorded commit), Rust/Cargo, Python3, g++, make, git and taskset. Build
artifacts are optional cached dependencies, not required repository binaries.

```bash
bash scripts/setup_mqlib.sh
cargo test --release -p research --example mqlib_compare
python3 -m unittest discover -s research/experiments/mqlib_screen -p 'test_*.py' -v
python3 research/experiments/mqlib_screen/build.py
python3 research/experiments/mqlib_screen/campaign.py --smoke --output /tmp/mq-smoke-new
# Require independent smoke PASS before the next command.
python3 research/experiments/mqlib_screen/campaign.py --output /tmp/mq-main-new
python3 research/experiments/mqlib_screen/independent_audit.py /tmp/mq-main-new
```

Output directories must not already exist. Rerun timing/results may vary; retain
new records separately. The auditor requires the corresponding local build
manifest and binaries: comparing old retained binary hashes to a different
compiler's rebuild is not an independent reproduction of those binary hashes.
Metadata records commands, compiler, CPU, environment and linked upstream build.
Do not overwrite the frozen run to obtain a more favorable comparison.

## Decision and next task

This corpus/budget does not discriminate final quality; it cannot support
algorithm-selection or complementarity claims. Keep all eight cases as validation
fixtures, not as unseen evidence for a tuned successor. Do not enlarge the same
campaign after observing ties. Next: prospectively qualify a separate difficulty
corpus with strong fixed baselines, include an independent holdout, and define
cost curves and an endpoint that can distinguish methods before training a
selector. Do not add operators or start record hunts on the basis of this null.
