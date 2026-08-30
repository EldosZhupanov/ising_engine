# Incremental relinking — the project's first wall-time claim, and the protocol that earned it

**Date:** 2026-08-30. **Kind:** engineering record, not a research cycle. No
scientific claim is made or changed here.

## Why this target

RC-026 closed the architecture line and named its own blocker while doing so:

> `relink_source` walks a path of length equal to the Hamming distance `d` and
> re-evaluates every still-differing site at every step: **Σᵢ₌₁..d i = d²/2**
> delta-energy evaluations per source per round. Embedding multiplies an already
> quadratic cost by `k`.

That is a cost, not a mystery. Flipping one site changes the local field only at
its **neighbours** — `h_i = linear[i] + Σⱼ qᵢⱼ xⱼ` moves only where `qᵢⱼ ≠ 0` —
so every non-adjacent site's delta is unchanged and was being recomputed for
nothing.

## The change

`relink_source` now evaluates each differing site once at setup, keeps the
values in a cache, and after each flip refreshes **only the flipped site's
still-differing neighbours**. The pick is an argmin over cached values with the
same lowest-index tie-break as before.

Cost falls from `d²/2` to `d + Σ_steps deg(picked)` — from quadratic in the
Hamming distance to linear in it times the average degree.

The adjacency comes from `SpinState::neighbors`, the topology-only accessor the
cluster operators already use. **Nothing in the read-only core changed**, no new
dependency, no new API, no `unsafe`.

## Bit-identity, proven three ways

The optimisation is only legitimate if it changes nothing observable. It does not.

**1. A golden record captured before the change.** `the_greedy_walk_is_bit_identical_to_its_golden_record`
pins the exact path, walk length, retained prefix and resulting energy as
constants — not tolerances — on a 40-variable frustrated ring. All four survive:

| quantity | before | after |
|---|---|---|
| walk length | 21 | 21 |
| retained prefix | 18 | 18 |
| path | `[30, 3, 2, 5, 8, 11, 27, 16, 15, 26, 6, 7, 33, 34, 10, 37, 24, 25, 38, 39, 21]` | identical |
| energy | −16.0 | −16.0 |
| **delta evaluations** | **231** | **46** |

**2. The published RC-024 artifact, reproduced byte for byte.** Re-running the
frozen RC-024 command with the optimised binary produces a file whose SHA-256 is
`7a136e595732b8b3cb48f78aeb0d19ed91b0edefb900483b7abddc98ece49289` — the value
published in `results/rc024/RESULT.md`. 90 rows, 30 instances, 3 seeds.

**3. The published RC-026 artifact, reproduced byte for byte.** Same for the
embedding ladder: SHA-256
`ac91d6bf8d3de316df48f83e716e13b6983d05ae09f9daf802337533696726f3`, 450 rows, 30
instances, 5 embedding depths, 3 seeds.

**540 registered rows across two preregistrations reproduce exactly.** That is a
stronger statement than any test suite could make, because those numbers were
published before the optimisation existed.

## The one thing that did change, declared

`Report.work` falls, because the operator genuinely does less work. This is not
cosmetic: `work` reaches `lab.rs`'s `fitness(score, work, …)`, so **evolved plans
in the `--orchestrate` loop can differ**. It is declared rather than hidden.

It is a correction, not a regression. `OPEN_PROBLEMS.md` §4 (RC-005) records that
the operator cost model omits its dominant variable; reporting a count the
operator no longer performs would have made it wronger. Every registered artifact
in this repository records energies only, which is why all three verifications
above are possible at all.

## The measurement

Three interleaved repetitions of the RC-024 command, alternating the saved
pre-change binary with the new one so drift hits both arms alike:

| rep | before (s) | after (s) |
|---:|---:|---:|
| 1 | 113.03 | 20.03 |
| 2 | 119.91 | 19.63 |
| 3 | 112.70 | 19.75 |
| **median** | **113.03** | **19.75** |

**Speedup 5.72×** — the run takes 17.5 % of the time it did.

**This host is entitled to that claim, and we know it because we measured.**
`research/RC021_C10_DIAGNOSIS.md` established that this machine resolves a paired
wall-time difference of ~25 % cleanly, ~5 % marginally, and 1 % not at all, with
two identical arms differing by up to 3 % at p90. The within-arm spread here is
6.4 % and 2.0 %, consistent with that null. An 82.5 % difference sits two orders
of magnitude above it. **This is the first wall-time claim this project has been
entitled to make**, and the entitlement came from calibrating the host before
making it — not from hoping.

Separately, and as an **uncontrolled single observation** rather than a
measurement: the RC-026 ladder took over an hour before the change and **375 s**
after. The ratio is larger there because relinking dominates more of that run at
`k = 25`. It is reported because it is operationally the point — the harness that
made this line unaffordable to explore is now cheap — not because one timing pair
is evidence.

## What this does **not** do

**It does not reopen the architecture line.** RC-026's killer was the exponent,
not the constant: `gain ∝ k^0.18` means a 1 % mean gain needs of order `3.6 × 10⁵`
rounds, and making each round ~6× cheaper buys none of that back. The line stays
closed.

No claim about `UltimateSolver`, production routing, or any solver's quality.
Every energy this repository has published is unchanged, by construction and by
three independent verifications.

## The protocol this establishes

For any future wall-time work in this repository:

1. **Calibrate the host first.** `cargo run --release --bin host_timing_calibration`
   reports what a paired comparison can resolve here, with a null arm and an
   injected positive control. A threshold chosen without it is a wish.
2. **Keep the pre-change release binary** and interleave the arms, so drift is
   shared rather than attributed.
3. **Prove bit-identity against something that was published before the change.**
   A test you wrote after the change tests what you already believed.
4. **Declare what did change**, however diagnostic it looks. `Report.work` looked
   diagnostic and reaches an evolutionary fitness function.
