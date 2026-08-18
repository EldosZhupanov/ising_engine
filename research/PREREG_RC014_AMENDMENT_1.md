# PREREG — RC-014 Amendment 1: make Phase 1 executable and falsifiable

**Status:** binding amendment to `PREREG_RC014.md`, written and committed before
instrument code.  The original pre-registration remains unchanged except for a
pointer to this file.  The provisions below replace only the conflicting clauses
they name; every other threshold, instance, seed, arm and gate remains binding.

**Reason for amendment:** an independent read-only review found that the proposed
controls and transition record asked the existing `RunController` seam to observe
data it cannot see, and that equal draw count had been described incorrectly as
equal measured cost.  Correcting those defects before implementation preserves
the purpose of pre-registration; ignoring them would make Gate A subjective or
unexecutable.

---

## A1. Primary estimand: equal work, not equal measured wall-cost

Section 6's statement that equal sweeps are "budget matching ... exact by
construction" is replaced by the following.

The Gate-A primary estimand is:

```text
I_replace_work(k) = Y(P[k <- B, equal sweeps and equal draws]) - Y(P)
```

It is an **equal-work / equal-random-opportunity** counterfactual, not an
equal-wall-time counterfactual.  `metropolis_sweep` and `gibbs_color_sweep` visit
the same `(sweep, site, replica)` coordinates and consume one `f64` draw at each,
but their instruction mix differs: Gibbs evaluates its exponential
unconditionally while Metropolis does so only on the uphill branch.  Equal draw
count therefore does not prove equal elapsed cost.

Wall cost is a required observable.  Before the held-in science run, conditions
are timed interleaved within repetitions and summarised by the median, separately
per instance and initialisation arm.  The frozen table reports
`median_ms_gibbs / median_ms_metropolis`; it does **not** change sweep counts and
does not affect Gate A.

The measured-cost estimand is a required robustness endpoint for Gate B:

```text
I_replace_cost(k) = Y(P[k <- B_c]) - Y(P)
```

where integer sweep counts are selected from the frozen calibration table before
the Gate-B instances are run.  No online adjustment is permitted.  Because the
Phase-1 schedule has no downstream stochastic operator (`greedy_descent` consumes
zero draws), unequal calibrated draw counts cannot shift a later random stream;
they are part of the explicitly different compute budget.  Gate A makes no
equal-cost claim.

---

## A2. Observable Phase-1 data unit

Section 8's full tuple is a long-term schema, not wholly observable through the
current read-only seam.  Phase 1 records only:

```text
(S_post_t, O_t, B_t) -> (acceptance_t, Y_{t:H})
run metadata          -> (backend, seed, init_arm, instance, total/aggregate cost)
```

with `H in {1, 2, end}`.  For `t > 0`, `S_post_(t-1)` is the next transition's
`S_pre_t`; the initial pre-state is known from the declared initialisation arm.
`S_post` contains exactly the fields exposed by `StepSensors`: iteration,
fraction elapsed, operator, best energy, mean energy, energy entropy, diversity
and acceptance.  Horizon values are reconstructed from the ordered best-energy
series.

The following are **not claimed as Phase-1 transition fields**:

- per-invocation wall cost (`Profiler` aggregates by operator name);
- RNG state/fingerprint (`Runtime::rng` is private);
- pairwise overlap and acceptance by temperature rung (`S_2`, not exposed).

Aggregate wall cost comes from the frozen interleaved calibration.  `S_2` remains
frozen as a permitted future extension but is not silently fabricated in Phase 1.
Adding any missing field requires a separately reviewed seam and cannot be done
by parsing the Runtime's human-readable decision string.

---

## A3. RNG-alignment control

The requirement that the Runtime recorder emit a per-step draw counter and RNG
fingerprint is replaced by a direct machine check at the operator boundary:

1. construct identical states and cloned `ChaCha8Rng`s;
2. apply Metropolis to one and Gibbs to the other with equal non-zero sweeps;
3. draw the next `u64` from each generator;
4. require exact equality;
5. in the deliberate-shift arm, consume one extra `u32` before the final probe
   and require inequality.

This validates stream position without exposing the private Runtime RNG or
changing the read-only core.  Run-level null replay still requires identical
best state, energy and event fields for `metropolis_sweep -> metropolis_sweep`.

The control proves only the pre-registered pair and schedule.  It does not
generalise to other operators, deletion, or a downstream stochastic operator;
ADR-0009 continues to block those claims.

---

## A4. Controls

Section 7's controls are replaced as follows.

### Null

