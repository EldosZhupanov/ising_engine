---
name: experimental-methodology
description: Preregistration, hypothesis formulation, and experimental protocol management.
---

# Experimental Methodology Protocol

This skill enforces strict preregistration and scientific integrity standards across all experiments.

## Preregistration Standard

Every non-trivial benchmark, parameter study, or algorithmic comparison must be preregistered before execution:

File location: `research/experiments/<experiment_id>/protocol.md`

### Required Fields in `protocol.md`:
1. **Hypothesis**: Specific, directional, and falsifiable statement (e.g. "Algorithm A achieves $\ge 5\%$ higher median cut on G-set than Baseline B with $p < 0.01$").
2. **Instances**: Exact list of problem instances, benchmark files, and SHA-256 hashes.
3. **Baseline**: Explicit competing solver version, configuration, and source.
4. **Metrics**: Primary and secondary metrics (e.g. median objective, TTS_99, wall-clock seconds).
5. **Budget & Timeout**: Wall-clock deadline per cell / sweep count limit.
6. **Execution Environment**: OS, CPU model, RAM, compiler version (`rustc --version`), `RUSTFLAGS`, Rayon thread count, core pinning.
7. **RNG Seeds**: Explicit list of random seeds (minimum 10 independent seeds for stochastic comparisons).
8. **Sample Size**: Number of paired runs per cell.
9. **Falsification & Success Criteria**: Strict statistical condition for accepting or rejecting the hypothesis (e.g. two-sided Wilcoxon signed-rank test $p < 0.05$ AND median difference $> \delta$).

## Protocol Immutability Rule

- Once an experiment is launched, the success criterion and evaluation protocol are **FROZEN**.
- It is strictly forbidden to alter the success threshold, exclude outliers post-hoc, or switch metrics (e.g. from median to mean) after observing the data.
- If an unforeseen flaw in the protocol is discovered:
  1. Record the discrepancy in an explicit amendment (`AMENDMENT_1.md`);
  2. Create a new experiment version (`<experiment_id>_v2/`);
  3. Never overwrite the original protocol or raw run data.
