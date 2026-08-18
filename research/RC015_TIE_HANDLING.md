# RC-015 — The cold difference between Metropolis and heat-bath is one line about ties

**Status: CONFIRMED**, held-in and held-out, on every instance where the effect
exists. Pre-registered in `PREREG_RC015.md`, committed before the instrument
existed; descendant D-15 committed before the held-out arm. Instrument:
`src/bin/exp_tie_handling.rs`.

**Origin.** RC-014 §10 found `gibbs_color_sweep` decisively better than
`metropolis_sweep` on G43 at flat `T = 0.1` and recorded it as an anomaly, having
dismissed tie handling because G43's tie mass is "only 2.3%". **That dismissal was
wrong on three counts** (`PREREG_RC015.md` §1), and this cycle exists because the
error was caught in review, not because the anomaly was pursued.

---

## 1. Result

Three arms, each consuming exactly one `f64` per `(sweep, site, replica)`, so all
pairwise comparisons are exact counterfactuals:

```
   I_tie  = Y(M½) − Y(M)     tie handling alone
   I_off  = Y(G)  − Y(M½)    everything other than ties
   I_full = Y(G)  − Y(M)  =  I_tie + I_off
```

Flat `T = 0.1`, 8 seeds per arm, after `greedy_descent`:

| Instance | n | `I_full` in | `I_full` out | share in | share out | `I_off` ρ out | verdict |
|---|---|---|---|---|---|---|---|
| **G43** (primary) | 1000 | **−44.125** (ρ 5.31, p .008) | **−40.875** (ρ 2.90, p .008) | **0.963** | **1.058** | 0.214 | **CONFIRMED** |
| G23 | 2000 | **+57.000** (ρ 123, p .008) | **+54.750** (ρ 20.1, p .008) | 1.007 | 0.966 | 0.119 | **CONFIRMED** |
| G24 | 2000 | **+45.000** (ρ 7.70, p .008) | **+49.125** (ρ 9.81, p .008) | 1.150 | 1.051 | 0.411 | PARTIAL in → **CONFIRMED** out |
| G44 | 1000 | −0.250 (p .97) | +1.500 (p .63) | — | — | — | `I_full` not material |
| G45 | 1000 | −5.375 (p .30) | −3.875 (p .37) | — | — | — | `I_full` not material |
| G22 | 2000 | +1.875 (p .71) | −0.625 (p .91) | — | — | — | `I_full` not material |

**In every cell where the effect exists, tie handling carries 94–115% of it, and
the off-tie residual never reaches materiality** (largest `ρ_off` = 0.504
held-in, 0.411 held-out, always at rel < 0.06% against a 0.1% bar). The
decomposition closes **exactly** (`0.000e0`) in all 24 cells.

All three material contrasts **replicate** under the RC-014 A6 rule (sign,
independent held-out materiality, ratio in [0.5, 2.0]): 0.926, 0.961, 1.092.

The picture is the same **before** and **after** `greedy_descent` — share 0.967
vs 0.963 on G43 — so the divergence is present at the quench's starting point and
is not manufactured by it.

## 2. The gradient — the mechanism's quantitative prediction, confirmed

Pre-registered as non-gating: if ties are the channel, their share must **fall as
`T` rises**, because the off-tie acceptance ratio climbs while the tie ratio does
not. The exact identity (Amendment 1 A1.1, verified to 1e-12) is

> **`p_M(ΔE) / p_G(ΔE) = 1 + e^{−|ΔE|/T}` for every `ΔE`, and exactly `2` at
> `ΔE = 0` at every temperature.**

So the tie channel's ratio is **temperature-independent at 2**, while every other
channel decays to 1 as `e^{−Δ_min/T}` — with `Δ_min = 1.0` measured by the
per-arm census. Off-tie the ratio is 1.0000454 at `T = 0.1`, 1.1353 at 0.5 and
1.6065 at 2.0. The cold dominance of ties is the `T → 0` limit of a closed form,
not an approximation.

| T | `I_full` | `I_tie` | `I_off` | tie share |
|---|---|---|---|---|
| 0.1 | −44.125 | −42.500 | −1.625 | **0.963** |
| 0.5 | +15.875 | +11.750 | +4.125 | **0.740** |
| 2.0 | +12.500 | +5.500 | +7.000 | **0.440** |

Monotone, and it crosses over: at `T = 2.0` the off-tie channel is the larger of
the two. `I_full` also flips sign between 0.1 and 0.5, so G43's reversal is
specifically a **cold** phenomenon and the tie channel is what produces it there.

## 3. What this says about the operator library

`metropolis_sweep.rs:117` reads:

```rust
let accept = if d <= 0.0 {
    let _u: f64 = rng.gen();   // alignment draw
    true
} else { /* exp(−d/T) test */ };
```

The `<=` folds `ΔE = 0` into the downhill branch. Heat-bath, by construction,
flips at ties with probability ½. **That single convention accounts for
essentially the entire measured difference between the two operators in the cold
regime** — two entries in the registry, declared as different physical laws with
different capability passports, differ at `T = 0.1` almost only in how they treat
neutral moves.

