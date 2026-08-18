# RC-008 — Ensemble collapse at low temperature, and an audit of RC-003's premise

**Vector 2 (spectral / mixing complexity), turned inward.** RC-003 sharpened a
hypothesis using the ergodic theorem — temporal and ensemble statistics coincide
*at equilibrium*. That premise was never tested. RC-002 separately found
initialization diversity surviving to `temp_hi = 4.0`, which is itself a
statement that the chain has **not** mixed. This cycle tests the premise.

## Method — two-chain coupling

Two ensembles from **independent** random configurations, independent RNG
streams, identical dynamics, **flat** ladders (a PT ladder would confound chain
mixing with replica exchange). Track:

- `q_AB` — mean |overlap| across chains
- `q_AA` — mean |overlap| within one chain; at equilibrium "which chain" is
  meaningless, so these must agree
- energy agreement within **2σ of the sampling error of the means**

Absolute overlap throughout: zero-field Ising has an exact Z₂ gauge symmetry, so
signed overlap averages to zero for reasons unrelated to mixing.

## A mis-calibrated criterion, caught and fixed

The first pass reported "not mixed" at **every** temperature. That was a false
negative of my own making: I required `|E_A − E_B| < 1e-3` in relative terms — an
absolute threshold **below the sampling noise of a 16-replica mean**. Two
independent ensembles differ by O(σ/√R) from sampling alone. Replacing the
constant with a 2σ bound on the difference of two means corrected it.

## Result — G11, n=800, R=16

| T (flat) | q_AB | q_AA | E agree (2σ)? | reading |
|---|---|---|---|---|
| **0.1** | 0.22–0.24 | **1.0000** | **no** | **collapsed, not mixed** |
| 0.5 | 0.110–0.112 | 0.115–0.127 | yes | mixed in overlap |
| 1.0 | 0.045 | 0.038–0.047 | yes | mixed in overlap |
| 2.0 | 0.032 | 0.029–0.034 | yes | mixed in overlap |
| 4.0 | 0.029 | 0.028–0.034 | yes | mixed in overlap |
| 8.0 | 0.027–0.030 | 0.026–0.032 | yes | mixed in overlap |

### The headline: at T = 0.1 the ensemble is not an ensemble

**q_AA = 1.0000 exactly.** All 16 replicas within a chain have collapsed to a
*single configuration*. Meanwhile the two chains sit 0.22–0.24 apart and their
energies never agree, unchanged from 200 to 800 sweeps. Each chain froze into its
own basin: broken ergodicity, definitively, and "ensemble statistics" at this
temperature are statistics of one point duplicated 16 times.

### T ≥ 0.5 mixes by 200 sweeps

Cross- and within-ensemble overlap are statistically indistinguishable, and
energies agree within 2σ at every checkpoint. The ergodicity crossover sits
between T = 0.1 and T = 0.5.

## Consequences for this session's own results

**RC-003's ergodic premise was sound where it was applied.** Its sweeps ran
`temp_hi` over 0.1–4.0, and mixing holds for T ≥ 0.5. The cold end was outside
the premise, which is a scope caveat on RC-003 rather than a refutation.

**RC-002 is reconciled, not contradicted.** It reported diversity surviving to
`temp_hi = 4.0` — but those were PT **ladders** spanning `temp_hi → 0.1`, so every
configuration always contained cold, non-mixing replicas. The *ladder* preserved
the memory. This cycle uses flat ladders and finds mixing at the same nominal
temperatures. Both are correct; they measure different objects.

## Limitations — stated, not buried

- **R = 16 is too small for a tight overlap tolerance.** The MIXED / not-mixed
  verdict flickers between checkpoints at T ≥ 0.5 because a 10% band on q_AA is
  narrow against the sampling noise of 120 replica pairs at q ≈ 0.03. The
  flicker is instrument noise and is **not** reported as signal; the substantive
  claim is only that q_AB and q_AA are indistinguishable there.
- One instance (G11, toroidal ±1). Not replicated across families.
- `metropolis_sweep` only.
- Mixing is assessed in overlap and mean energy — necessary conditions, not a
  proof of convergence to the Gibbs measure.

## Reproduction

```bash
cargo run --release --bin exp_mixing_time -- \
    --file benchmark_suite/data/gset/G11 --replicas 16 --sweeps 800 --step 50
```
