# Ising Engine — Project Plan

**What this file is.** The single active sequence: where the project stands
today, the one atomic task in flight, and the ordered gates between here and a
verifiable product. It does **not** restate direction (`research/ISING_ENGINE_CONSTITUTION.md`),
philosophy (`SOUL.md`), or working rules (`AGENTS.md`, `CLAUDE.md`). Status by
stage lives in `ROADMAP.md`; this file is the *path*, not the *inventory*.

**Read order for a new agent:** this file → `memory/CURRENT_HANDOFF.md` → only
the files that handoff names. Do not scan the repo.

---

## 1. Standing negative record — binding

These are the load-bearing facts. Every plan below is shaped by them, and none
of them may be softened in a summary, a commit message, or a report.

| Cycle | What it actually established |
|---|---|
| **RC-016** | `RC016_CYCLE_RECORD.md`: `K = 25`, `k+ = 25`, `k- = 0`, **`VERDICT = SIGN CONSTANT`**. H-16 — the `S0`-matched opposite-action pair — is **REFUTED**, and **no causal `S0` counterexample was found**. What this establishes is a **stable causal equal-sweep outcome ordering for this operator pair, slot, initialization, corpus and equal-sweep budget** — and nothing wider. It is **not** an equal-cost result, **not** a policy, **not** general `S₁` sufficiency, and **not** a universal ordering of Markov kernels. Amendment 4 withdrew the cost arm; only 1 of 30 arms was genuinely powered. |
| **RC-017** | **No scientific result.** Aborted as an invalid-instrument attempt (`RC017_ABORT_RECORD.md`). Seeds `5001–5008` are burned and permanently unusable. |
| **RC-018** | **Instrument invalid** (`RC018_PILOT_ABORT_RECORD.md`): a mandatory pre-registered guard was asserted in prose but absent from executable code. Closed, not repaired. No RC-018 datum, seed, threshold or verdict is evidence for anything. |
| **RC-020** | **CLOSED WITHOUT A SCIENTIFIC VERDICT.** `RC020_PILOT_ABORT_RECORD.md`: two pilot attempts, no artifact from either, **Gate A = NO VERDICT**. Attempt 1 was refused at the gate (K1, exit 2, body never ran). Attempt 2 ran the body, then §5 discarded the session when repetition 1 exceeded the sentinel bound twice (exit 5); **`K3` did NOT fire** — **failed executions = 2; unique discarded repetition IDs = 1; internal discard counter = 2.** `K3` cannot fire under any reading, because no value exceeds 2, while `K3` requires more than 2; both failures were of the *same* repetition. Seeds `10001–10008` are **burned** conservatively. Held-in `11001–11008` and held-out `12001–12008` are **untouched**; §9 is still blank. RC-020 measured **nothing that survived** and licenses no claim about cost, ordering, equal cost or any cost model. |

Consequence: the project currently holds **no** general claim about operator
selection, cost models, or superiority over `UltimateSolver`. Nothing downstream
may assume one.

---

## 2. The scientific sequence

Strictly ordered. A stage may not begin until the one before it has returned a
verdict that permits it.

### Stage S1 — RC-020 pilot, Gate A — **CLOSED, NO VERDICT**

Scope was pilot block `10001–10008` only. Gate A asked one question — *is `b`
identifiable on this host at this precision?* **It was never answered.**

Binding documents: `research/PREREG_RC020_MARGINAL_WALL_COST.md` and
`research/PREREG_RC020_AMENDMENT_1.md` (`CONTROL_SEED = 20001` frozen).
Instrument: `src/bin/exp_marginal_cost.rs`, commit `567dd7e`.
Outcome record: **`research/RC020_PILOT_ABORT_RECORD.md`**.

**Final position.** Two attempts, neither producing an artifact.

| | attempt 1 | attempt 2 |
|---|---|---|
| stopped at | control gate, K1 (P3 clean half) | §5 session discard, repetition 1 twice |
| exit | 2 | 5 |
| pilot body | did not run | **ran** — repetition 0 completed |
| seeds `10001–10008` | not executed | **executed** |
| artifact | none | none |

`experiments/rc020/` does not exist. **Gate A has no verdict**, so none of S2's
four branches fires and **S3 is unreachable**. Held-in, held-out and the §9
descendant are untouched.

