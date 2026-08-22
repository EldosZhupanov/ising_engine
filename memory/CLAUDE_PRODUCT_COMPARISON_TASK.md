# Claude coordinator task — evidence, comparison, then product

**Authority:** direct user instruction, 2026-08-22.  Claude Code is the main
coordinator.  Work is manual: no DAG, automation, orchestration tasks, or
`worker_done` protocol.

## Objective

Finish the scientific and product specification, then compare Ising Engine
honestly against established external baselines.  Build the product only if a
predeclared breakthrough gate passes.  Do not turn an engineering difference
into a novelty or superiority claim.

## Ordered actions

1. Finish the current `PROJECT_PLAN.md`, `memory/CURRENT_HANDOFF.md`,
   `CLAUDE.md`, and `ROADMAP.md` docs-only step.  Correct RC-016 to the exact
   `SIGN CONSTANT` record (`K=25`, `k+=25`, `k-=0`; H-16 refuted), incorporate
   the independent Antigravity `CONSISTENT` review, run `git diff --check`, show
   the result to the user, and make one docs commit only after approval.
2. Write a compact `PRODUCT_SPEC.md` with three labels on every capability:
   `CURRENT/PROVEN`, `TARGET`, or `GATED`.  Cover customer/problem, QUBO/Ising
   input, automatic instance analysis, initial operator choice, mid-run sensor
   control, measured-cost budgets, `UltimateSolver` fallback, outputs and
   provenance, CLI/API, deployment/security, success metrics, demo, and business
   hypotheses.  Do not implement the product before the benchmark gate.
3. Write `research/EXTERNAL_COMPARISON_PROTOCOL.md` from
   `research/EXTERNAL_PROJECTS_BACKLOG.md`:
   - wave 1: ASlib/DACBench conventions plus SMAC3/Nevergrad/Optuna;
   - wave 2: BiqMac plus OpenJij or MQLib;
   - Soup/PEFT only after a core optimization win.
   Freeze project commit/version, adapters/conversions, corpus, seeds,
   wall-time and evaluation budgets, anytime curves, TTS, quality gap, overhead,
   confidence intervals, held-out routing, and kill criteria.  Start with
   `random + greedy + one established specialist + ours`; do not install every
   candidate.
4. Close RC-020 first.  This user instruction authorizes one further pilot
   attempt only when the tree is clean, `experiments/rc020/` is absent, prior
   evidence proves seeds `10001-10008` did not execute, the host is quiet, and
   all in-process controls including P3 pass.  Do not weaken the frozen 9% bound
   and do not repeat after science seeds actually begin.  Record exact
   provenance and the Gate-A route.
5. Branch exactly as preregistered.  Only `IDENTIFIED + FEASIBLE` permits a
   committed equal-cost descendant before held-in/held-out.  `INSTRUMENT
   INVALID`, `NOT IDENTIFIED`, or `NOT FEASIBLE` is published and stops that
   line.  Propose any alternative comparison explicitly and wait for the user;
   never imply equal cost without evidence.
6. Only after a replicated equal-cost result, execute external comparison waves
   as separate atomic plans with independent read-only Antigravity review.

## Breakthrough and product-build gate

Predeclare the exact test before external data.  At minimum require a held-out,
replicated practical gain above 1%, or a strictly better quality-versus-wall-time
Pareto point, against both an established specialist and `UltimateSolver`, with
correctness and deterministic replay intact.  Review novelty separately against
primary literature.

- If the gate fails: publish the negative result; do not rescue it by changing
  metrics, budgets, corpus, or marketing language.
- If it passes: present the user with the evidence and an MVP build plan.  Only
  then begin product implementation or use a "best"/"breakthrough" claim.

After every atomic step update `memory/CURRENT_HANDOFF.md` (maximum 80 lines).
Use targeted reads, short path-based prompts, one implementer plus one reviewer,
and a fresh/compacted session between tasks.

Start with action 1 and communicate progress directly to the user.