Unchanged: replacing `metropolis_sweep` with itself at non-zero sweeps must be
bit-identical at the run outcome and event level.

### Inert A — zero budget

Unchanged: both substituted thermal operators at `sweeps = 0` consume no draws
and change no state.  This is plumbing-only and is not counted as the independent
inert control.

### Inert B — structurally inert replica exchange

From the legacy all-zero initialisation, before any diversity-producing step,
all replicas are identical.  A `replica_exchange` step can only permute identical
states and is therefore state-inert.  Deleting that step, with no later stochastic
operator, must leave best state and canonical energy exactly unchanged.  This is
the RC-007 vacuity case used deliberately as a control.  The prior claim that a
post-greedy state is a fixed point of Metropolis and Gibbs at `T > 0` is withdrawn:
thermal kernels can leave a local optimum.

### Synthetic arithmetic positive

The statistical/attribution layer is fed a fixed synthetic paired table rather
than a stochastic solver result:

```text
full    = [-10, -20, -30, -40]
replace = [ -9, -18, -27, -36]
```

With `I = replace - full`, the exact paired effects are `[1, 2, 3, 4]`, the mean
is `2.5`, every sign is positive, and `d_seed` for the declared full sample is
the sample standard deviation of `full`.  The implementation must recover these
values to floating-point arithmetic precision and must not set
`DEGENERATE_NULL`.  A second table with constant `full` values must set
`DEGENERATE_NULL` instead of returning infinity.  This validates attribution,
materiality and null handling independently of Runtime behaviour.

### Synthetic operator positive

On an enumerated `n <= 20` instance, a reference implementation independently
computes the Metropolis and Gibbs transition decisions from the fixed seed's
draws and the exact local fields.  The production operators must produce the
same post-step states and canonical energies, and the harness's `I_replace_work`
must equal the reference difference exactly.  The instance/seed is accepted as
a positive control only if this pre-analysis reference difference is non-zero;
that non-zero fixture is committed before any Gate-A science seeds are run.

This is a construction/validation fixture, not a searched scientific contrast,
and is excluded from multiplicity accounting.

### Real positive

The fresh deletion CI remains a Runtime-pipeline positive control, but is named
honestly: it validates reproduction of the existing Theory Engine protocol, not
replacement attribution.  The synthetic operator positive above is the blocking
positive control for the replacement path.  Both must pass Gate A.

---

## A5. Multiplicity family

The primary `G22/legacy` contrast is tested at the pre-declared two-sided
`alpha = 0.05` without multiplicity adjustment.  Benjamini-Hochberg at
`FDR = 0.10` is applied to the **11 secondary contrasts only**.  The primary is
reported alongside the adjusted table but is not inserted into the secondary
BH family.

For the paired randomisation test with eight seeds, use the exact two-sided
sign-flip distribution over all `2^8 = 256` assignments.  The statistic is the
absolute paired mean.  Ties are retained; the p-value is

```text
count(|T_perm| >= |T_obs|) / 256.
```

No Monte-Carlo approximation is used.

---

## A6. Held-out replication rule

Gate A condition 5 is made numerical.  Let `I_in` and `I_out` be the held-in and
held-out paired means for the contrast that satisfied condition 4.  Magnitude
and sign replicate only if all are true:

1. `sign(I_out) = sign(I_in)`;
2. the held-out effect independently satisfies both materiality thresholds from
   §6 (`rho_I >= 0.5` and relative magnitude `>= 0.1%`), with the same
   `DEGENERATE_NULL` rule;
3. `0.5 <= |I_out / I_in| <= 2.0`.

If `I_in = 0` or either arm is `DEGENERATE_NULL`, replication fails.  Held-out
p-values are descriptive and are not used to renegotiate the selected contrast.

---

## A7. Gate A controls, as amended

Gate A conditions 1–3 are read with these replacements:

1. synthetic arithmetic positive and synthetic operator positive both pass;
2. the real deletion positive reproduces inside its freshly established CI;
3. null and both inert controls are exact, the direct draw-alignment probe
   matches, and the deliberate shift is detected.

Conditions 4–6 remain as written, subject to A5 and A6.  Gate B condition 3 is
specifically the frozen-calibration `I_replace_cost` robustness endpoint from
A1.  Failure of any amended blocking control invalidates the instrument; it is
not evidence for or against H-14.

---

## A8. Scope preserved

This amendment authorises no Runtime change, no general addressable-RNG claim,
no new sensor, no Foundry/DSL work and no dataset growth.  It narrows claims to
what the current seams can actually observe and makes every Gate-A decision
machine-checkable before Phase 1 begins.
