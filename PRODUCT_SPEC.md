# Ising Engine — Product Specification

**Scope.** What the product is, and what is honestly true about it today. Every
capability carries exactly one label:

- **`CURRENT/PROVEN`** — implemented, tested, and measured in this repository.
- **`TARGET`** — intended, not yet built; no evidence claimed.
- **`GATED`** — blocked behind a named gate in `PROJECT_PLAN.md`. May not be
  built, demoed, or described as working until that gate passes.

Sequence and gates live in `PROJECT_PLAN.md`; current state in
`memory/NOW.md`. **No product implementation begins before the
benchmark gate in §12.**

---

## 1. Customer and problem — `TARGET`

**Who.** Teams with a combinatorial objective already expressible as QUBO/Ising
who currently hand it to a general solver (CP-SAT, SCIP, Gurobi) or a hand-tuned
heuristic, and who care about *quality reached inside a wall-clock budget*
rather than proven optimality.

**Problem.** Choosing and configuring a heuristic per instance is manual,
undocumented, and does not transfer between instances.

**What we would sell.** A per-instance operator/schedule choice that is
*measured*, *reproducible*, and *falls back safely* — not a new metaheuristic.

**Status.** No customer has been interviewed and no design partner exists. This
section is a hypothesis, labelled as one.

## 2. QUBO / Ising input — `CURRENT/PROVEN`

`src/core` holds the QUBO/HUBO model and Ising types. Parsers in
`src/benchmark/instances.rs`: `parse_rudy_maxcut`, `parse_orlib_bqp`,
`parse_biqmac_sparse`, `parse_qplib`, plus `load_gset`. The research bridge
`engine_v2::frontend::qubo_model_to_ir` is energy-exact.

Objective conventions per corpus are a known trap and are validated against
published optima before any number is used.

## 3. Automatic instance analysis — `TARGET`

Descriptor extraction exists inside the research platform, but **as a product
capability it does not exist**: there is no supported "analyse this instance and
report its class" entry point, and the descriptor work that was tried has a
recorded refutation (multidimensional-descriptor H1 overfits; ruggedness and
entropy survive). Nothing here may be presented as working.

## 4. Initial operator choice — `GATED`

Gated on **`PROJECT_PLAN.md` §2 Stage S4** (causal runtime-sensor sufficiency),
which is itself downstream of RC-020 Gate A and a replicated equal-cost result.

What is actually established: RC-016 is `SIGN CONSTANT` (`K=25, k+=25, k-=0`),
H-16 refuted, **no causal `S0` counterexample found** — a stable causal
equal-sweep ordering for **one** operator pair, slot, initialization, corpus and
budget. That is not a selection policy and may not be sold as one.

## 5. Mid-run sensor control — `GATED`

Gated on **Stage S4**. RC-017, the cycle that would have established mid-run
sensor sufficiency, **produced no scientific result** and was aborted as an
invalid instrument; its seeds `5001–5008` are burned.

The mechanism exists in the substrate (`Runtime::maybe_adapt`, opt-in,
bit-identical on replay per ADR-0004). Mechanism availability is not evidence
that control helps.

## 6. Measured-cost budgets — `GATED`

Gated on **RC-020 Gate A** (`PROJECT_PLAN.md` §2 Stage S1) and then Stage S3.

RC-020 measures exactly one quantity: marginal wall cost
`b(i,o) = d(wall)/d(sweeps)` in ms/sweep. Gate A asks only whether `b` is
*identifiable*. It licenses **no** equal-cost comparison and **no** efficacy
claim. Until a descendant is committed and a replicated equal-cost result
exists, the product has **no** cost-budget capability.

## 7. `UltimateSolver` fallback — `CURRENT/PROVEN`

`src/solver/ultimate.rs` — MSC bit-sliced (64 replicas/u64) + Parallel Tempering
+ Population Annealing, with QPBO/roof-duality presolve (`src/presolve`).
Entry points `solve`, `solve_with_luby_restarts`, `solve_with_pa_diagnostics`.
Protected by `tests/test_regression_golden.rs`.

**It is the default path and the arbiter, not a fallback of last resort.** Any
selector ships *in front of* it and must beat it at identical seeds to be
enabled at all. Evolved plans currently lose 5/5 against it.

## 8. Outputs and provenance — split

