# RC-016 falsifiable descendant — frozen after held-in, before held-out

**Status:** binding descendant required by `PREREG_RC016.md` §5.

**Written after:** the held-in block at
`experiments/rc016/rc016_heldin.tsv`.

| provenance field | frozen value |
|---|---|
| held-in artifact SHA-256 | `7ab1e47ad187d1f91ee003cb5cfb090a3d420b369ce2460d9f4cfbaf3a9d904e` |
| size | 3296 bytes |
| mtime | 2026-08-19 20:28:35 +0500 (`1787153315`) |
| held-in seeds | `1001–1008` |
| held-out status when written | not run and not inspected |

No threshold, corpus member, seed, estimand, or verdict rule is changed here.
The equal-cost arm remains withdrawn by Amendment 4.

## Held-in census that generated the descendant

Applying the frozen held-in parts of the rules gives:

- 26 of 30 held-in p-values pass Benjamini–Hochberg at FDR 0.10;
- 25 are also material and non-degenerate, and therefore are candidates for
  final `QUALIFIES` after independent A6 replication;
- every one of those 25 effects has `I_replace_work > 0`;
- `G13`, `G48`, `G49`, and `G50` are `DEGENERATE_NULL`; `G48–G50` are also
  `TIE-BLOCKED`;
- `G53` is neither BH-rejected nor material (`rel = 0.0661% < 0.1%`).

The 25 frozen candidates are:

`G1, G2, G3, G11, G12, G14, G15, G16, G22, G23, G24, G32, G33, G34,
G35, G36, G43, G44, G45, G51, G52, G55, G60, G63, G70`.

Calling them candidates is deliberate: `QUALIFIES` is undefined until the A6
held-out condition is evaluated. Nothing in this document promotes a held-in
effect to a replicated result.

## D-16 — one falsifiable prediction

Let `K_rep` be the number of the 25 named candidates whose held-out row satisfies
the already frozen A6 rule:

1. held-out sign matches held-in (therefore `I_replace_work > 0` here);
2. the held-out effect is independently material under the same conjunctive bar;
3. `0.5 <= I_held-out / I_held-in <= 2.0`.

**Prediction: `K_rep >= 20`.**

- **Confirmed:** `K_rep >= 20`.
- **Refuted:** `K_rep <= 19`.

This is intentionally stronger than the `K >= 6` floor for `SIGN CONSTANT`.
It tests whether the broad held-in pattern is stable across independent seed
blocks, rather than merely asking whether the minimum verdict threshold can be
cleared. The cutoff was proposed by an independent read-only skeptic agent after
seeing only the preregistration and held-in artifact.

## Interpretation guards

- A held-out sign flip fails A6; it must not be relabelled post hoc as evidence
  for `SIGN VARIES` unless some instance independently satisfies the complete
  frozen `QUALIFIES` predicate for the negative sign.
- The `[0.5, 2.0]` A6 window is not widened for small integer-valued effects.
- Degenerate `G48–G50` cannot support interchangeability.
- `G53` cannot be rescued by a post-hoc feature narrative.
- `K_rep >= 20` would concern only equal-sweep `I_replace_work`; it would not
  license an equal-cost, deployment-optimal, scheduler-utility, wall-time, or
  efficiency claim.
