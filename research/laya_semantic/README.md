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

## Training feasibility assessment — 2026-09-27

This is a prospective hypothesis, not a training result or a change to LAYA-001.
The local snapshot `5e7b2b1b8ca2ecdd3f2322d94069c9b6ce7e844b` is still
present. Its safetensors header contains 421,293,830 stored tensor elements;
`rl_agent_config.json` identifies ModernBERT-large, two head layers and context
512. Thus the older result's “Laya-1B” label is inaccurate. The earlier
`/tmp/laya_test_venv` no longer exists; retained weights do not establish a
working training environment. No packages or weights were changed.

Upstream inspection was pinned to
[`4066d5d5fbf08b66c6757ddeedbd797bd7655bc0`](https://github.com/NandhaKishorM/laya/tree/4066d5d5fbf08b66c6757ddeedbd797bd7655bc0).
Its [fine-tuning notebook](https://github.com/NandhaKishorM/laya/blob/4066d5d5fbf08b66c6757ddeedbd797bd7655bc0/notebooks/laya_finetune_typed_decisions_2xT4_kaggle.ipynb)
and [`proper_reward`](https://github.com/NandhaKishorM/laya/blob/4066d5d5fbf08b66c6757ddeedbd797bd7655bc0/laya/common.py)
already implement training machinery and log/spherical/ranked-distribution reward
components. This read establishes that changing weights is technically possible;
it does not independently validate upstream training/accuracy claims. The local
2026-09-23 pilot used package 0.3.7, not this later upstream source.

**HYPOTHESIS:** on externally annotated groups with genuine interacting decisions,
a solver-informed training loss improves held-out joint decision accuracy beyond
ordinary supervised training followed by the *same* constrained decoder at the
same training/inference cost. Begin with a frozen encoder and a small trainable
head to isolate the loss; do not attempt to optimize all 421M weights as a QUBO.

A possible training loop is text -> logits -> constrained candidate decisions ->
independently checked candidates -> supervised/structured loss -> gradient update.
Discrete argmin is not automatically differentiable. Specify an estimator
(e.g. a structured surrogate or policy gradient) and test its bias/variance and
cost; a changed reward is not automatically a new learning algorithm. A solver
can certify constraint feasibility, not whether the text interpretation is true.
Do not use every repaired prediction as a correct target: that can reinforce
errors or collapse to a trivially feasible all-zero decision.

Closest verified prior art:

- Xu et al., ICML 2018, [A Semantic Loss Function for Deep Learning with Symbolic
  Knowledge](https://proceedings.mlr.press/v80/xu18h.html), PMLR 80:5502–5511:
  symbolic constraints in a neural training objective.
- Mandi et al., ICML 2022, [Decision-Focused Learning: Through the Lens of Learning
  to Rank](https://proceedings.mlr.press/v162/mandi22a.html), PMLR 162:14935–14947:
  training objective coefficients by ranking feasible decisions.

Novelty status: **OVERLAPPING / specific contribution UNKNOWN**. A constraint loss,
solver-in-training or verifier-derived reward alone is not a new class of AI.
No general learning or calibrated-confidence guarantee follows from this proposal.

Minimum separate experiment before promotion: human/externally annotated grouped
train/validation/test split, split by source to prevent template leakage; retain
raw model, supervised head, supervised head plus identical decoder, and proposed
loss plus identical decoder. Match encoder access, labels, tuning and compute;
measure joint accuracy, constraint violations, calibration and total cost. Include
wrong/noisy-rule and all-zero controls; enumerate small cases independently.
The earlier 24 synthetic pilot groups are opened development evidence, not a
fresh test set; complete presolve there does not demonstrate a hard-search gain.
Training requires a separately frozen corpus, protocol and resource budget.
