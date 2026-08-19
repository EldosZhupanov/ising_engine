# PREREG — RC-017: mid-run causal action variation and `S1` capture

**Status:** binding pre-registration, written before RC-017 instrument code and
before any RC-017 pilot, held-in, or held-out datum exists.

RC-016 established `SIGN CONSTANT` for the first-slot replacement
`[X@16, greedy_descent@16]`: 25/25 qualifying effects favored Metropolis at equal
sweeps. RC-017 asks the next smaller question already reserved by RC-016 §10:
does the causal choice change in the second slot after a shared thermal prefix?

This document deliberately does **not** call that question “sensor sufficiency.”
The reason is mathematical and architectural (§4): no deployed operator-choice
policy consumes the continuous `S1` vector, and arbitrary distinct `f64` vectors
are separable by an unrestricted deterministic policy.

## 1. One primary experiment

For every instance and seed, compare these two plans:

```text
M arm: [metropolis_sweep@16, metropolis_sweep@16, greedy_descent@16]
G arm: [metropolis_sweep@16, gibbs_color_sweep@16, greedy_descent@16]
```

Everything except the operator in slot 2 is identical: legacy all-zeros
initialisation, 32 replicas, geometric ladder `4.0 -> 0.1`, the RC-016 hashed
30-instance G-Set census, and best-of-32 canonical energy at the end of the plan.

The primary estimand is

```text
I_replace_work(i, slot 2) = E_final(G arm) - E_final(M arm).
```

Positive means Metropolis is better; negative means Gibbs is better. “Work” here
means equal sweeps only. It does not mean equal wall cost.

The primary question is **action-sign variation**:

> Among independently replicated, material instance effects, do both signs
> occur after the fixed Metropolis prefix?

## 2. Exactness proof and required controls

The counterfactual is exact under the current stream RNG only for this design:

1. Both arms execute the identical `metropolis_sweep@16` prefix from the same
   state and seed, so the pre-slot-2 state, ledger, and RNG position are identical.
2. Metropolis and Gibbs traverse the same coloring order and unconditionally
   consume one `f64` draw per `(sweep, site, replica)` in slot 2.
3. `greedy_descent` consumes no randomness downstream.
4. Runtime quality/sensor recording reads energies and overlaps, takes no RNG
   reference, and does not mutate spin state.

Before either science block, the instrument must pass:

- null replacement: both complete M-arm plans are bit-identical;
- prefix identity: independently executed common prefixes have identical state,
  ledger digest, RNG probe, and `StepEvent[0]` sensor bits;
- aligned M/G slot-2 arms leave the RNG probe identical;
- a deliberate one-word shift is detected;
- zero-sweep and structurally inert controls return exactly zero;
- the independent operator reference reproduces all 648 fixtures;
- a recorder that only reads `StepEvent[0]` leaves final state and energy
  bit-identical;
- the historical RC-016 first-slot control remains reproducible from its frozen
  TSV/hash. It is a regression control, not part of RC-017 inference.

Any failed control kills the cycle before a science seed is run.

## 3. Frozen design and provenance

| item | frozen value |
|---|---|
| corpus | the exact 30 entries and hashes in `PREREG_RC016.md` §4 |
| initialisation | legacy all-zeros only |
| prefix | `metropolis_sweep@16` |
| substituted slot | slot 2, `X@16` |
| finisher | `greedy_descent@16` |
| pilot/control seeds | `4001–4008` |
| held-in seeds | `5001–5008` |
| held-out seeds | `6001–6008` |
| CI bootstrap base | `20260901 + 2*instance_index + block` |
| primary horizon | canonical best-of-32 energy after the finisher |

All seed blocks are disjoint from RC-016. Pilot seeds may exercise controls and
sensor non-vacuity, but pilot outcome contrasts may not select instances,
thresholds, prefix length, or a descendant claim. There is no diverse-init arm:
the common thermal prefix creates dynamic state while preserving the exact
initialisation contract and avoiding an RC-002/RNG confound.

The implementation must hard-refuse held-in unless this file is tracked, clean,
and committed before the first RC-017 artifact. It must hard-refuse held-out
unless `research/PREREG_RC017_DESCENDANT.md` is tracked, clean, and committed
strictly after held-in. No cost/equalised-sweep mode may exist.

## 4. Frozen `S1` capture — secondary, correctly typed

The snapshot is taken after the common prefix and before slot 2, from
`StepEvent[0]`. RC-017 records the exact non-bias vector used by the deployed
`dynamics::step_features` transformation:

| index | coordinate |
|---:|---|
| 0 | `FeatureRegistry::v0.log_n` |
| 1 | `density` |
| 2 | `clustering` |
| 3 | `mean_degree / 10` |
| 4 | `degree_cv` |
| 5 | `energy_entropy` |
| 6 | `diversity` |
| 7 | prefix acceptance |
| 8 | `frac_elapsed = 1/3` |
| 9 | `recent_progress = 0` at history index `k=0` |
| 10 | `(mean_energy - best_energy) / max(abs(best_energy), 1)` |
| 11 | `best_energy / (abs(first_best_energy) + 1)` |

