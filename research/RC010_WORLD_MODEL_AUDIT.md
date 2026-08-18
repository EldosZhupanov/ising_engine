# RC-010 — Auditing "World model: imagined-vs-real Spearman 0.975"

**Method:** the user's generative principle — an optimizer contains internal
models (cost, mixing, operator equivalence, state representation) and
discrepancies between model and behaviour are where new algorithms live. RC-005,
RC-007, RC-008 and RC-009 covered four. This one audits a *claim* rather than a
model: suspiciously strong results are the cheapest to falsify.

`ROADMAP.md` presents the World model's imagined-vs-real schedule ranking at
**Spearman 0.975** as evidence it is a useful cheap filter.

## What the forensics found — and one thing I got wrong

**Correct:** `world.rs`'s own test asserts only `rho >= 0.5`. The 0.975 is an
*observed value*, never an enforced property.

**Correct:** of its 5 candidate schedules, **4 are verbatim members of the
training set**. The source comment says "held-out-ish", which is honest; the
ROADMAP presentation is not.

**WRONG, and corrected here:** I first argued 0.975 is attainable only at n = 9
(Σd² = 3), so it could not have come from the n = 5 test. That assumed *untied*
Spearman. `evaluation.rs:110` averages ranks over ties, which makes intermediate
values reachable at n = 5 — and the reproduce arm below returns **0.9747**. The
number almost certainly did come from that test. My inference was wrong.

## Pre-registered prediction

Three candidate sets against one trained model:

- **A reproduce** — the original set (4/5 in training).
- **B heldout** — no candidate in training, still spanning noise→greedy, so the
  easy quality gap survives.
- **C homogeneous** — every candidate ends in `greedy_descent`, differing only in
  thermal prefix. The easy gap is **removed**; only fine-grained ranking remains.

Predicted: rho **collapses on C**, showing the correlation carried one obvious
distinction rather than ranking skill.

## Result — prediction REFUTED

| set | n | in-train | Spearman | real-score spread | note |
|---|---|---|---|---|---|
| A reproduce | 5 | **4/5** | **0.9747** | 272.5% | easy gap present |
| B heldout | 9 | 0/9 | **0.8984** | 286.5% | no overlap |
| C homogeneous | 9 | 0/9 | **0.8485** | **37.9%** | easy gap removed |

rho does **not** collapse. From B to C the ranking task narrows **7.5×**
(spread 286% → 38%) while rho falls only 0.90 → 0.85. The model retains
substantial ranking ability on a much harder, fully held-out task. **The World
model is better than this audit suspected.**

## The finding that stands

The claim is **overstated, not wrong**:

- **Reported: 0.975. Honest held-out value: ≈ 0.85–0.90.**
- The ~0.08 gap between A and B is attributable to 4/5 candidates being
  in-sample. Real inflation, modest size.
- The remaining correlation survives removal of the easy gap, so it is genuine
  ranking skill, not one bit of noise-vs-greedy.

**Recommended documentation fix (applied):** report the held-out figure and note
the in-sample caveat, rather than the 0.975.

## Limitations

- n = 9, **one** instance, 3 seeds. rho = 0.85 vs 0.90 is plausibly within noise;
  the A > B > C ordering should not be over-read.
- One instance family (frustrated rings), matching `world.rs`'s own test so the
  comparison is like-for-like rather than a harder problem.
- The model is small (ridge over observable-state features); this audits the
  reported metric, not the architecture.

## Reproduction

```bash
cargo run --release --bin exp_world_model_audit
```
