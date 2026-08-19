# PREREG — RC-020 Amendment 1: the control seed, frozen

**Status:** binding amendment to `PREREG_RC020_MARGINAL_WALL_COST.md`. Written
**before any RC-020 pilot execution**, before any pilot datum exists, and before
any artifact has been written to `experiments/rc020/`. Documentation only: no
code is changed by this commit, and no control, pilot or science run was
performed.

**Reason.** The pre-registration's §2 names a pilot block, a held-in block and a
held-out block, and §4 requires controls that execute on a real instance. It
names **no seed for the controls to use**. The instrument therefore had to choose
one, and `src/bin/exp_marginal_cost.rs` (commit `01f04b1`) recorded the omission
in source as a `RECORDED GAP` and used `CONTROL_SEED = 20001` provisionally.

A seed the instrument picked is not a pre-registered seed. This amendment freezes
it, proves it collides with nothing, and adds the requirements that keep it from
drifting.

## A1 — The frozen value

```
CONTROL_SEED = 20001
```

It is frozen for the whole of RC-020 and may not be changed, extended into a
block, or supplemented by a second control seed without a further amendment.

Two **control-internal** deterministic seeds are frozen at the same time, since
they are equally unnamed by §2 and equally load-bearing for replay:

```
SYNTHETIC_CONTROL_SEED  = 991   // P1/N2 synthetic observation generator
CONTROL_BOOTSTRAP_SEED  = 993   // N2 percentile bootstrap
```

These three values, together with §2's `CI_BASE_SEED` and `PERM_BASE_SEED`,
are now the complete set of seed-valued constants in RC-020. No other seed may
be introduced anywhere in the cycle.

## A2 — Disjointness, proved exhaustively

Every seed-valued quantity RC-020 names, enumerated in full:

| family | values | count |
|---|---|---|
| prior forbidden blocks (`FORBIDDEN_LO[i] … +7`) | `1001–1008, 2001–2008, 3001–3008, 4001–4008, 5001–5008, 6001–6008, 7001–7008, 8001–8008, 9001–9008` | 72 |
| RC-020 pilot | `10001–10008` | 8 |
| RC-020 held-in (reserved) | `11001–11008` | 8 |
| RC-020 held-out (reserved) | `12001–12008` | 8 |
| CI bootstrap, `20261201 + 2·corpus_index + arm_code` over corpus indices `{0, 3, 6, 9, 12, 17}` and arms `{A=0, B=1}` | `20261201, 20261202, 20261207, 20261208, 20261213, 20261214, 20261219, 20261220, 20261225, 20261226, 20261235, 20261236` | 12 distinct |
| permutation, `20261301 + repetition_index`, `repetition ∈ 0…8` | `20261301–20261309` | 9 |
| control-internal (A1) | `991, 993` | 2 |
| **total named seed values** | | **119** |

**Result, computed over the enumerated set:**

- `20001` lies in **no** forbidden block — the forbidden span ends at `9008`.
- `20001` lies in **no** RC-020 block — the highest reserved value is `12008`.
- `20001` equals **no** CI bootstrap seed — the lowest is `20261201`.
- `20001` equals **no** permutation seed — the lowest is `20261301`.
- `20001` equals **neither** control-internal literal.
- Membership test against the full 119-value set: **no collision.**

**Margins**, so the value is not merely disjoint but not adjacent:

```
nearest named value below : 12008      gap 7,993
nearest named value above : 20261201   gap 20,241,200
```

The lower gap is larger than every RC-020 block put together, so no plausible
extension of a block — even a tenfold one — reaches `20001`. The upper gap makes
collision with the bootstrap or permutation families impossible by construction.

`991` and `993` are likewise disjoint: they lie below the forbidden span's first
value `1001`, and are distinct from each other.

## A3 — What the control seed may and may not do

**May:** seed the Runtime-facing controls of §4 — N1, N3, P3, P4 — and the warmup
trajectory of §3.3.

**May NOT — binding:**

- it may **not** appear in any recorded observation. Control executions are not
  observations, are not written to the §10.1 artifact, and contribute to no
  estimate;
- it may **not** be used by the pilot body, which uses `10001–10008` and nothing
  else;
- it may **not** be expanded into a block. RC-020's controls use exactly one
  seed, so a control result is a single-seed check and is reported as such —
  it is a check on the instrument, never evidence about marginal cost;
- it may **not** substitute for a pilot seed if a pilot seed is later judged
  unusable. That situation requires an amendment, not a swap.

## A4 — Provenance and frozen-constant requirements, updated

§10's gate list is extended. Each item is a **gate**, refusing with a non-zero
exit and naming the failed condition; none may be a warning.

Renumbering nothing, the following are added:

8. **Control-seed disjointness is checked at run time, not assumed.** Before any
   control executes, the instrument verifies that `CONTROL_SEED` is a member of
   none of the six families of §A2, and that `SYNTHETIC_CONTROL_SEED` and
   `CONTROL_BOOTSTRAP_SEED` are likewise members of none. A collision is a
   **Class I** condition (§8, instrument invalid), not a Class II null.

9. **The frozen-constant test covers the seed set.** The instrument's
   frozen-constants test asserts all three values of §A1 alongside §2's
   constants, so a change to any of them fails the test suite rather than
   silently altering what the controls did.

10. **The artifact records the control seed.** The results document — not the
    §10.1 per-observation schema, which stays at its 22 columns — records
    `CONTROL_SEED` and both control-internal seeds in its provenance header,
    so a reader can replay the controls exactly.

**§10.1's schema is unchanged.** No column is added, removed or reordered; the
control seed is session-level provenance, not per-observation data, and the
22-column schema is frozen as written.

## A5 — What this amendment does not change

Nothing else in `PREREG_RC020_MARGINAL_WALL_COST.md` is altered:

- §1's scope, and the exclusion of efficacy and equal-cost claims;
- §2's corpus, operators, temperatures, windows, arms, repetitions and all three
  seed blocks;
- §5's numerically frozen bounds — drift `0.09`, degeneracy `100 ×` timer
  resolution, load `2.0`, discard limit `2 of 9`;
- §6's estimators; §7's `A1 ≤ 0.10`, `A2 ≤ 0.10` and the feasibility widths;
- §8's Class I / Class II separation;
- §9, which remains **blank and frozen**, so held-in and held-out stay
  unreachable;
- §11's implementation plan.

No threshold is moved by this amendment.

## Facts verified for this amendment

| claim | how verified |
|---|---|
| §2 names no control seed | read of `PREREG_RC020_MARGINAL_WALL_COST.md` §2 |
| the instrument used `20001` and recorded the omission | `src/bin/exp_marginal_cost.rs`, `CONTROL_SEED` doc comment, commit `01f04b1` |
| the six families enumerate to 119 distinct values | computed over the enumerated set |
| `20001` collides with none of them | membership test over the full set |
| gaps are 7,993 below and 20,241,200 above | computed |
| `991` and `993` collide with nothing | computed; both below `1001` |
| no pilot artifact exists yet | `experiments/rc020/` absent at the time of writing |