This is the concrete continuation of RC-014's methodological point. Deletion could
not separate the two operators; substitution separated them; and decomposition now
shows *what* separates them is not "thermal physics" but a tie convention that no
document in this repository had ever named as a design choice.

**Actionable consequence, not applied:** tie handling is a first-class operator
parameter, not an implementation detail. Making it explicit is trajectory-changing
under ADR-0004 and is filed as an open decision.

## 4. What is NOT explained — the sign

The mechanism sets the **magnitude**; nothing here sets the **direction**.

- G43 is **negative** (Metropolis's always-accept is worse); G23 and G24 are
  **positive** (it is better), at 1.2–1.3× the magnitude.
- **Matched siblings disagree.** G43 −44.1, G44 −0.25, G45 −5.4 — all at
  `n = 1000, m = 9990`, all weights `+1`. G22 +1.9 against G23 +57.0 and G24
  +45.0 at `n = 2000, m = 19990`.
- **Tie count does not predict it.** Per-arm census: G43 has the *fewest* ties in
  set A (368/replica) and the *largest* effect; G44 has 453 and none.

So *matching on (n, m, weight) does not determine the effect*, and the RC-014
framing "is the anomaly a G43 peculiarity or a family property?" has a third
answer: **the channel is universal, the sign and magnitude are per-instance.**

What sets the sign is **declared open** in `PREREG_RC015.md` §9 so that any later
answer cannot be presented as anticipated. D-15 pre-registers the dose–response
test (tie-acceptance probability `q` over `{0, .25, .5, .75, 1}`; `Y` must be
monotone in `q`, and `q = 1` bit-identical to `metropolis_sweep`).

## 5. Novelty, stated honestly

**LIKELY KNOWN — the mechanism.** That Metropolis and heat-bath coincide off ties
as `T → 0` follows from their acceptance functions and is arithmetic, not
discovery; that they differ at `ΔE = 0` by `1` versus `½` is a definition. The
closed form `p_M/p_G = 1 + e^{−|ΔE|/T}` is likewise a derivation, not a finding. Under
`RESEARCH_GAPS.md` §5c this is at best LIKELY KNOWN.

**What is measured, and was not known here:** that this single convention accounts
for ~100% of a *material, replicating, instance-specific* performance difference
on real benchmark instances at low temperature, with a monotone temperature
gradient, and with a sign no measured instance property predicts. The repository
had never named tie handling as a design axis.

## 6. Controls and instrument

| Control | Result |
|---|---|
| `TieMode::Accept` bit-identical to `metropolis_sweep` | **pass**, 3 seeds, same pre/post energies and best state — the null lives inside the operator (RC-001) |
| All three arms leave the generator aligned | **pass**, `0x6a96f3fefc02b846` from M, M½ and G alike |
| Injected one-word shift detected | **pass** |
| Decomposition closes | **pass**, `0.000e0` in all 24 cells |

M½ is registered into an **experiment-local** registry; the production registry
stays at 18 operators (Constitution §13 — prototypes do not touch production).
Per-arm `ΔE`-class counts come from each arm's own trajectory, so RC-014's
post-treatment defect cannot recur.

**Reporting defect found and fixed mid-cycle, recorded rather than quietly
patched.** The verdict predicate fired "CONFIRMED" on cells where `I_full` is
null, because the share ratio is undefined there (it reached 19.0 on G44) and
"residual not material" is trivially true. The gate is now evaluated only where
`I_full` is itself material. The numbers never changed; only the predicate was
wrong.

## 7. Scope

Flat `T = 0.1` (plus the three-point gradient on G43 only) · one schedule
`[X@16, greedy@16]` · legacy all-zeros init · 32 replicas · six G-Set instances,
all unweighted · cache-resident scale. No Runtime change, no `theory.rs` change,
no production operator added, no published fact altered. RC-014's Gate B remains
failed and is not reopened.

## 8. Reproduction

```bash
cargo run --release --bin exp_tie_handling -- --controls
cargo run --release --bin exp_tie_handling -- --science
cargo run --release --bin exp_tie_handling -- --holdout
cargo run --release --bin exp_tie_handling -- --gradient
```

Expected: all three controls pass; G43 `I_full = −44.125`, share 0.963 held-in and
−40.875, share 1.058 held-out; gradient share 0.963 → 0.740 → 0.440.

## 9. Follow-ups

1. **D-15** — dose–response in the tie probability `q` (pre-registered).
2. **What sets the sign** — open, no candidate. The obvious next observables are
   properties of the zero-field network itself (its size, connectivity, and where
   it sits relative to the basins the quench reaches), none of which this cycle
   measured.
3. **Make tie handling an explicit operator parameter** — trajectory-changing
   under ADR-0004; open decision, not applied.
4. **Re-read RC-014 §10 with this in hand:** G11 and G32 carry 13–34% tie mass,
   an order of magnitude above G43's 2.3%, yet were never decomposed. Their
   ladder-based effects are the natural next cell for this instrument.
