# H01: experimental implementation

Implemented in [probe.rs](../probe.rs), linked against the unchanged production
release library at source `fdec0df`. [run_exp001.py](../run_exp001.py) builds,
checks and executes [EXP001_PROTOCOL.md](../EXP001_PROTOCOL.md);
[analyze_exp001.py](../analyze_exp001.py) reports all arms and the frozen screens.
No production source, dependency or API changes.

Exact conditional elimination records reversible local response rules. Pair
refinement uses edge cross-curvature after a common all-variable 1-opt finish.
Only a complete solve/lift/common/pair unit emits a target observation. Common
and extra refinement work are counted separately; baseline kernel evaluations
remain unavailable and nominal proposals are explicitly an upper bound.

Verification before candidate data: 9 Rust tests, including exhaustive
conditional energy equivalence on integer and non-integer small models,
all-pair delta identities, a genuine two-bit barrier, never-worse refinement,
independent native Ising scoring, deterministic replay, one observation per
unit and separate work accounting; 3 Python tests exercise all-instance
screening, all-arm budget exclusions and censored target times. CLI smoke uses
non-registered synthetic seeds. No performance result exists at this point.
