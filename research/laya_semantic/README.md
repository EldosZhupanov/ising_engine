# LAYA-001 semantic reconciliation pilot

An isolated offline chain from the installed Laya English checkpoint to the
production UltimateSolver, with independent exhaustive checks and existing
presolve instrumentation. Start with [the frozen protocol](PROTOCOL.md) and
[the Builder intake](HYPODIVE_BUILDER_INTAKE.md). This is a synthetic capability
pilot, not evidence of real-domain accuracy or speed superiority.

## Verification

From the repository root:

```bash
python3 -m unittest discover -s research/laya_semantic -p 'test_*.py'
cargo test --offline --release -p research --example laya_bridge
cargo clippy --offline -p research --example laya_bridge -- -D warnings
cargo check --offline -p research --example laya_bridge
cargo build --offline --release -p research --example laya_bridge
rustfmt --edition 2021 --check research/examples/laya_bridge.rs
```

The bridge accepts JSONL with `linear`, unique upper-triangular `pairs`, `offset`
and `seed`. At most 16 variables; exponential oracles are deliberately bounded.
It returns full exact, UltimateSolver and reduced-exact assignments/energies,
the complete energy spectrum and reduction diagnostics. No truth enters Rust.
The production solver already includes presolve; it is not a no-presolve arm.

## Frozen pilot

Run from a clean checkout of the recorded instrument commit, with output outside
that checkout. The installed Python environment and snapshot are local resources,
not bundled dependencies. No model downloads or CUDA installation are performed.

```bash
PYTHONDONTWRITEBYTECODE=1 /tmp/laya_test_venv/bin/python \
  research/laya_semantic/run_pilot.py \
  --output /tmp/laya-001-run \
  --bridge /absolute/path/to/target/release/examples/laya_bridge \
  --snapshot /home/eldos/.cache/huggingface/hub/models--convaiinnovations--laya/snapshots/5e7b2b1b8ca2ecdd3f2322d94069c9b6ce7e844b
python3 research/laya_semantic/analyze.py /tmp/laya-001-run
```

The runner refuses a nonempty existing output directory by default. `--resume`
checks instrument/model identities, reuses successful raw inference files, and
retains each bridge attempt separately. It does not authorize a new scientific
run after a correctness failure; follow the protocol's amendment rule.

Keep raw answers, case files, corpus, environment and completion marker together.
Analysis regenerates descriptive metrics from the raw probabilities and original
truth, and rechecks the cross-language energy spectrum and optimum preservation.
Elapsed observations include startup and are not calibrated benchmarks.

## Boundaries

- Penalties encode exclusions and directed implications only; all-zero is feasible.
- The domain truth is synthetic and consistent by construction. Real annotations
  and noisy/incomplete rules require a separate protocol.
- Finite probability clipping is explicit; the installed checkpoint is not calibrated
  to this domain. Confidence scores never determine fixing or correctness.
- Existing first-order/full presolve and component extraction are reused unchanged.
  Presolve is exact with respect to the objective, not a guarantee of text truth.
- Tests include failure witnesses for corrupted energies and retained bridge errors.