**Three findings carried forward, each binding on the successor.**

1. **§8 does not classify this outcome.** It matches none of `K1`–`K8`: controls
   passed, the sentinel was present, `K3` needs more than 2 while failed
   executions were 2 and unique discarded repetitions 1, no degeneracy bound was
   crossed, no forbidden seed ran,
   provenance passed, and `K7`/`K8` both presuppose a Gate-A verdict.
   Substantively Class I; unclassified as written. The successor must close the
   gap.
2. **§5 and §10.1 contradict each other on an aborted session.** §5 requires the
   discard to be written to the artifact; §10.1 forbids writing the artifact
   unless the session completes. The instrument obeyed §10.1 and violated §5, so
   every discard row accumulated in memory and died with the process. The
   successor must separate the guard/discard journal from the observation
   artifact.
3. **Cause is not established, and the three hypotheses are not distinguished.**
   Elevated paired spread existed already in the control phase — in-process `N1`
   `0.0755`, 84% of the bound, versus `0.0012` standalone two minutes earlier —
   so warming from repetition 0 alone is insufficient. But the control phase
   itself loads the CPU (a warmup trajectory, one 100 000-resample bootstrap,
   then twelve more warmups), so thermal, bursty-host and CPU-contention
   explanations are **not separated** by these data. See the record's §4.

**Method rule, binding.** The pilot may **not** be retried until it happens to
pass. Repeating a run until a drift guard admits it selects an atypically quiet
moment and biases the very timing quantity being measured. **The instrument must
work reliably, not pass once by luck.** Do not lower the `0.09` bound.

### Stage S1b — RC-021 host/instrument successor — **ACTIVE, THE ONLY TASK IN FLIGHT**

Gate A returned nothing, so S2 cannot fire and S3 is unreachable. The sequence
does not resume at RC-020; it resumes at a **fresh pre-registered successor**,
provisionally **RC-021**, whose subject is the *host and the instrument*, not
marginal cost.

**Status: pre-registered. The document is
`research/PREREG_RC021_HOST_INSTRUMENT.md`. Implementation remains BLOCKED and
requires a separate explicit decision.** No RC-021 instrument exists, no RC-021
datum exists, and `experiments/rc021/` does not exist.

Its frozen design: primary estimand is the **paired wall-time spread** of one
deterministic sentinel workload; **6 sessions × 3 phases × 5 pairs = 90**
measurements against the **inherited** `0.09` bound; `QUALIFIED` iff every
session completes, failures ≤ 2 of 90, no session contributes more than 1, and
every control passes. `p ≥ 0.9023` is what a successor session needs, and the
**minimum** sample size clearing it at `k ≤ 2` is `N = 63`; RC-021 fixes `N = 90`
as a **choice above that minimum**, for protocol symmetry (`6 × 3 × 5`) and
margin (lower bound `0.9317` against `0.9034`). Diagnostic seeds `31001–31004`,
band `[31001, 31099]`, disjoint from every prior family.

RC-021 required, in its pre-registration and before any code:

1. **Characterise the host's timing distribution as its own object of study**,
   with a pass criterion fixed in advance — rather than assuming a quiet moment
   exists and sampling until one appears.
2. **Resolve the §5 / §10.1 contradiction** by separating the guard and discard
   journal from the observation artifact, so a session that yields no
   observations still leaves a complete record of why it yielded none.
3. **Close the §8 taxonomy gap**, giving the session-discard path an explicit
   kill criterion and class.
4. **Name fresh, disjoint seed blocks.** `10001–10008` are burned;
   `11001–11008` and `12001–12008` stay reserved and unopened. RC-021's own
   diagnostic seeds are `31001–31004` in the reserved band `[31001, 31099]`,
   fixed by its pre-registration §3.2 — they are diagnostic, not science seeds,
   and no *science* seed number is chosen by this plan.
5. State what would make the successor **stop** — including the outcome that
   this host cannot support ms/sweep timing at all, which is a legitimate and
   publishable conclusion.

**Not permitted as the next step:** retrying the RC-020 pilot, repairing the
RC-020 instrument in place, lowering the `0.09` bound, opening any reserved
block, or writing the §9 descendant.

### Stage S2 — the four branches out of Gate A

Exactly one fires. The branch is read off the artifact, never argued for.

