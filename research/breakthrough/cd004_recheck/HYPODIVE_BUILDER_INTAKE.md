# CD004-R intake

Mode C: research evidence. The original CD004 witness returns an incumbent of
-10 for a zero-field complete graph with `N=8, seed=1`, while exhaustive search
and chronological branch-and-bound return -12. Its seven published cases all
start from optimal incumbents, so their energy agreement does not validate
backjumping. This was discovered before this iteration and is calibration data,
not a confirmatory result.

Objective: determine whether explicit learned soft-conflict cores can preserve
exactness and produce useful work reduction against a matched chronological
branch-and-bound. The strongest admissible claim is limited to the tested small
integer Ising instances; node count alone is not a speed claim. The old
backjumping function and its historical records are untouched.

Canonical path: `research/breakthrough/cd004_recheck/instrument.py`, importing
the historical energy and certified lower-bound functions. One shared DFS owns
both arms; the candidate additionally learns and checks core assignments. The
exact enumeration oracle is independent of both searches. No production source,
solver family, public API or dependency is changed.

Invariants: every reported incumbent has a spin witness; every pruned node has
a certified lower bound at least the incumbent; a learned core is independently
rechecked against its saved threshold; all returned energies equal exhaustive
ground states. Any failure stops the utility analysis. Existing known examples
are smoke tests only. The untouched seed block and analysis rules are in
[`PROTOCOL.md`](PROTOCOL.md). If correctness passes but time does not improve,
the result is a valid exact prototype without an acceleration claim.
