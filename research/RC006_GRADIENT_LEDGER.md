# RC-006 — The gradient-ledger invariant

**Status: CONFIRMED (exact identity, bit-exact on 5 instance families).**
Promoted to a permanent test.

## Problem

`SpinState` maintains **two ledgers** that are updated independently on every
flip: the per-replica `energies`, and the field ledger read by `delta_e_into`.
Divergence between them is a silent correctness failure. `audit()` catches it by
rebuilding everything from scratch — **O(E) per call**, which is why it runs in
tests and verification passes rather than during a run.

Nothing cheaper existed, so no cross-check was affordable at run frequency.

## Hypothesis

The two ledgers are algebraically linked, and the link permits an **O(n)**
consistency check.

## Mathematical reasoning

For `E(x) = offset + Σᵢ aᵢxᵢ + Σ_{i<j} q_ij xᵢxⱼ` over x ∈ {0,1}, flipping site i
gives `ΔEᵢ = (1−2xᵢ)·hᵢ` with local field `hᵢ = aᵢ + Σⱼ q_ij xⱼ`. Summing:

    Σᵢ ΔEᵢ = Σᵢ hᵢ − 2Σᵢ xᵢhᵢ
           = (A_tot + Σⱼ dⱼxⱼ) − 2(L(x) + 2Q(x))
           = A_tot + Σⱼ dⱼxⱼ − 2·L(x) − 4·Q(x)

with `A_tot = Σaᵢ`, weighted degree `dⱼ = Σᵢ q_ij`, `L(x) = Σaᵢxᵢ`, and `Q(x)`
the quadratic part. Since `E = offset + L + Q`, solving for Q:

    **E = offset + L(x) + ( A_tot + Σⱼ dⱼxⱼ − 2·L(x) − Σᵢ ΔEᵢ ) / 4**

`A_tot` and `dⱼ` are per-instance constants (one-off O(E)); every later check is
**O(n) per replica with no edge traversal**.

## Experimental setup

Harness `src/bin/exp_gradient_invariant.rs` — deliberately a *falsification*
harness: the derivation was done by hand, and a wrong constant would appear as a
systematic residual rather than noise.

Deterministic pseudo-random states; both `ReferenceState` (f64 oracle) and
`SparseBitSlice` (exact integer), since a backend-specific identity would not be
an invariant of the model.

## Dataset

Five structurally distinct instances, chosen so a factor error could not hide:

| instance | character |
|---|---|
| `biqmac/be100.1.sparse` | **145 distinct weights, −100…100** — the decisive weighted test |
| G1 | dense unweighted |
| G11, G32 | toroidal ±1 grids |
| G22 | dense unweighted |
| `dimacs_maxcut/sg3dl051000.mc` | 3D lattice spin glass |

## Measurements

**200 checks. Worst absolute residual: `0.000000e0`** — bit-exact, not merely
within tolerance, on both backends and every family. Also holds under 200
sequential flips, with `audit()` agreeing at 0.0.

## Statistical evidence

None required and none claimed: this is an exact algebraic identity, not a
statistical result. The evidence is the zero residual.

## Counterexamples / refutations

None found. Unweighted MaxCut alone could have concealed a factor error, which is
why the weighted biqmac instance was included — it did not.

## Final conclusion

The identity holds. The energy ledger is reconstructible from the gradient ledger
in O(n) after an O(E) one-off.

## Practical consequences

Promoted to `tests/test_gradient_ledger_invariant.rs` (3 tests): mixed-sign
instance on both backends, survival under 200 flips, and a real G-Set instance.

**Stated limitation, in the test file so it is not over-trusted:** this is a
*scalar aggregate*, therefore a **necessary but not sufficient** condition — two
compensating errors in different `ΔEᵢ` cancel in the sum. It does **not** replace
`audit()`. Its value is being affordable at a frequency `audit()` is not.

## Open questions

- Should it become a debug-only runtime assertion inside the Runtime? Not done:
  that touches the read-only core hot path and needs a bit-identity A/B.
- Does an analogous identity exist for HUBO (order ≥ 3)? Underived.

## Reproduction

```bash
cargo run --release --bin exp_gradient_invariant -- --file benchmark_suite/data/gset/G1
cargo test --release --test test_gradient_ledger_invariant
```
Expected: `worst absolute residual: 0.000000e0`, `VERIFIED`; 3 tests pass.
