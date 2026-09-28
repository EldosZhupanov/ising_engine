# MQ-QUALITY-002 — two-second fixed-wrapper qualification

Status: **CLOSED**. H1 **SUPPORTED_ON_QUALIFICATION**; H2 **NOT_ESTABLISHED**.
This is a complete, audited result on one exposed synthetic qualification family,
not an unseen evaluation or a claim of general solver superiority.

## Chronology and scope

The preceding [MQ-DIFFICULTY-001](../../mqlib_difficulty_qualification/closure/RESULT.md)
failed its 250 ms admission and never ran its main. The [prospective revision](../protocol.md)
was committed at `4d0b3f4` with that failure disclosed. Its primary two-second
hypotheses, all 24 input hashes, four fixed wrappers and previously unused seeds
were retained. The new coordinator and auditor were committed at `6d0f5a5` and
independently reviewed before observations. Unchanged native workers came from
`f76991d`; production and Cargo files were not changed. The [manifest](../manifest.json)
contains the precise settings and input provenance.

The [admission](../admission001/README.md) ran once on `0750a1a`: 20/20 valid
cells, each with one timely independently checked witness; zero late witnesses;
maximum kill-request overshoot 2.102 ms. Its [raw audit](../admission001/audit.json)
and [review](../admission001/review.json) passed before the main run. Admission
elapsed 42.710 s. This is a liveness/correctness gate, not a short-time precision
calibration or proof of reliability on the larger inputs.

The [main](../main001/README.md) then ran once with 24 instances × 10 fixed seed
labels × 4 wrappers = **960 valid cells**, each with a two-second delivered and
validated-result budget. The parent measured preparation through output checking.
The [raw audit](../main001/audit.json) independently recalculated all energies,
deadlines, source/binary hashes, the full schedule and the statistics; its
[independent read-only review](../main001/review.json) passed. There were **12,500
verified witnesses**; two were late and excluded. Main elapsed 1,933.361 s, so
admission plus main used 1,976.072 s of the registered 3,000 s cap. Full per-cell
traces and all ten energies per arm/instance are retained under `main001/`.

## Registered outcomes

| Endpoint | Result | Interpretation |
|---|---:|---|
| H1: at least 8 instances with best-to-worst seed-mean spread ≥ 0.001 L | **21/24** | The four fixed wrappers produce distinguishable results on this family. A weak arm can cause this. |
| H2: at least two arms each with ≥ 4 unique wins over every other arm by ≥ 0.001 L | **0 unique wins for every arm** | The registered complementary-winner pattern is absent. |
| Largest best-to-second-best normalized spread | **0.000678** | Below the 0.001 margin on all 24 instances. |
| Fallback-only runs | Ultimate 60/240; other arms 0/240 each | All 60 Ultimate fallbacks are on the six dense 512-variable instances. |

Here `L = Σ|hᵢ| + Σ|Jᵢⱼ|`, excluding the constant offset. Lower energy is better.
The [analysis](../main001/analysis.json) gives all per-instance summaries, paired
outcomes and preregistered descriptive instance-bootstrap intervals. For example,
the mean normalized contrast `mqlib_merz_vs_mqlib_mst2` is −0.000975 with
descriptive 95% interval [−0.001259, −0.000690]; its sign means MST2's mean was
lower in that contrast. These intervals describe resampling the 24 related
synthetic instances, **not** independent external replication. The maximum
best-to-second-best spread stays below the registered materiality margin.

## What the result does and does not support

The 2 s quality gate sees a real difference between some of these fixed wrappers,
particularly when Ultimate produces no timely improvement on dense 512-variable
inputs. The data do **not** establish that `engine_v2` selects better operators,
that a learned selector would help, that MST2 or any arm is generally strongest,
or that any implementation is faster in pure search. There is no held-out corpus,
known optimum, time-to-target endpoint, optimizer replication or S3/X3 admission.
The transferred seeds were not fresh independent replications. No tuning,
rescue run, extra arm or outcome-filtered subset was introduced.

**Next narrow task:** inspect the frozen worker and retained traces to determine
why Ultimate produced no timely improvement on those 60 dense cases. Treat
initialization, chunk duration and output cadence as hypotheses until the traces
or a separately registered diagnostic resolve them. Any new comparison needs a
new protocol and independent data; this closed cycle does not authorize
production routing or a selector claim.

Recompute the artifact audit without rerunning optimization or overwriting the
saved one-shot `audit.json`:

```bash
python3 - <<'PY_AUDIT'
import importlib.util
import json
from pathlib import Path
module = Path('research/experiments/mqlib_coarse_quality/check.py').resolve()
spec = importlib.util.spec_from_file_location('mq_quality_audit', module)
auditor = importlib.util.module_from_spec(spec)
spec.loader.exec_module(auditor)
folder = module.parent / 'main001'
assert auditor.audit(folder) == json.loads((folder / 'audit.json').read_text())
print('PASS: retained main audit reproduced')
PY_AUDIT
```