| Branch | Class | What happens next |
|---|---|---|
| **INSTRUMENT INVALID** (K1–K6) | Class I | **No result of any kind.** Publish the abort record. Repair or replace the instrument under a fresh pre-registration. Never write it up as a null. |
| **NOT IDENTIFIED** (K7) | Class II | A **scientific null and a first-class result**. Publish it. `b` is not identifiable at this precision on this host; the cost-model line is closed until a materially better instrument or host exists. |
| **IDENTIFIED + NOT FEASIBLE** (K8) | Class II | Gate A passed; the CI on `k'_real(i)` is ≥ 3.0 sweeps wide on the qualifying cells. Publish. **No equal-cost design may be constructed.** The line stops here. |
| **IDENTIFIED + FEASIBLE** | — | The **only** branch that opens Stage S3. CI width < 1.0 sweep on ≥ 5 of 6 instances. `FEASIBLE AT REDUCED RESOLUTION` (width in `[1.0, 3.0)`) is **not** this branch: it may only justify a further calibration amendment, never a descendant. |

### Stage S3 — equal-cost descendant — **only from IDENTIFIED + FEASIBLE**

`PREREG_RC020_MARGINAL_WALL_COST.md` §9 is deliberately blank and frozen. A
descendant must:

1. be written **after** Gate A returns its verdict on the pilot, and
2. be **committed before any held-in seed is opened**, and
3. name its candidate set and an exact numeric pass/fail threshold using **only
   quantities RC-020 actually measured**.

Held-in precedes held-out. Held-out is opened once, after the descendant and the
held-in analysis are both frozen in git.

### Stage S4 — causal runtime-sensor sufficiency

Only after S3 has produced a replicated equal-cost result. This is the honest
successor to RC-016/RC-017: does the runtime sensor set causally suffice to
choose an operator mid-run? Requires a fresh pre-registration and an ablation on
the Runtime (`AGENTS.md` Theory-Engine rule). Seeds `6001–6008` are reserved to
RC-017's successor and remain uninspected.

### Stage S5 — algebraic feasibility map

Only after S4. A map over the operator/architecture space of what is
**algebraically possible** before anything is generated — the successor to
RC-004's impossibility proofs and RC-006's gradient-ledger identity. Deliverable:
for each axis, a proof or a bound saying whether a gain is reachable at all.

### Stage S6 — Instance Foundry — **gated behind S5**

Extending `families.rs` into an Instance Foundry is forbidden until the
feasibility map exists. The standing rule from `RC014_PLAN_CAUSAL_INSTRUMENT.md`
holds: *do not build the Foundry to strengthen a weak signal.*

### Stage S7 — DSL and operator synthesis — **last**

The largest, least-constrained step. It runs only on top of a feasibility map
and a working causal selector. Nothing earlier may be justified by "we will need
it for the DSL".

---

## 3. External comparisons — the honesty check on the route above

Full rationale, links, stop conditions and non-goals:
**`research/EXTERNAL_PROJECTS_BACKLOG.md`** (§1 priority queue, §6 activation
order). Not restated here. These run **after** the causal/equal-cost route has a
selector worth comparing — they exist to stop us presenting a known field as a
new one.

Ordered:

1. **ASlib + DACBench**, then **SMAC3 / Nevergrad / Optuna** under equal budgets.
   ASlib is the standard per-instance algorithm-selection baseline; DACBench is
   the closest established field to a runtime operator controller. Our method
   must improve held-out quality/cost, not merely find a different schedule.
2. **BiqMac corpus**, then one external solver baseline — **OpenJij** or
   **MQLib**. A G-Set-only result is insufficient by construction for any
   weight-aware or frustration claim.
3. **Soup applied pilot** (LoRA subset selection as QUBO) — last, and only at an
   identical real-evaluation budget. Our engine is an outer-loop discrete
   optimizer there, never a replacement for gradient training.

Bounded uses, fixed: **MiroFish — market/scenario research only**, never solver
validation or demand forecasting. **Obsidian — a generated, one-way Markdown
view only**; git-tracked files stay the source of truth. **Graphiti — rejected**,
it duplicates the existing deterministic knowledge graph.

## 4. The product path

Research alone is not the deliverable. The product sequence is equally ordered,
and each step needs the one before it.

