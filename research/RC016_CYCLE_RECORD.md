# RC-016 cycle record — sensor sufficiency by exact replacement

**Status:** complete. All frozen controls passed; held-in and held-out were run
once on their preregistered disjoint seed blocks.

## Provenance

| artifact | SHA-256 |
|---|---|
| `experiments/rc016/rc016_calibration.tsv` | `2905eefd54c90c9b39dd80c77f7762904d24cfe845c8f44a183e3bbf4531e424` |
| `experiments/rc016/rc016_heldin.tsv` | `7ab1e47ad187d1f91ee003cb5cfb090a3d420b369ce2460d9f4cfbaf3a9d904e` |
| `experiments/rc016/rc016_heldout.tsv` | `8e028f509110742980c268862a38babd67e181ea074562a13e50c8a74068862c` |

The falsifiable descendant was committed as `464461a` after the held-in artifact
and before held-out. Its prediction was `K_rep >= 20` among the 25 frozen
held-in candidates.

## Controls

- corpus: 30/30 present with preregistered hashes;
- CI seed space: 60/60 distinct and disjoint from the power seed;
- arithmetic positive and tie-block guard: pass;
- RNG alignment: Metropolis and Gibbs end at the same generator word, deliberate
  one-word shift detected;
- null replacement: bit-identical;
- both inert controls: exactly zero / state-identical;
- independent operator-positive reference: 648/648;
- Amendment 4 provenance gate: pass;
- descendant gate before held-out: all four conditions pass.

## Frozen descendant D-16

**CONFIRMED: `K_rep = 25/25`.** Every named held-in candidate retained the
positive sign, was independently material on held-out, and had
`0.5 <= I_out/I_in <= 2.0`. The observed ratio range was 0.535714 (`G15`) to
1.887324 (`G3`). The prediction required only 20.

## Preregistered verdict

```text
K = 25 qualifying of 30
k+ = 25, k- = 0
qualifying coverage = 5 matched groups, 2 structural families
degenerate cells = 4/30 (13.3%)
VERDICT = SIGN CONSTANT
```

Thus H-16 — the proposed construction of an `S0`-matched opposite-action pair —
is **refuted for this operator pair, slot, initialization, corpus and equal-sweep
budget**. The cycle did not find a counterexample to the frozen sensor map because
no negative qualifying action exists here.

The positive result is narrower but unusually stable: replacing
`metropolis_sweep` with `gibbs_color_sweep` worsened the objective on all 25
qualified instances under the exact paired counterfactual. This establishes a
causal equal-sweep outcome ordering for this experimental configuration, not an
equal-cost result or a universal ordering of Markov kernels.

## Exceptions and non-results

- `G13` is degenerate on held-in despite becoming non-degenerate/material on
  held-out, so it cannot qualify.
- `G48–G50` are degenerate and tie-blocked in both blocks. They provide no
  evidence of interchangeability.
- `G53` is non-material and not BH-rejected on held-in, although material on
  held-out. Held-out cannot rescue a failed held-in predicate.
- There is no qualifying negative instance and therefore no `S0` causal
  counterexample in this cycle.

## Binding scope limits

This cycle measured `I_replace_work` at equal sweeps only. Amendment 4 withdrew
the cost arm because whole-policy timing does not identify equal operator cost.
Therefore the result licenses **none** of the following:

- equal-cost or wall-time superiority;
- deployment-optimal defaults;
- scheduler utility or a policy recommendation;
- a general theorem that Metropolis dominates heat bath;
- novelty claims for algorithm selection, DAC, or feature insufficiency.

The contribution that survives is methodological: the exact replacement
instrument distinguishes two operators that deletion against a common
greedy-only remainder could not distinguish, and it did so with preregistered,
held-out replication. The hoped-for sensor-impossibility result did not survive.
