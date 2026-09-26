# HUBO-C001 Builder intake

Mode C: research evidence. User authorized continuing HUBO comparison after
RG001, with reproducible outcomes and independent verification. Success of the
engineering task means a complete integrity-valid comparison, whether favorable,
null or adverse. Scientific continuation criteria are only in protocol.md.

Canonical code belongs in this directory plus research/examples/hubo_compare.rs.
Production engine/core/compiler and both solver families are read-only. Reuse
engine::step; do not create a new optimizer and call it UltimateSolver. Four
arms isolate representation within the MSC and OpenJij driver families.
Dependencies exist in benchmark-env: OpenJij0.12.0, dimod0.12.22, NumPy2.5.1;
no new packages are required. Old adapter's fallback dropping unsupported seeds
is unsuitable and will not be reused. Raw deadline/seed/witness integrity is
mandatory, using warm-worker semantics explicitly, not pretending Python imports
and Rust startup have equivalent cold costs.

Known risks: conservative global penalties and untuned schedules can explain
an apparent native advantage; direct OpenJij is a compulsory competing baseline.
Small synthetic corpus, two-second warm budget and WSL host cannot establish
SOTA, cold-service latency, optimum hitting time or application usefulness.
All-zero baseline makes empty search visible. A valid decoded original x need
not have feasible auxiliaries; report original energy, not Q energy.

Missing capability to build: shared supervised worker, MSC experiment driver,
strict recorder/analyzer, deterministic tests and immutable raw evidence. No
Laya training, record hunt, production integration or old experiment changes.
Stop at frozen handoff and independent review; any NO-GO remains preserved.