1. **Domain-specific causal selector.** Not a universal optimizer — one problem
   family, one selection rule, with a stated domain of validity.
2. **Replicated equal-cost gain.** The gain must reproduce on an independent
   seed block at equal cost. One block is not replication.
3. **Production A/B vs `UltimateSolver`.** Identical seeds, bit-identical
   protocol, `benchmark_suite/scripts/ab_engine_compare.py` or
   `bin/ab_evolved_vs_ultimate.rs`. `UltimateSolver` is the only arbiter. The
   golden regression must pass unchanged.
4. **API / demo.** Only after step 3 wins by a measured >1% margin.

**Until step 3 has been won, no universal-optimizer claim may be made** — not in
docs, not in a README, not in a demo, not in a commit message. The current
honest statement is: *no superiority is claimed for the research engine over
production.*

---

## 5. Kill criteria

Stop and publish the negative, rather than continuing:

- **K-A** Gate A returns INSTRUMENT INVALID twice on a repaired instrument →
  the marginal-cost line is closed; write the closure record.
- **K-B** Gate A returns NOT IDENTIFIED → publish the null; do not re-scope,
  re-threshold, or move to a friendlier host to get a different answer.
- **K-C** Gate A returns IDENTIFIED + NOT FEASIBLE → publish; no descendant.
- **K-D** The equal-cost gain fails to replicate on the independent block →
  the selector is dead in that domain; publish and do not widen the domain to
  rescue it.
- **K-E** Production A/B loses to `UltimateSolver` at identical seeds → no
  product claim; record the refutation.
- **K-F** Host cannot hold P3 spread under `0.09` across three separate quiet
  attempts → the measurement is not possible on this hardware; record that as
  the finding.

A kill is a result. It gets written down with the same care as a win.

---

## 6. Forbidden actions

Beyond the hard "never" rules (`CLAUDE.md` §4, `AGENTS.md` §2.5), and binding
for this sequence:

- Opening held-in or held-out seeds before the descendant is committed.
- Reusing burned seeds `5001–5008`, or inspecting reserved `6001–6008`.
- Changing `CONTROL_SEED = 20001` or any frozen seed without an amendment.
- Lowering the 9% drift bound, or any pre-registered threshold, after seeing
  data.
- Writing a Class I (invalid instrument) outcome up as a Class II (null) result.
- Building the Foundry, the DSL, new operators, or growth campaigns while any
  stage above them is unresolved.
- DAG runs, automations, and `orchestration worker_done`. The user coordinates
  stages manually; the coordinator is Claude Code.
- Running RC-020's pilot without an explicit decision from the user.

---

## 7. TOKEN ECONOMY

Binding for every agent on this project. Reading order, search order and role
definitions are **not** restated here — follow `AGENTS.md` §2.6 (Context Budget),
§2.7 (Search Strategy) and §3 (Agent Roles). What follows is only what those do
not already cover.

- **Enter through the handoff.** `PROJECT_PLAN.md` → `memory/CURRENT_HANDOFF.md`
  → only the files the handoff names. Never scan the repo; never read every `.md`.
- **Two agents per task, maximum.** One implementer, one independent reviewer
  (`AGENTS.md` §3.5). No duplicate agents on the same work.
- **One atomic task at a time.** The handoff names exactly one. Finish it or
  stop it; do not open a second front.
- **Short prompts that reference files** instead of quoting them. Never paste
  logs, artifacts, or diffs into a prompt — give the path and the line range.
- **Fresh session or `/compact` between tasks.** Do not carry a finished task's
  context into the next one.
- **Route by difficulty.** Cheap mechanical work on a small model; only
  gate-level reasoning and reviews on the largest one.
- **Docs-only changes need no `cargo`.** `git diff --check` is still mandatory.

---

## 8. Where each answer lives

| Question | File |
|---|---|
| Direction, amendments | `research/ISING_ENGINE_CONSTITUTION.md` |
| Why the project exists | `SOUL.md` |
| How work is done | `AGENTS.md`, `CLAUDE.md` |
| Architecture decisions | `research/architecture/ADR/` |
| Stage inventory and % | `ROADMAP.md` |
| **Active sequence and gates** | **this file** |
| **What to do right now** | **`memory/CURRENT_HANDOFF.md`** |
| Open decisions | `memory/OPEN_PROBLEMS.md` |