- **`CURRENT/PROVEN`** — deterministic replay: same seed → same trajectory →
  same energy (ADR-0004). Append-only experiment DB with full provenance.
  Statistics in `src/benchmark/stats.rs` (Wilcoxon, paired-t, bootstrap);
  time-to-solution in `src/solver/tts.rs`; A/B harnesses
  `benchmark_suite/scripts/ab_engine_compare.py` and
  `src/bin/ab_evolved_vs_ultimate.rs`, which assert bit-identical energies.
- **`TARGET`** — a customer-facing result object: solution, energy, wall time,
  seed, solver identity, commit hash, and the reason this configuration was
  chosen, in one signed record.

## 9. CLI and API — split

- **`CURRENT/PROVEN`** — `src/bin/server_api.rs`: axum, `GET /health`,
  `POST /api/v1/solve`, constructing an `UltimateSolver` per request. Never
  bypasses it. `src/bin/control_api.rs` binds `127.0.0.1` only and is read-only
  by construction. `src/bin/research_platform.rs` is the single research entry
  point.
- **`TARGET`** — a stable public CLI and versioned API contract, batch
  submission, and a client library. The current HTTP surface is a development
  fixture, not a product API.

## 10. Deployment and security — `TARGET`

Nothing here is built. Current honest state: **no authentication, no TLS, no
tenancy, no rate limiting, no audit log.** `control_api` documents in source
that it has no authentication because it performs no mutation; `server_api`
accepts unauthenticated solve requests on whatever address it is bound to.

Before any external exposure: authentication, transport security, request
limits, resource caps per job, and an audit trail. Treat today's binaries as
localhost-only development tools.

## 11. Success metrics — `TARGET` (definitions fixed now, unmeasured)

Fixed in advance so they cannot be reshaped later:

1. **Practical gain** — held-out, replicated improvement **> 1%** in solution
   quality at equal wall-clock budget, against both `UltimateSolver` and an
   established external specialist.
2. **Pareto** — alternatively, a strictly better quality-versus-wall-time point,
   shown with anytime curves, not a single endpoint.
3. **Overhead** — selector overhead reported as a share of total wall time.
4. **Correctness** — golden regression unchanged; deterministic replay intact.
5. **Coverage** — the domain of validity stated explicitly, with the instances
   where the selector does *not* help listed.

No metric may be swapped, re-weighted, or re-scoped after external data is seen.

## 12. Breakthrough / product-build gate — the blocker on everything above

Predeclared, before any external data is collected. To pass, **all** must hold:

- held-out, **replicated** practical gain > 1% **or** a strictly better
  quality-vs-wall-time Pareto point;
- against **both** an established external specialist **and** `UltimateSolver`;
- correctness and deterministic replay intact;
- novelty reviewed **separately**, against primary literature — an engineering
  difference is not a novelty claim.

**Fail ⇒** publish the negative result. Do not rescue it by changing metrics,
budgets, corpus, or marketing language.
**Pass ⇒** present the evidence and an MVP build plan to the user. Only then may
implementation start or the words "best" / "breakthrough" be used.

Comparison mechanics are fixed separately in
`research/EXTERNAL_COMPARISON_PROTOCOL.md`.

## 13. Demo — `GATED`

Gated on §12. A demo that shows a selector "choosing" without a passed gate is a
superiority claim in disguise. When permitted: one instance family, side-by-side
anytime curves against `UltimateSolver` and one external specialist, at equal
wall clock, with seeds and commit shown on screen.

## 14. Business hypotheses — `TARGET`, all unvalidated

| # | Hypothesis | How it dies |
|---|---|---|
| B1 | Teams will pay for *measured, reproducible* configuration rather than a faster solver. | Interviews show they buy proven optimality or nothing. |
| B2 | Per-instance choice transfers within a family well enough to be worth automating. | Gain does not replicate on held-out instances of the same family. |
| B3 | A narrow domain-specific selector beats a general portfolio tool (SMAC3/Optuna) at equal budget. | Wave-1 comparison shows parity or worse. |
| B4 | Deterministic replay is a purchasing criterion, not a nicety. | No prospect raises reproducibility unprompted. |

Each is a hypothesis, not a finding. None may appear in external material as a
statement of fact.

---

**Standing constraint.** Until §12 passes, the honest public statement is: *no
superiority is claimed for the research engine over production.* Nothing in this
document overrides `research/ISING_ENGINE_CONSTITUTION.md`.
