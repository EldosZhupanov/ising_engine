# RC-014 Phase 0 — Feasibility audit

**Date:** 2026-08-19 · **Method:** source reading + corpus inspection. **No code
was changed and no experiment was run.** Every claim below cites the file:line or
the command that produced it.

**Headline:** the general instrument is **blocked** — `engine_v2` consumes
randomness as a single stream, so removing or replacing a step desynchronises
every later draw. But a **narrow path is open today with zero core changes**:
`metropolis_sweep` and `gibbs_color_sweep` consume *exactly* the same number of
draws of the same type, so substituting one for the other leaves the stream
byte-aligned. Phase 1 proceeds in that restricted form; general addressability is
deferred to an ADR.

Two audit items returned answers that **contradict the plan and other repository
documents**, and both are corrected here (§0.3, §0.5).

---

## 0.1 — Does the Runtime preserve random-event correspondence under
## step removal/replacement? **NO (blocking, with a narrow exception)**

`runtime.rs:196` — one generator per run:

```rust
let rng = ChaCha8Rng::seed_from_u64(plan.seed);
```

`runtime.rs:296` — the same generator, by `&mut`, through every operator:

```rust
let report = op.apply(state, &view, &mut self.rng, step.budget);
```

This is a **pure stream**. Draw position depends on everything consumed before,
so deleting a step, or replacing it with an operator of a different draw count,
shifts the randomness of every subsequent step. The comparison would then be a
different random realisation — i.e. exactly the seed-variance null RC-007 uses,
not a counterfactual.

**The exception, and it is usable.** The project already practises intra-operator
draw-count stabilisation for the same reason. `metropolis_sweep.rs:119`:

```rust
let accept = if d <= 0.0 {
    // still draw to keep the RNG stream aligned across configs
    let _u: f64 = rng.gen();
    true
} else { ... }
```

Measured draw counts per `apply` call:

| Operator | Draws per call | Type | Content-dependent? |
|---|---|---|---|
| `metropolis_sweep` | `sweeps · n · r` | `f64` | **no** (aligned by `_u`) |
| `gibbs_color_sweep` | `sweeps · n · r` | `f64` | **no** (`gibbs_color_sweep.rs:152`, unconditional) |
| `random_flip_sweep` | `sweeps · n · r` | `bool` | no, but **different width** |
| `replica_exchange` | `sweeps · ⌊(r − parity)/2⌋` | `f64` | no (depends only on `r`, round parity) |
| `greedy_descent`, `steepest_descent` | 0 | — | no (`_rng`) |
| `cluster`, `population_annealing`, `move_synthesis`, `extremal_*`, `history_field` | variable | mixed | **yes** |

`rand 0.8.5` + `rand_chacha 0.3.1`: `f64` consumes `next_u64` (two 32-bit words),
`bool` consumes `next_u32` (one word). So **`bool` and `f64` operators are not
interchangeable** even at equal counts.

**Verdict.** `metropolis_sweep ↔ gibbs_color_sweep` at equal `sweeps` is an
**exactly draw-aligned substitution**: identical count, identical width, so the
stream state after the step is identical and all downstream randomness is
preserved. This is a valid exact counterfactual **today**, without touching
`runtime.rs`.

**Deferred to ADR.** General addressable randomness
(`U = f(seed, run, phase, logical_slot, operator, site, replica, draw_kind, draw_index)`)
requires the Runtime to derive a per-step sub-generator instead of threading one
`&mut ChaCha8Rng`. That changes the read-only core's contract and every operator's
replay identity. It is not done as part of RC-014.

---

## 0.2 — Does `ExperimentDb` record the code version? **NO**

`db.rs:13–48`, 22 fields: `id, hypothesis_id, sequence, sweeps, temp_hi, temp_lo,
num_replicas, seed, backend, work, score, baseline, density, clustering,
instance_id, n, mean_degree, degree_cv, wall_ms, campaign_id, generation_id,
timestamp`.

