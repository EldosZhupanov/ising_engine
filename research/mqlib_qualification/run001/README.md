# MQ-QUAL-001 retained run

Instrument commit: `2b6106872d35c912c71e30496c4e97733254c27b`. Status: **PASS**.

Reproduce from repository root after checkout of the instrument commit:

```bash
bash scripts/setup_mqlib.sh
python3 -m unittest discover -s research/mqlib_qualification -p "test_*.py" -v
python3 research/mqlib_qualification/qualify.py --output .cache/mqlib-reproduction
```

Use a new output directory. Wall-limited search can return different equally valid
states; exhaustive objectives must match exactly, every candidate must independently
verify. Timings, temporary paths and binary hashes vary across builds/hosts.
metadata.json retains build output (including upstream warnings), environment and hashes.
See ../README.md and ../../EXTERNAL_COMPARISON_AMENDMENT_1.md for scope.
