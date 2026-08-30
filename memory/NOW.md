---
id: memory-now
kind: live-state
status: active
authority_scope: current-task
updated: 2026-08-30
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

**RC-024 is complete. Registered outcome: WORKS BUT IMMATERIAL.**

Endpoint-guided path relinking was preregistered (`28710dc`), implemented and
mutation-verified (`4af912e`), and run once as registered. Ninety paired
observations over 30 G-Set instances at seeds 101/102/103:
**69 wins, 0 losses, 21 ties**; mean gain **+0.105 %**, median +0.077 %,
Wilcoxon **p = 5.4 × 10⁻¹³**. Not one pair of the ninety cleared the 1 %
materiality floor. Full record: `results/rc024/RESULT.md`.

The zero in the loss column is the never-worse invariant, proven on synthetic
endpoints before the run. The candidate does strictly more logical work than its
control and this host is INSTRUMENT-INVALID, so **no wall-time or equal-cost
claim is licensed** — a +0.105 % gain bought with unmeasured extra work is not a
speed result.

**Taken with RC-003 and RC-023, this is the cycle's real finding.** RC-022 named
synthesized moves as the largest mechanism gap between our corpus and the human
one — 9 of 20 external families. It is now measured twice in two independent
forms: covariance-mined collective moves (RC-003, ≈ +0.06 %) and endpoint-guided
path relinking (RC-024, +0.105 %). Both land an order of magnitude under the
floor. RC-023 found the *form* of memory not to matter at all. Three
architecture experiments in a row say the mechanism inventory is **not** where
this corpus is short — a claim about these instances at this budget, and the
hypothesis the next cycle should try to break rather than a conclusion.

RC-023's post-hoc signed/unweighted stratification **does not reproduce here**
(+0.126 % signed vs +0.100 % unweighted). That removes one way it could have
been a property of the corpus rather than of tabu memory. It neither confirms
nor refutes the original observation, which concerned a different mechanism.

## Working context

- Active branch: `feat/solver-research-upgrades`.
- RC-021 Step 6 is complete, committed as
  `1b855b774654f85a326baf37436262e9f5322e5b`, and an independent read-only
  review of it returned PASS.
- RC-021 Step 7 is complete and committed as
  `3493ed8`; its independent review blockers and adversarial follow-up findings
  are closed, and the full source gate set is green.
- **Step 8 is committed as `35fb5fa`, with `ad24013` and `e9a0450` on top.**
  Seven independent review rounds ran. Every HIGH and MEDIUM was fixed and
  mutation-verified; the deliberately open findings are recorded in
  `memory/OPEN_PROBLEMS.md` §0b.
- **RC-021 executed once on instrument commit `e9a0450` and is terminal.** The
  mandatory controls ran; eleven passed and `C10` failed because diagnostic
  overhead was `0.010802221586322025`, above the frozen `0.01` limit. The
  complete closure is Class I `INSTRUMENT-INVALID`, exit 3. No qualification
  session began, no paired-spread datum exists, and no Class II host verdict is
  licensed. The durable publication is
  `research/RC021_INSTRUMENT_INVALID_RECORD.md`.
- RC-021 closeout is committed as `af8da91` and `c110e2b`: the immutable Class
  I record is published, all formerly missing synthetic state-machine rows are
  covered by non-vacuous tests, the named mutation is killed, and the raw
  terminal evidence remained byte-identical across the full gate run.
- The durable-memory series is integrated on this branch as commits
  `983497d..d5880e3`.
- The integrated memory state at `e824b7f` was independently reviewed and
  returned PASS.
- `experiments/rc021/` is the existing gitignored terminal evidence directory;
  its closure and five input artifacts are bound by hashes in the abort record.
- RC-022 is closed: twenty MQLib families occupy at least eight RC-004 cells.
- RC-023 is closed with registered outcome (a): hard and soft memory forms were
  indistinguishable at the frozen budget. Synthesized moves are the next
  untested architectural axis.
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

1. An Explorer map proves the exact existing synthesized-move implementation,
   runtime/registry call sites, reusable harness and do-not-touch boundary.
2. A binding RC-024 preregistration is committed before candidate code or
   result data exists.
3. Candidate and control differ on one named synthesized-move mechanism, use
   identical seeds and logical budgets, and have deterministic backend/energy
   equivalence tests.
