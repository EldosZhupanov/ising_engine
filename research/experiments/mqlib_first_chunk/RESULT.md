# MQ-FIRST-CHUNK-001 — first verified output delivery

Status: **CLOSED; C1, C2 and CONTROL observed on exposed inputs**. This is an
informed mechanism diagnostic after [MQ-QUALITY-002](../mqlib_coarse_quality/closure/RESULT.md),
not an independent test of solver quality or speed.

The [protocol](protocol.md) was committed at `922a3a0` before the instrument
(`7ef649a`) and the one run (`0a496f1`). The unchanged Rust worker and production
solver were used. [Instrument review](instrument_review.json),
[raw audit](run001/audit.json) and [independent raw review](run001/review.json)
all passed. All 12 cells were valid, with 45 independently verified witness
messages, none late. The run used 123.307 s of its 180 s cap. Exact model,
source and binary hashes, CPU affinity, raw output, all event times and complete
individual states are retained under [run001](run001/README.md).

| Input | Arm | First verified `inc` delivery, seconds, seeds 61101–61103 |
|---|---|---|
| q12: 512 variables, sparse p=1/16 | Ultimate | 0.587, 0.547, 0.531 |
| q12 | `v2_default` | 0.089, 0.050, 0.036 |
| q18: 512 variables, dense p=1/2 | Ultimate | **3.004, 3.061, 2.962** |
| q18 | `v2_default` | 0.487, 0.436, 0.436 |

For Ultimate/q18, readiness arrived by 0.075 s on all three runs. The first
verified output arrived after 2 s but before 10 s on all three. Thus the
registered C1/C2 pattern holds, and the v2 delivery-path CONTROL holds. In the
prior two-second campaign, all 60 Ultimate runs on six dense 512-variable inputs
had no timely improvement. The current q18 timings give a concrete mechanism
consistent with those outcomes: this worker emits an `inc` only after a whole
`UltimateSolver::solve()` call completes. On q18, this fixed configuration did
not complete and deliver its first candidate inside the two-second window.

The time from readiness to first output includes solver construction, `solve`,
energy recomputation, serialization, pipe delivery and independent validation.
It is **not an internal kernel benchmark**. Two exposed instances and three
reused seeds cannot establish a density law or general performance ranking.
The result does not rescue H2 from MQ-QUALITY-002: no material complementary
winner pattern or learned-selector benefit was demonstrated.

**Next step:** design a budget-aware output experiment for Ultimate, preserving
its public `solve()` entry point and the scalar/MSC boundary. First map whether
an additive checkpoint/callback or shorter deterministic chunks can emit valid
incumbents before 2 s. Preregister a matched-cost comparison of baseline and
candidate on independent data, with independent energy checking and explicit
quality-at-budget endpoints. This record authorizes no production change or
superiority claim.

Read-only audit reproduction without overwriting `audit.json`:

```bash
python3 - <<'PY_AUDIT'
import importlib.util
import json
from pathlib import Path
module = Path('research/experiments/mqlib_first_chunk/audit.py').resolve()
spec = importlib.util.spec_from_file_location('first_chunk_audit', module)
auditor = importlib.util.module_from_spec(spec)
spec.loader.exec_module(auditor)
folder = module.parent / 'run001'
assert auditor.audit(folder) == json.loads((folder / 'audit.json').read_text())
print('PASS: first-witness raw audit reproduced')
PY_AUDIT
```