Coordinates 8 and 9 are constants by construction. The linear bias `1.0` is
stored in the model input but omitted from the descriptive sensor table.

The instrument stores all 12 coordinates per seed and their instance means.
It must call one shared implementation of the deployed transformation; a private
copy in the experiment binary is forbidden.

### What these sensors may and may not establish

RC-017 performs exactly one sensor inference. For every concrete per-seed common
prefix snapshot `(instance, seed)`, associate the paired individual horizon effect
`delta = E_final(G arm) - E_final(M arm)`. Search for two snapshots whose 12
coordinates are bit-identical but whose nonzero `delta` values have opposite
signs. Such a collision is an information-theoretic counterexample for predicting
the deterministic realised horizon effect from this projection: a policy sees the
same vector but one action cannot be correct for both snapshots.

Instance-mean vectors are reported descriptively but **cannot** establish this
counterexample, because a runtime policy acts on an individual snapshot, not on a
mean over seeds. Near collisions, nearest-neighbour distances, standardized
distances, classifiers, and linear probes are out of scope: without a separately
frozen function class or regularity bound they cannot prove sensor insufficiency.

Code inspection fixes the scope further:

- `OperatorPolicy` selects operator tokens from five static `S0` features and
  token context; it does not see `S1`;
- `DynamicsModel` consumes `step_features`, but predicts remaining improvement
  for `Stop/SwitchOperator/Continue`; it is not a Metropolis/Gibbs selector;
- Runtime adaptation thresholds use only selected scalar sensors.

Therefore RC-017 makes no claim about a currently deployed `S1` operator policy.

## 5. Statistics inherited unchanged from RC-016

Per instance and block:

- exact paired two-sided sign-flip test over 8 differences;
- held-in Benjamini–Hochberg at FDR `q=0.10` over all 30 instances;
- materiality requires both `rho = abs(I)/d_seed >= 0.5` and
  `rel = abs(I)/abs(E_M) >= 0.001`;
- `d_seed < 1e-9` is `DEGENERATE_NULL` and cannot qualify;
- `z >= 3` exact zero differences is `TIE-BLOCKED` and cannot qualify;
- A6 replication requires matching sign, independent held-out materiality, and
  `0.5 <= abs(I_out/I_in) <= 2.0`;
- `QUALIFIES = held-in BH rejection AND held-in materiality AND A6`.

The deterministic 100,000-replicate percentile CI remains descriptive and uses
the frozen bootstrap seed formula above. It never changes qualification.

## 6. Primary verdicts

Let `K` be the number of qualifying instances, with `k+` positive and `k-`
negative.

- **BENCHMARK-VALIDITY:** at least 15/30 instances are degenerate in either
  block. This overrides every sign verdict.
- **SIGN VARIES:** `k+ >= 1` and `k- >= 1`. This establishes causal
  instance-dependent action preference at this exact prefix/slot design.
- **SIGN CONSTANT:** `K >= 6`, one sign only, spanning at least three frozen
  matched groups and both structural families.
- **NO MATERIAL EFFECT OBSERVED:** every instance is non-degenerate,
  non-tie-blocked, and non-material in both blocks.
- **Q-INCONCLUSIVE:** every remaining pattern.

Isolated degenerate or tie-blocked cells fail individually; they do not force the
whole census to `Q-INCONCLUSIVE`. `SIGN CONSTANT` licenses only:

> Under `[metropolis@16, X@16, greedy@16]` in the frozen RC-017 configuration,
> the sign of equal-sweep `I_replace_work` in slot 2 did not vary across
> qualifying instances.

It does not license invariance to thermalisation generally.

## 7. Descendant gate and kill criteria

After held-in and before held-out, one numeric falsifiable descendant must be
written and committed. It must name its candidate set and exact pass/fail
threshold using only held-in quantities. Held-out remains inaccessible until the
gate verifies the chronology and an explicit confirmation flag is supplied.

The cycle is killed or scoped as follows:

1. Any mandatory control failure: instrument invalid; no science claim.
2. At least 50% degenerate: benchmark-validity result; stop sign analysis.
3. `SIGN CONSTANT`: stop mining this pair at this prefix/slot; do not vary prefix
   length post hoc.
4. `SIGN VARIES` without an exact per-seed sensor collision: action variation is the
   primary result; no sensor-impossibility claim.
5. No exact collision: report its absence at the observed snapshots; do not add
   tolerances, classifiers, features, policy classes, or more seeds in this cycle.

## 8. Explicitly out of scope

- equal-cost, wall-time, throughput, deployment-default, or scheduler-utility
  claims;
- other prefix lengths, ladders, initialisations, finishers, operator pairs, or
  weighted instances;
- Runtime/ADR-0009 changes, Foundry, DSL search, new operators, growth campaigns;
- tuning a sensor-distance tolerance after seeing action signs;
- calling approximate feature proximity an impossibility theorem;
- using held-out to rescue a failed held-in predicate.

The novelty boundary remains the one frozen in `RC016_NOVELTY_REVIEW.md`: generic
algorithm selection, DAC, and feature insufficiency are known. RC-017 can
contribute an exact causal mid-run ordering result and, only in the unlikely exact
collision case, a domain-specific projection counterexample.
