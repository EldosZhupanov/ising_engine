# RC-005 — Flip density: a complexity parameter the engine's own cost model omits

**Vector 2 (new complexity parameters).** Not a speed finding. The engine's
scheduler divides measured quality by a *declared* cost; that declared cost is
blind to a variable which accounts for 57–79% of the real cost, and the blindness
is state-dependent, so it biases allocation as a function of temperature.

---

## 1. The discrepancy, from source

Every `cost_model` in the 18-operator library is a pure function of instance
**shape**:

```rust
work_per_sweep: ((shape.n + 2 * shape.num_pairs) * shape.num_replicas.max(1)) as f64
```

No operator's cost model reads state, temperature, or acceptance.

But `SparseBitSlice::apply_flips` (`sparse_bitslice.rs:265`) dispatches on **flip
count**:

```rust
let flips: u32 = mask.words().iter().map(|x| x.count_ones()).sum();
if (flips as usize) * 3 >= self.r { self.apply_flips_dense(site, mask); }
                              else { /* scattered */ }
```

- scattered: `O(neighbors · flips)` — scales with acceptance
- dense: `O(neighbors · r)` — constant in acceptance

The source comment is explicit that this tracks "hot (high-acceptance) sites"
versus cold ones. **The backend knows cost depends on acceptance. The cost model
asserts it does not.** `runtime.rs:298` records the declared value into the
profiler, and the Decision Engine's utility plan (`α·q̂ + β·ĉ + …`) consumes it.

## 2. Pre-registered prediction

Define **flip density φ = flips/r**. Because each replica accepts independently,
E[flips]/r = acceptance rate, so φ *is* the acceptance rate.

- declared work: identical at every temperature (shape-only)
- actual ms: monotone in φ, spread **> 2×**
- **refuted if** actual ms is flat (< 1.2× spread); this host's same-code drift
  is ~9% (`PERF.md`)

Design guard: temperatures are **interleaved within each repetition** and the
median taken, so monotonic host drift lands on every temperature equally and
cannot manufacture a monotonic trend.

## 3. Result — confirmed, 4 instances

Harness `src/bin/exp_cost_model.rs`. `metropolis_sweep`, r=32, 20 sweeps.

| instance | φ range | ms range | declared work | **error spread** |
|---|---|---|---|---|
| G1 | 0.064 → 0.196 | 5.68 → 11.84 | constant | **2.09×** |
| G22 | 0.067 → 0.259 | 14.96 → 27.21 | constant | **1.82×** |
| G32 | 0.139 → 0.432 | 13.44 → 19.97 | constant | **1.64×** |
| G11 | 0.140 → 0.436 | 7.10 → 11.03 | constant | **1.61×** |

## 4. The critical point is visible in the data

G11 and G32 are the only instances whose φ crosses 1/3. On **both**, cost stops
rising and turns over exactly there:

| instance | φ = 0.351–0.354 | φ = 0.432–0.436 |
|---|---|---|
| G11 | 11.448 ms | **11.025 ms** |
| G32 | 20.543 ms | **19.974 ms** |

Past the threshold the dense path is taken and cost becomes independent of φ, so
further acceptance is free. This is the dispatch regime change, observed rather
than assumed.

## 5. The corrected cost law

    W(φ) = W_scan + W_flip · min(φ, ⅓)

| instance | fit | R² capped | R² **un**capped |
|---|---|---|---|
| G1 | ms = 3.72 + 43.04·min(φ,⅓) | 0.9322 | 0.9322 |
| G22 | ms = 12.01 + 62.57·min(φ,⅓) | 0.9550 | 0.9550 |
| **G11** | ms = 4.78 + 19.39·min(φ,⅓) | **0.9693** | 0.8943 |
| **G32** | ms = 8.85 + 34.90·min(φ,⅓) | **0.9426** | 0.8604 |

**The cap earns its place exactly where it binds.** On G1 and G22, which never
reach φ = ⅓, capped and uncapped fits are *identical to four decimals* — the
`min` is a no-op. On G11 and G32, which do cross, the cap lifts R² by 5–8 points.
A term that improves the fit only on the instances where theory says it should,
and is provably inert elsewhere, is a correct functional form rather than an
extra free parameter.

The flip term is **57–79% of total cost at saturation**: the omitted variable is
the dominant one, not a correction.

## 6. Consequence

The declared model is the corrected law with **φ ≡ ⅓ hard-coded** — it always
charges the saturated price. So it **overcharges by up to 2.1×, worst in the cold
regime**, where φ is smallest.

The Decision Engine ranks plans by utility per unit cost. Cold, low-acceptance
operation is therefore systematically penalised in budget allocation — precisely
the low-temperature refinement phase, and precisely the regime where
`state-dependent-operator-selection` says the best operator changes. The bias is
not noise; it is aligned with a real phase of the search.

## 7. Scope and honesty

- Measured on `metropolis_sweep` / `SparseBitSlice` only. Other operators share
  the same shape-only cost form, so the same blindness is expected, but is
  **not yet measured**.
- `DenseByte` and `ReferenceState` were not tested; the ⅓ threshold is specific
  to `SparseBitSlice`'s dispatch.
- Wall-ms is used as the cost proxy. On this host that is defensible only via the
  interleave-and-median design; absolute ms are not comparable across sessions.
- No fix is applied here. Changing `cost_model` alters plan ranking and therefore
  **trajectories**, which is behaviour-changing under ADR-0004 and needs explicit
  approval plus an A/B at identical seeds.

## 8. Reproduction

```bash
cargo run --release --bin exp_cost_model -- \
    --file benchmark_suite/data/gset/G11 --replicas 32 --sweeps 20 --reps 5
```
