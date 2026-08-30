---
id: memory-now
kind: live-state
status: active
authority_scope: current-task
updated: 2026-08-28
immutable: false
---

# NOW — the only active project state

This file is the sole repository authority for what is in flight. Always verify
its claims against `git status --short` and the current branch before acting.

## Standing objective — the thing we do not drift from

Set 2026-08-28. **Subordinate to `research/ISING_ENGINE_CONSTITUTION.md`, which
is the Single Source of Truth for direction and is immutable.** This section
does not set direction; it states the one concrete objective the current work
serves, so that any task can be checked against it in a sentence.

The Constitution's mission is **access**: the commodity CPU as a competitive
optimization machine, delivered as *an instrument whose readings can be trusted*
— and it names three assets that do not expire: the architecture, the evidence
base, and the discipline.

**The objective, concretely:**

> Establish whether the algorithm space for our problem class has more than the
> one architectural cell our own corpus occupies — and if it does, build the
> instrument that finds candidates in the empty cells and can prove what it
> found.

**Why this and not something else.** Five recorded results say the same thing
five ways: RC-004 places all eighteen operators at the single point
`(config, energy, binary, temperature, R, local, sweeps)`;
`AXIOMS_OF_OPTIMIZATION.md` finds we kill none of the eight axioms;
`RELATIONAL_PRIMITIVE.md` proves marginal-state methods vacuous on MaxCut;
evolved plans lose 5/5 to `UltimateSolver`; runtime operator synthesis bought
+0.06%. We search one cell extremely well. Nothing yet shows a second cell
exists.

**The test that decides it** is `research/EXTERNAL_PROJECTS_BACKLOG.md` §8.A —
map MQLib's human-designed MaxCut/QUBO heuristics onto RC-004's seven axes.
Read-and-classify only: no compute, no dependency, no LLM. All three outcomes
are results, and the outcome selects the program:

- other cells are occupied → the taxonomy discriminates; §8.B and §8.C follow;
- everything collapses into our cell → a real structural convergence of the
  field, stronger than RC-004 alone, and scaling the dataset becomes the better
  use of compute, **confirming** the current roadmap rather than correcting it;
- the heuristics resist clean classification → the taxonomy is repaired before
  it is allowed to steer anything.

**What this objective forbids.** Adopting an external framework before the
census says the space has room. Growing the dataset toward 500k as a *goal*
rather than as a consequence. Publishing any Class II claim without the arm that
could refute it — the Soup pilot of 2026-08-28 is the standing example of what
that costs: a clean infrastructure result (a 7B model QLoRA-tuned on a 6 GB
laptop GPU, ~2.07 GB peak, deterministic receipt) that says **nothing** about
our selector, because no random-subset arm was run.

## Active task

**RC-021 Step 8 — CLI and end-to-end synthetic tests.**

This is the single active task. The five public modes and synthetic or injected
end-to-end tests are permitted. Running RC-021 is not: no production control,
no qualification session, and no sentinel on G11 may execute, and
`experiments/rc021/` must not be created.

## Working context

- Active branch: `feat/solver-research-upgrades`.
- RC-021 Step 6 is complete, committed as
  `1b855b774654f85a326baf37436262e9f5322e5b`, and an independent read-only
  review of it returned PASS.
- RC-021 Step 7 is complete and committed as
  `3493ed8`; its independent review blockers and adversarial follow-up findings
  are closed, and the full source gate set is green.
- **Step 8 is committed as `35fb5fa`, with `ad24013` on top.** Six independent
  review rounds have run. The sixth found a HIGH — session journals were never
  bound to the run identity, so a journal from another run counted toward the
  ninety and `--verify` would have confirmed it. Every HIGH and MEDIUM across
  all six rounds is fixed and confirmed by a killed mutation. The full finding
  set, fixed and deliberately open, is `memory/OPEN_PROBLEMS.md` §0b.
- **RC-021 has still never been executed** and must not be until a review round
  returns no HIGH and no MEDIUM. §2 permits no retries and no "repeat in a
  quieter moment": the run is spent the moment it starts, so it starts only on
  an instrument no round is still finding defects in, and on a quiet host with
  nothing else running. A full run is ~65 minutes, of which ~60 is the six
  mandatory ten-minute idle intervals.
- The durable-memory series is integrated on this branch as commits
  `983497d..d5880e3`.
- The integrated memory state at `e824b7f` was independently reviewed and
  returned PASS.
- The working tree contains an unrelated draft change to
  `research/EXTERNAL_PROJECTS_BACKLOG.md`; preserve it and exclude it from every
  RC-021 commit.
- RC-021 has never been executed: no controls, no qualification session, no
  sentinel on G11, and no measurement data of any kind.
- Binding RC documents and their paths are immutable.
- `docs/memory-architecture` is a historical backup reference only. It is not
  the active branch and holds no authority.

## Product position

- The production solver exists and remains the comparison arbiter.
- The `engine_v2` selector, capability registry, plan synthesis, Runtime,
  knowledge system, and research orchestration exist.
- Production auto-routing is not yet licensed: the domain-specific selector
  still needs replicated equal-cost evidence and a production A/B win against
  `UltimateSolver`.