4. The registered experiment runs once, its complete outcome is published even
   if null or adverse, and prohibited wall-time/production claims are absent.
5. An independent review and every applicable source, memory and immutable-file
   gate pass; RC-021 terminal evidence remains byte-identical.

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
soft decaying bias cannot?

**RC-023 answered it: no.** `research/PREREG_RC023_MEMORY_FORM.md` was committed
(`74e425e`) before any code; `tabu_sweep` was built as `history_field`'s twin
(`34eab7c`), differing only in the form of the memory, with the shared RNG
stream pinned by a test so the comparison isolates that one variable. Two-sided
Wilcoxon over 30 paired G-Set scores: **p = 0.87** as registered, **p = 0.63**
when the harness's 3-decimal printing is removed as a possible artifact. Mean
difference −0.0018, median +0.0010 — they disagree in sign. Registered outcome
(a): the two forms are indistinguishable at this budget on this corpus. Full
record and its prohibited claims: `results/rc023/RESULT.md`.

The result is worth as much as a win. RC-022 named the *form* of memory as the
surviving Ax2 route after the Z₂ theorem closed the marginal cell. That route is
now measured and it is flat — which means the gap between our corpus and MQLib's
is not the tabu spelling, and the next candidate must come from a different axis
(synthesized moves, 9 of 20 families, is the largest remaining one).

**The post-hoc observation that must not be promoted without its own
preregistration:** the sign of the per-instance difference separates perfectly
by whether the instance carries negative weights — tabu is lower on 6/6 signed
(±1) instances and 3/24 unweighted ones, Mann–Whitney U = 0. Density is
excluded (G48/G49/G50 match G11/G12/G13 on topology and m/n and show ~0), but
the matched control is **saturated** — nine of nineteen operators sit at its
optimum — so it cannot discriminate and the confound is only partly excluded.
RC-004 retracted Law 2 for exactly this shape of error. Treat it as a
hypothesis, and note that its first design obstacle is finding a control corpus
that is not already solved.

## Next action — in this order

1. **Decide what the three flat cycles mean before running a fourth.** RC-003,
   RC-023 and RC-024 each added a mechanism the human corpus has and we lacked,
   and each returned an effect an order of magnitude under the materiality
   floor. Either the remaining unoccupied cells hold nothing either, or the
   corpus cannot express the difference. `OPEN_PROBLEMS.md` §1 already records
   that G-Set is degenerate on two axes. **The next preregistration should test
   the corpus, not another operator** — a candidate that is immaterial on G-Set
   but material on an instance family G-Set cannot express would settle it, and
   a candidate flat on both closes the mechanism hypothesis honestly.
2. **Decide item D** in `OPEN_PROBLEMS.md` §0b: does exit 4 mean "the evidence
   is broken" or "something went wrong"? A sentinel that could not run is not a
   damaged journal. A contract question; it belongs to the maintainer.
3. **C10 remediation is deferred by the user's decision** and is the only route
   to any wall-time claim. Nothing in RC-022/023/024 needed it, which is why
   three cycles ran on an unqualified host without a single prohibited claim.

RC-021 is an instrument, not a result, and it is still unqualified on this host
(C10). That is why RC-023 makes **no wall-time claim of any kind** — only
paired quality at identical seeds, which needs no qualified host.

## Forbidden during this task

- Re-running RC-021, any real control, any qualification session, or a sentinel
  on G11.
- Editing, deleting, repairing, or supplementing the existing
  `experiments/rc021/` terminal evidence.
- Editing or moving any existing binding preregistration, amendment, or cycle
  record; RC-024 gets a new prospective preregistration and result.
- Fixing the non-blocking F1/F2 test-hardening findings "while here".
- Starting any six-session execution under the spent RC-021 protocol.
- Changing the scalar solver family, `core`, Cargo dependencies, public API,
  `UltimateSolver`, or production auto-routing.
- Inspecting or changing registered result seeds after the RC-024
  preregistration is committed.
- Making a wall-time, equal-cost, universal-optimizer, or production-superiority
  claim from RC-024.
- Starting Soup/LoRA/video-model work inside this research cycle.
- Weakening the durability contract, the terminal evidence lock, or the
  read-only guarantee of `--verify`.
- Treating Obsidian, generated indexes, summaries, chat logs, or model memory as
  a source that outranks Git-tracked canonical documents.
