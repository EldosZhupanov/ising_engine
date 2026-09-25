# EXP002: falsify the EXP001 instrument and easy-family interpretation

Date: 2026-09-12. Binding on commit before validation code or new executions.
This is a post-EXP001 correctness/structure audit, not independent performance
replication. Its selection is explicitly motivated by [EXP001_RESULT.md](EXP001_RESULT.md).
Frozen EXP001 data and source remain untouched.

1. Independently regenerate all 45 original instances in Python from the exact
SplitMix64 specification, compare fingerprints, and directly score all 4,200
saved assignments and all 45 target assignments using native Ising edges/fields.
For n=16 retain EXP001 exhaustive optimum, checking that no saved energy is below
it. Do not construct new targets or modify old ones.
2. Derive and implement independent exact cycle and subdivided-family oracles.
For a zero-field three-edge path the response is offset=-sum(abs(J))+min(abs(J))
and effective coupling=product(sign(J))*min(abs(J)). Group opposite core vertices
into four-state rung variables, enumerate each initial rung state, propagate
minimum costs along both rails, then close the two twisted boundary bonds.
Compare these oracles with all six-family/size n=16 stored exact targets (cycle
and subdivided, three instance seeds each). For every registered size and seed
in these two families report the true optimum, target gap and each arm's saved
optimum success probability. This is post-hoc diagnostic truth, not replacement
of the registered target or recomputation of its continuation screen.
3. Replay all 3,600 EXP001 fixed-exchange rows once using the exact preserved
probe binary, with original frozen arguments and RAYON_NUM_THREADS=1, sequential.
Verify its SHA256 against exp001/source.sha256 before launch. Compare every
non-time field exactly: assignment, energy, success, residual sizes, work counters,
observations, and refinement gains. Exclude time_target_ms, time_best_ms,
elapsed_ms, prep_ms, generator_ms, audit_ms from equality; compare presence/absence
of time_target_ms censoring separately. No wall rows replayed; no timing claim.
A mismatch or execution failure stops the audit and is preserved, never silently
rerun. Do not rebuild/overwrite the original probe during this audit.

Output: research/breakthrough/exp002/ with a refusal to reuse an existing output
directory, validation source/hash, oracle tables, replay comparison log and closure.
Command: python3 research/breakthrough/validate_exp002.py --run.
Allow --check only for tiny synthetic chain/oracle identity tests without running
registered candidate seeds. Independent read-only review before --run.

Decision: any objective/fingerprint/replay mismatch blocks interpretation and
requires diagnosis; do not repair EXP001 artifacts. If all pass, record technical
reproducibility only. If the exact ladder oracle confirms that the family is easy,
classify H01's current success as bounded-width structural evidence and test new
random cubic cores later. Passing does not remove the short-restart confound.