## This task is complete only when

1. The five public modes and their exact argument/precondition state machine are
   wired through the existing private modules.
2. Every mode exit code is derived from the frozen mode/run-status taxonomy.
3. Synthetic end-to-end tests cover the legal transitions, refusals, terminal
   lock, finalization and read-only verification.
4. No test executes a real control, qualification session, or G11 sentinel.
5. The applicable `cargo` check, test, build, `clippy`, and `fmt` gates pass.
6. An independent read-only review of the complete instrument returns PASS.
7. No `experiments/rc021/` and no RC-021 measurement data exists.

## When Step 8 is finished — the stopping criterion

Set 2026-08-28 after three review rounds each found defects, several of them in
the previous round's own fixes. Without a stated criterion this converges on
nothing: the diff grows, each round finds less, and an uncommitted ~4 000-line
change is itself a risk — it can be lost, and it grows past the point where a
reviewer can hold it.

**Commit Step 8 when a review round returns no HIGH and no MEDIUM correctness
finding.** LOW findings are recorded in `OPEN_PROBLEMS.md` and closed
deliberately afterwards, not before the commit.

This is not a lowering of the bar. Every HIGH and MEDIUM found so far has been
fixed and mutation-verified; the criterion only says that polish below that
severity does not justify holding four thousand lines out of history.

## The census is done — and it decided the programme

**`research/RC022_ARCHITECTURE_CENSUS.md`, 2026-08-30.** The §8.A experiment
that the standing objective named as decisive has run. Analysis only: no
compute, no dependency, no execution.

Twenty independently designed MaxCut/QUBO heuristic families from MQLib spread
across **at least eight cells** of RC-004's seven-axis space, against one for
our eighteen operators. **No family occupies our cell**, and three mechanisms
the corpus uses heavily are absent from our operator library entirely:

- **memory beyond the configuration** — 7 of 20 families, all tabu-family. We
  have **one**, in a different form: `history_field`, a metadynamics soft
  decaying penalty, which is among our **top four** operators by mean rank. The
  first version of RC-022 said we had none; that was wrong and the correction is
  §0 of the record. The gap is not the axis but the **form**: no hard
  prohibition, no recency list, no aspiration criterion anywhere;
- **synthesized moves** (crossover, path-relinking) — 9 of 20; RC-003 touched
  this axis once, for +0.06 %;
- continuous domain with gradient moves — 1 family.

So RC-004's one-cell result is a property of **our corpus**, not of the problem
class, and the taxonomy discriminates well enough to steer generation. The
marginal/probabilistic cell stays closed to us by `RELATIONAL_PRIMITIVE.md`'s
Z₂ theorem, which leaves **memory as the surviving Ax2 route** —
`AXIOMS_OF_OPTIMIZATION.md` §5's named attackable axiom.

This does **not** say an empty cell holds a better algorithm, that memory helps
our instances, or that any of these beats `UltimateSolver`. It is a map of what
exists, not a measurement of what wins.

**The question it produced, which is the next piece of work:** does the *form* of
memory matter — can hard prohibition with an aspiration criterion do something a
soft decaying bias cannot? Both fit the existing Operator API unchanged
(`apply(&mut self, …)`, one instance per plan, determinism already proven by
`history_field`), so the test is one new operator against one existing one on
the same substrate at identical seeds.

## Next action — in this order

1. **Close the three untested state-machine rows** listed in
   `OPEN_PROBLEMS.md` §0b item I. They are the last coverage debt between Step 8
   and a review that can return PASS.
2. **One independent read-only review of the complete instrument.** If it
   returns PASS, commit Step 8 as a single source commit — and only then.
   Leaving ~3 000 lines uncommitted is currently the largest recoverable risk in
   the repository.
3. **Decide item D** in `OPEN_PROBLEMS.md` §0b: does exit 4 mean "the evidence is
   broken" or "something went wrong"? A sentinel that could not run is not a
   damaged journal. This is a contract question and belongs to the maintainer.
4. **Then the census** — `EXTERNAL_PROJECTS_BACKLOG.md` §8.A. It costs no compute
   and it selects which research program is worth running at all.

RC-021 is an instrument, not a result. Finishing it earns the right to measure;
it does not measure anything. Item 4 is where evidence starts.

## Forbidden during this task

- Running RC-021, any real control, any qualification session, or a sentinel on
  G11.
- Creating `experiments/rc021/`.
- Editing or moving any binding preregistration, amendment, or cycle record.
- Fixing the non-blocking F1/F2 test-hardening findings "while here".
- Starting the real six-session execution or writing the instrument-review
  record before the complete-instrument review returns PASS.
- Modifying or staging `research/EXTERNAL_PROJECTS_BACKLOG.md` as part of
  RC-021.
- Changing `solver`, `core`, `engine_v2`, or any Cargo dependency.
- Weakening the durability contract, the terminal evidence lock, or the
  read-only guarantee of `--verify`.
- Treating Obsidian, generated indexes, summaries, chat logs, or model memory as
  a source that outranks Git-tracked canonical documents.