| Required (plan §2.1) | Present |
|---|---|
| seed · backend | ✅ |
| plan (ops, sweeps, temps, replicas) | ✅ (phases/repeats implied — see 0.4) |
| git commit · dirty-tree · patch hash | ❌ |
| crate / rustc version · target features | ❌ |
| registry version · operator ID set | ❌ (operator *names* only) |
| RNG algorithm/version | ❌ |
| **instance content hash** | ❌ (`instance_id` is a name string) |
| lowering/pass versions · thread layout | ❌ |

**Consequence (the plan's recorded fallback applies).** The historical corpus is a
source for **statistical analysis of stored results**, not for guaranteed rerun
candidates. `timestamp` allows correlating a row with a commit date, but that is
inference, not provenance. Rerun candidates for RC-014 are drawn **only from rows
written by the current code version**.

Note also that `instance_id` being a name and not a content hash means a changed
benchmark file is silently undetectable.

---

## 0.3 — Canonical corpus: **the 109,758 figure is a triple-count.
## The distinct total is 72,618.**

`platform_gset` is a **byte-identical prefix** of both other campaigns:

```
first 18,570 lines, md5:
  platform_gset      186297d730b2caa1780bf785abf7f44a
  platform_grow      186297d730b2caa1780bf785abf7f44a
  platform_grow_div  186297d730b2caa1780bf785abf7f44a
```

Common prefixes (full-line equality):

| Pair | Common prefix | Lengths |
|---|---|---|
| gset vs grow | 18,570 | 18,570 / 48,653 |
| gset vs grow_div | 18,570 | 18,570 / 42,535 |
| grow vs grow_div | 18,570 | 48,653 / 42,535 |

```
18,570  shared prefix
30,083  platform_grow      tail (unique)
23,965  platform_grow_div  tail (unique)
──────
72,618  distinct rows          (naive sum: 109,758 — overstated by 37,140, +51%)
```

**Worse: record `id` is unique only within a campaign directory.** In the two
tails, **23,965 ids collide** between `platform_grow` and `platform_grow_div`, and
**zero of those rows are identical** — the same id denotes a different experiment
in each campaign. Any union keyed on `id` silently merges distinct experiments.

**Consequence.** `COMPETITIVE_ANALYSIS.md` ("109,758 runs"), the website Mission
Control readout, and `YC_APPLICATION_FALL_2026.md` ("110,000 experiments") all
quote the inflated figure. In an honesty-first product this is the expensive kind
of defect. **Canonical corpus for RC-014: `platform_gset`, 18,570 rows** — the
shared, unambiguous base; the two tails are usable only with a
`(campaign, id)` composite key.

*Filed as a correction, not a cover-up: the number was a sum of directory line
counts, and the directories turned out to be nested, not disjoint.*

---

## 0.4 — Are stored plan/instance/seed data sufficient to re-execute a row? **YES,
## conditional on 0.2**

`executor.rs:56–79`, `lower()` is a total deterministic function of the row:

- `phase: Phase::Exploit` for **every** step, `repeat: 1` — so phases and repeats
  are not free parameters and need not be stored;
- `temperatures = geometric_ladder(num_replicas, temp_hi, temp_lo)` — recoverable
  from three stored fields;
- `backend = DecisionEngine::analyze(ir).select_backend()` — derived from the IR,
  and the row's stored `backend` is a cross-check;
- initial state `vec![0u8; ir.n]` (`executor.rs`, `run_one`) — the all-zeros init
  RC-002 documented, still in force.

**Verdict:** the mapping row → plan is exact. Re-execution is valid **within one
code version only**, which 0.2 shows the row cannot identify.

---

## 0.5 — What does `--investigate` compute today?
## **`I_delete`, not `I_replace` — this corrects the plan**

`theory.rs:315–329`:

```rust
let full = self.mean_best(ir, registry, schedule, seeds);
let ablated_ops: Vec<String> = schedule.ops.iter()
    .filter(|o| o.as_str() != operator).cloned().collect();
let ablated = Schedule {
    sweeps: vec![schedule.sweeps.first().copied().unwrap_or(16); ablated_ops.len()],
    ops: ablated_ops, ...
};
```

The operator's steps are **removed**. Commit `8ec5c43` ("make `--investigate` a
marginal-contribution test, not vs-nothing") made the ablated arm retain the
*rest of the schedule* rather than being empty — that is deletion against a
non-empty remainder, **not** cost-matched replacement.

`RC014_PLAN_CAUSAL_INSTRUMENT.md` §3.2 states "`I_replace` … is the semantics
`--investigate` already adopted." **That is wrong and is corrected here.** The
replacement arm does not exist and must be built.

**A second confound, previously unrecorded.** `theory.rs:324` rebuilds the ablated
schedule with the **first** operator's sweep count applied to *all* remaining
operators. If the original schedule had heterogeneous sweeps, the ablated arm
differs from the full arm in more than the removed operator. `investigate_operator`
(`orchestrator.rs:591`) uses `sweeps: vec![16, 16]`, so its own path is unaffected
— but any caller with heterogeneous sweeps is.

---

## 0.6 — Can transition observation be added without violating the read-only
## core? **YES**

Two seams already exist:

1. **`RunRecord.events: Vec<StepEvent>`** (`runtime.rs:53–66`) is returned from
   every run and already carries, per step: `iteration`, `operator`,
   `acceptance`, `best_energy`, and `QualityMetrics { best_energy, mean_energy,
   energy_entropy, diversity }`, plus the `decision` string.
2. **`RunController` + `StepSensors`** (`runtime.rs:88–118`) is a declared
   plug-in point called after every step. A recorder implementing it and always
   returning `RunControl::Continue` observes every step and **changes nothing** —
   bit-identical by construction, above the core, no core edit.

**Available today:** `S_t` ⊇ {iteration, frac_elapsed, operator, acceptance,
best_energy, mean_energy, energy_entropy, diversity}; `Y_{t:H}` is computable
post-hoc from the `best_energy` series in `events`.

**Missing:** per-step **cost** (`ctx.profiler` aggregates by operator *name*, not
per step) and **replica overlap**. Both are additions above the core.

---

## 0.7 — Can the historical `metropolis_sweep` ≈5% ablation be reconstructed?
## **The "≈5%" is not a constant. Use the best-powered recorded row instead.**

Recorded ablation degradations for `metropolis_sweep`, from
`experiments/*/knowledge_graph.txt`:

| Corpus / condition | Degradation | Verdict | Trials |
|---|---|---|---|
| gset, `density>=0.05` | **+8.1%** | UPHELD | **27/30** |
| gset, `density<0.05` | +3.7% | UPHELD | 5/5 |
| grow, `density<0.05` | +4.8% | UPHELD | 1/1 |
| grow, `density<0.05` | +0.6% | **REFUTED** | 0/1 |
| grow, `density>=0.05` | +0.9% | **REFUTED** | 0/1 |
| grow_div, `density<0.05` | **+100.0%** | UPHELD | 1/1 |

The same operator is recorded as both causal and spurious depending on instance
and campaign. `ROADMAP.md`'s "ablation costs 5%" is a **selected value from a
distribution spanning 0.6% to 100%**, not a stable property.

The +100% rows are the deletion confound in its purest form: `ablated best 0.00`
means the ablated schedule contained no optimiser at all and left the state at
all-zeros. That measures "the schedule became empty", not the operator's
contribution — and it is precisely why §0.5's `I_replace` matters.

**Control target adopted:** the **gset `density>=0.05`** row — +8.1% degradation
at 27/30 trials, the only well-powered entry. The instrument must reproduce that
*paired* effect within a CI established by re-running the same protocol
(`schedule = [metropolis_sweep, greedy_descent]`, `sweeps = [16, 16]`,
`temp_hi = 4.0`, `temp_lo = 0.1`, per `orchestrator.rs:591–596`) with the current
code. This is the plan's fallback for 0.7 — self-consistency, not historical
identity — and it is now the primary path, since the historical seed lists are
not recorded (0.2).

---

## 0.8 — Sensor maps: **FROZEN**

**`S_0` — what the Decision Engine / learned models actually read.**
`FeatureRegistry::v0()` (`feature_registry.rs:46–72`), version 0, marked FROZEN in
source, 5 features with exact transforms:

| # | Name | Transform |
|---|---|---|
| 0 | `log_n` | `ln(n + 1) / 10` |
| 1 | `density` | `density` |
| 2 | `clustering` | `clustering` |
| 3 | `mean_degree` | `mean_degree / 10` |
| 4 | `degree_cv` | `degree_cv` |

Backed by `InstanceSignature { n, density, clustering, mean_degree, degree_cv }`
(`predictor.rs:16–22`), computed by `InstanceStats::analyze` (`decision.rs:20–34`,
which additionally holds `num_pairs` and `integral`, not exposed to `S_0`).

**`S_1` — Runtime/controller sensors.** `StepSensors { iteration, frac_elapsed,
operator, best_energy, metrics, acceptance }` + `QualityMetrics { best_energy,
mean_energy, energy_entropy, diversity }` + `RuntimeView { iteration,
temperatures, num_replicas, recent_acceptance, remaining_ms }`.

**`S_2` — the permitted minimal extension, declared now, before any result:**
mean pairwise replica overlap `q`; per-step measured cost; acceptance split by
temperature rung. Nothing else may be added to `S_2` after results are seen.

---

## Verdict and consequences

| Item | Answer | Consequence |
|---|---|---|
| 0.1 | **NO**, narrow exception | Phase 1 restricted to draw-count-matched substitution; general addressability → ADR |
| 0.2 | NO | Old corpus = statistics only, not rerun candidates |
| 0.3 | **Corrected: 72,618 distinct, not 109,758** | Canonical corpus = `platform_gset` (18,570); fix the three documents quoting the inflated figure |
| 0.4 | YES, within one code version | Rerun path valid for current-version rows |
| 0.5 | **Corrected: `I_delete`, not `I_replace`** | The replacement arm must be built; plan §3.2 amended |
| 0.6 | YES | `RunController` recorder, bit-identical, above the core |
| 0.7 | "≈5%" is not a constant | Control target = gset `density>=0.05` +8.1% (27/30), re-established with current code |
| 0.8 | Frozen | `S_0`/`S_1`/`S_2` fixed in this file |

**Phase 0 gate: PASS, restricted.** Phase 1 proceeds with
`metropolis_sweep ↔ gibbs_color_sweep` as the exactly draw-aligned contrast. The
general instrument is blocked behind an ADR on the Runtime randomness contract and
is out of scope for RC-014.

### Follow-ups this audit generated (not part of RC-014)

1. Correct the experiment count in `COMPETITIVE_ANALYSIS.md`, the website ingest,
   and `YC_APPLICATION_FALL_2026.md`: 72,618 distinct, or 18,570 for the canonical
   G-Set campaign. Quoting 109,758 triple-counts a shared prefix.
2. Add a composite `(campaign, id)` key, or campaign-scoped id ranges, to prevent
   silent id collisions across campaign directories.
3. Add engine-version provenance to `ExperimentRecord` (git commit + registry
   version at minimum), without which no row is a guaranteed rerun candidate.
4. `theory.rs:324` overwrites per-operator sweeps with the first operator's count
   when building the ablated schedule — a confound for heterogeneous schedules.
5. `ROADMAP.md`'s "ablation costs 5%" should cite the distribution and the
   condition, not a single selected figure.
