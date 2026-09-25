---
name: reproducibility
description: Standards for environmental provenance, artifact metadata, and complete experimental reproducibility.
---

# Reproducibility Protocol

This skill ensures that every benchmark result and scientific experiment can be replicated by an independent external researcher.

## Required Metadata

Every benchmark run must generate two artifact files alongside the raw output:

1. `metadata.json` (machine-readable)
2. `README.md` (human-readable execution instructions)

### Schema for `metadata.json`:
```json
{
  "experiment_id": "string",
  "git_commit": "string (40-char SHA)",
  "git_dirty": "boolean",
  "benchmark_dataset": {
    "name": "string",
    "git_commit": "string",
    "url": "string"
  },
  "environment": {
    "os": "string (e.g. Linux 6.8.0-x86_64)",
    "cpu": "string (e.g. AMD Ryzen / Intel Core i7-xxx, core/thread count)",
    "ram_bytes": "integer",
    "rust_version": "string (rustc -V output)",
    "profile": "release | debug",
    "rustflags": "string",
    "threads": "integer",
    "env_vars": {}
  },
  "execution": {
    "command_line": "string",
    "seeds": ["integer"],
    "timeout_seconds": "number",
    "start_time_iso": "string",
    "end_time_iso": "string"
  }
}
```

## Reproducibility Audit Test

An experiment is declared **REPRODUCIBLE** only if an independent auditor or external agent can:
1. Check out the exact specified git commit;
2. Execute the exact `commands.sh` script;
3. Obtain numerical outputs matching the recorded results within documented floating-point or stochastic tolerance.
