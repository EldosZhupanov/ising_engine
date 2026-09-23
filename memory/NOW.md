---
id: memory-now
kind: live-state
status: active
authority_scope: current-task
updated: 2026-09-23
immutable: false
---

# NOW — the only active project state

This file is the sole repository authority for what is in flight. Always verify
its claims against `git status --short` and the current branch before acting.

## QOBLIB — Quantum Optimization Benchmarking Library 2026 Integration — 2026-09-23

The user proposed testing against QOBLIB (Nature Computational Science, 2026; IBM Quantum & Zuse Institute Berlin / ZIB-AOPT):
- Implemented `src/bin/qoblib_mis_benchmark.rs` evaluating official Maximum Independent Set (07-independentset) instances in DIMACS format with official QOBLIB zero-collision feasibility verification.
- Downloaded official instances to `benchmarks/qoblib/instances/`:
  - `sloane_1dc_64`: UltimateSolver finds exact Gurobi optimum (10) in 1.04 s (0 violations).
  - `sloane_1dc_128`: UltimateSolver finds exact Gurobi optimum (16) in 2.32 s (0 violations).
  - `sloane_2dc_128`: UltimateSolver finds exact Gurobi optimum (5) in 4.03 s (0 violations).
  - `socfb-haverford76` (1,446 nodes, 59,589 edges): UltimateSolver achieves 280 (99.3% of official best-known 282) in 39.4 s (0 violations).
- Quality gates: PASS (cargo check, cargo test --release, cargo clippy, cargo fmt).

## SK-PARISI — Sherrington-Kirkpatrick Parisi Ground State Challenge COMPLETE — 2026-09-23

The user requested execution of the hardest global benchmark and physical frontier, followed by an adversarial peer-review audit.
- Exhaustive Verification: Added `test_sk_energy_exhaustive_equivalence` in `tests/test_energy.rs` asserting $|E_{\text{QUBO}}(x) - E_{\text{SK}}(\sigma(x))| < 10^{-11}$ and all 10 single-flip deltas match across all $2^{10} = 1024$ states. Confirmed CSR symmetric matrix factor 0.5 in `calculate_total_energy`.
- Algorithmic Rigor: Refactored `src/bin/sk_parisi_benchmark.rs`:
  - Replaced heuristic D-Wave formula with exact graph-theoretic treewidth theorem: $tw(K_N) = N - 1$. Since $tw(Z_{12}) \le 200$, direct minor embedding of $K_{256}$ and $K_{512}$ on D-Wave Advantage2 $Z_{12}$ is topologically ruled out.
  - Multi-start Greedy (20 restarts): converges to $e \sim -0.705 \dots -0.732$, matching quenches in SK literature (Folena et al. 2024: $-0.708 \dots -0.735$).
  - Evaluated on ensemble of independent disorder realizations (mean $\pm$ std dev).
  - Clarified finite-size reference $\langle e_0(N) \rangle$ as an empirical ensemble reference, not an instance-specific lower bound.
  - Acknowledged Andrea Montanari's IAMP polynomial-time algorithm ($C(\varepsilon)N^2$) and cited Talagrand (2006) and Auffinger & Chen (2017).
- Results:
  - $N=64$ (5 instances): Greedy $-0.7054 \pm 0.0328$, UltimateSolver $-0.7188 \pm 0.0263$ (mean time 756 ms).
  - $N=128$ (5 instances): Greedy $-0.7325 \pm 0.0183$, UltimateSolver $-0.7467 \pm 0.0146$ (mean time 3.2 s).
  - $N=256$ (3 instances): Greedy $-0.7084 \pm 0.0063$, UltimateSolver $-0.7401 \pm 0.0080$ (mean time 15.0 s).
  - $N=512$ (1 instance, 130,816 couplings): Greedy $-0.7293$, UltimateSolver $-0.7533$ (time ~85–92 s).
- Quality gates: PASS (cargo check, cargo test --release, cargo build --release --bins, cargo clippy, cargo fmt, git diff --check).

## LAYA-001 — semantic reconciliation pilot COMPLETE — 2026-09-23

The user explicitly requested execution of the LAYA-001 pilot. The frozen exploratory
pilot ran across 24 synthetic groups (192 decisions).
- Quality gates: PASS (zero mismatches on encoding, returned energies, exact and reduced optima).
- Result: Raw Laya produced 26 constraint violations (only 10/24 feasible groups, 84.4% accuracy).
- UltimateSolver (with CD005) eliminated 100% of violations (0 violations, 24/24 feasible groups),
  recovered 19 corrupted bits with 0 false damages, and achieved 94.3% accuracy (matching exact oracle).
  Full report: [`research/laya_semantic/RESULT.md`](../research/laya_semantic/RESULT.md).
- Presolve: `full_presolve` completely fixed all 24 groups analytically (0 residual states).
No root src changes; preserve the pre-existing ultimate.rs whitespace edit and
all unrelated untracked research. Older task selections below are historical.

## Explicitly resumed: cross-domain mechanisms with Ising — 2026-09-17

The user explicitly selected the five-step mechanism map / boundary / transfer /
prior-art falsification / minimal experiment workflow and requested both HYPODIVE
skills. This lifts the earlier audit stop for this scoped investigation. Do not
restart EXP001 or automatically execute the uncommitted EXP002 draft.
Active bundle: `research/CANDIDATE_IDEAS_LEDGER.md`, `research/breakthrough/cd005/`.
First cycle CD001/H11: rank-only response-count claim NO-GO (commit `20ce993`).
Second cycle CD002/H12: discrete gauge synchronization vs spectral sync NO-GO (commit `a900575`).
Third cycle CD003/H13: precision-rank bounded response N_resp <= (2bK+1)^r CONFIRMED (commit `b37c1c3`).
Fourth cycle CD004/H14: native soft-conflict learning cuts BnB nodes by 33-54% CONFIRMED (commit `f12ce06`).
Fifth cycle CD005/H15 (Option D: Edge-Restricted 2-Opt Escapes) completed and confirmed:
Theorem 1 verified (0 violations), escapes 93-97% of 1-opt local traps with up to 24.8x speedup over O(N^2) scans.
Phase 1 Integration: CD005 operator integrated into `src/solver/local_search.rs` and `UltimateSolver` (commit `895835e`).
All unit/integration tests (199 passed, 6 local search tests including Theorem 1 completeness), release binaries, clippy, and fmt green.
All candidate ideas maintained in `research/CANDIDATE_IDEAS_LEDGER.md` and `research/SYNTHESIS_CD_SOLVER_ARCHITECTURE.md`.
Preserve all unrelated research and unfinished earlier records.

The audit stop below is historical and superseded within this explicit scope.

## Audit stop — 2026-09-17

The latest explicit user instruction is: «аудить не надо делать стоп аудит».
Do not start or resume repository audits, independent review, falsification
passes or EXP002 under an automatic goal continuation. Resume those activities
only after an explicit user instruction lifting this stop. This does not cancel
the broader research objective or mark it complete. Both requested HYPODIVE skills
are already installed; their presence does not authorize an audit.

Preserved checkpoint: EXP001 has a completion marker for 4,200 rows at
`research/breakthrough/exp001/complete.json`; no rerun is required. The untracked
EXP001 result and EXP002 protocol are drafts, not committed/frozen records.
Post-experiment validation remains unfinished, so no new superiority claim is
licensed. The previously written “in progress” statement below is superseded.
No new experiments or scientific checks were run while recording this stop.
Preserve unrelated `research/fundamental_ai/` and `results/rc027/` work.

## Current user-authorized research expansion — 2026-09-11

The current conversation explicitly requests a repository-wide algorithm audit,
including `engine_v2`, ten mathematical hypotheses, and autonomous isolated
experiments. This supersedes the old task-selection restriction below, not any
binding protocol or historical outcome. The active task bundle is
[research/breakthrough/RESEARCH_STATE.md](../research/breakthrough/RESEARCH_STATE.md),
[the audit](../research/breakthrough/AUDIT.md), and
[EXP001](../research/breakthrough/EXP001_PROTOCOL.md).

**Resumed through the goal control on 2026-09-12:** the previously blocked
thread goal is active again. The saved source state is unchanged at `fdec0df`.
The audit and ten hypotheses are complete at the documented algorithm-surface
scope. EXP001 preregistration was committed as `aee6ea8` before prototype code or data.
The isolated prototype and analysis instrument now pass 9 Rust mathematical
tests and 3 Python screen tests; independent instrument review returned PASS
(no HIGH/MEDIUM findings; reviewer repeated all 12 tests).
Instrument committed as `526ddd3`; the single registered 4,200-row run is now
in progress under `research/breakthrough/exp001/`. Do not launch it again or
change the frozen source/protocol. Next: inspect completion and analyze the
registered screens, then investigate any unexpected large effects. The user requested and received preliminary findings on 2026-09-12 and explicitly directed continued research without a change of direction. See [HANDOFF.md](../research/breakthrough/HANDOFF.md) for the preserved checkpoint.

### Parallel Isolated Research: `research/fundamental_ai/` (2026-09-12)
- Autonomous exploration of higher-order tensor energy dynamics ($p \ge 3$) as AI computational primitives.
- **EXP-TEN-001** (Associative Scaling): Completed across $N \in \{128, 256, 512\}$. Published in `EXP_TEN_001_RESULT.md`.
- **EXP-TEN-002** (Adversarial Falsification): Completed across 6 data suites. Model C proved identical to degree-3 Polynomial DAM and collapses under correlation where 1-NN achieves 100%. Published in `EXP_TEN_002_RESULT.md`.
- **EXP-TEN-003** (Latent Compression $P \gg R$): Completed. Discovered the Sign-Erasure Pathology of odd-degree energy models ($p=3$); linear SVD outperforms CP-3 on unseen grammar ($0.773$ vs $0.322$). Published in `EXP_TEN_003_RESULT.md`.
- **EXP-TEN-004** (Relational Composition): Completed. Confirmed inference-time compute scaling ($T=1: 0.9\% \to T=2: 100.0\%$), while pairwise Hopfield collapses to $0.0\%$. Published in `EXP_TEN_004_RESULT.md`.
- **EXP-TEN-006A & EXP-TEN-006A-R Audit** (Prior-Art Reset & Adversarial Audit): Completed across 4,410 audit trials. Overclaims retracted. Audit proved LearnedDualEnergy was inert (Rescue Rate = 0.7%, identical to zero-interaction ablation, p=1.000). The cyclic task was algebraically reduced to Permutation Synchronization over S_d (Pachauri et al. 2013). Classical Spectral Sync (89.4%) and Loopy Min-Sum (95.1%) outperform candidate (79.6%). Published in `EXP_TEN_006_PRIOR_ART_RESET.md`, `EXP_TEN_006_STABILITY_THEORY.md`, and `EXP_TEN_006_NOVELTY_REVIEW.md`.
- **EXP-TEN-006B** (Rigorous Energy & Contractive Synchronization Falsification): Completed. Implemented and mathematically verified Exact Analytical BPTT (TRAIN-A) and Exact Analytical Equilibrium Propagation (TRAIN-C). Pilot Gate failed decisively across all continuous models (Net Rescue <= +3.7% vs threshold > +10.0%). Discovered the Inertia-Erosion Dilemma: ln-cosh potentials create zero-crossing vanishing gradient barriers causing total inertia (Rescue Rate = 0.0%), while linear EBMs and contractive GNNs cause symmetric error diffusion that damages clean bits at nearly the same rate it rescues corrupt bits (Damage Rate 18.9%, Rescue Rate 22.6%). Classical Spectral Synchronization (+40.1%) and Loopy Min-Sum BP (+40.6%) decisively dominate continuous neural relaxations. Published in `EXP_TEN_006B_RESULT.md`. All root production code and invariants remain untouched.

The pre-existing untracked `results/rc027/` remains untouched and uninspected.
RC021 terminal evidence and all existing binding records remain immutable.
`NOW.md` remains the sole project handoff; RESEARCH_STATE is an experiment ledger.
Full-worktree memory validation has a pre-existing uncatalogued RC027 file;
validate the intended-change tracked-source snapshot separately and report both.

The sections below preserve prior research context. Their RC027 “active task”
labels do not select the new task; its frozen protocol still governs RC027 data.

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

1. **The architecture line is closed. Do not open a sixth cycle on it.** Its
   three natural questions have all been asked and answered:

   | question | cycles | answer |
   |---|---|---|
   | is the mechanism inventory the gap? | RC-003, RC-023, RC-024 | effects an order of magnitude under the 1 % floor |
   | is G-Set's degeneracy the reason? | RC-025 | no — a weight-diversity ladder is flat |
   | were we measuring it outside the search it belongs in? | RC-026 | **partly yes** — and it changes nothing that matters |

   RC-026 is the only one that found a signal: embedding path relinking k times
   inside one search, at an unchanged 50-sweep budget, gives Spearman ρ = +0.312,
   **p = 1.0e-4**, positive on 26 of 30 instances. It is also `gain ∝ k^0.18`
   against a per-round cost of `d²/2` evaluations — 25× the rounds buys 1.77× the
   gain, and 1 % would need of order 3.6e5 rounds. **The direction that helps is
   the direction that cannot be afforded.** That is a cleaner ending than five
   nulls, and it is the ending.

   Everything is recorded: `results/rc026/RESULT.md`, `OPEN_PROBLEMS.md` §0h.

2. **The one asset this line produced is the never-worse invariant.** 478 wins,
   **0 losses** in 630 paired trials across two corpora, three preregistrations,
   nine seeds and five embedding depths. Whatever comes next, that property — a
   mechanism that cannot make things worse, proven before it was run — is the
   template worth reusing, not the mechanism itself.

3. **Wall-time work is open and the protocol now exists.**
   `research/PERF_INCREMENTAL_RELINK.md` made the project's first entitled
   wall-time claim: incremental relinking, **5.72×** measured against the host's
   own calibrated null, bit-identity proven by re-running two frozen commands and
   matching the SHA-256 published in their `RESULT.md` files — 540 registered
   rows. `CLAUDE.md` §9 now carries the protocol: calibrate the host, keep the
   previous binary and interleave the arms, prove identity against something
   published earlier, declare whatever moved.

4. **ACTIVE TASK — RC-027, the budget axis.** Every one of the six cycles ran
   50 sweeps; a mechanism that only pays on a long run would be invisible to all
   of them. It is the last cheap question this project has not asked and the only
   one left that could overturn the closure. Do the candidate and control curves
   **converge, diverge, or stay parallel** as the budget grows?

   **The full brief is [`NEXT_TASK_RC027.md`](NEXT_TASK_RC027.md).** Read it
   before anything else: it carries the design, the mandatory controls, the
   non-negotiable procedure, the six traps that have already cost this project
   time, the tools that now exist, and the hard prohibitions. Preregister before
   any code, as the last four cycles did.

2. **Decide item D** in `OPEN_PROBLEMS.md` §0b: does exit 4 mean "the evidence
   is broken" or "something went wrong"? A sentinel that could not run is not a
   damaged journal. A contract question; it belongs to the maintainer.
3. **C10 is diagnosed, and the wall-time block is not where the record said.**
   `research/RC021_C10_DIAGNOSIS.md`: C10's 1 % bound sits at the *median* of its
   own null distribution — two identical arms exceed it in 20 of 40 replications
   — so its verdict carried almost no information about the diagnostics. The
   outcome (INSTRUMENT-INVALID) stands; the recorded cause does not, and the
   diagnostic overhead remains **unmeasured** rather than shown to be zero.
   `src/bin/host_timing_calibration.rs` now measures what this host can resolve
   **before** a threshold is frozen: **~25 % cleanly, ~5 % marginally, 1 % not at
   all**, and `--pin` reports a known +25 % injection as +48 %. Consequences are
   recorded in `CLAUDE.md` §9 and `OPEN_PROBLEMS.md` §0f. A wall-time claim needs
   a fresh preregistration with a bound at or above 5 % and a mandatory null arm;
   C10 itself stays frozen exactly as written.

4. **The external-technology review is settled** —
   `EXTERNAL_PROJECTS_BACKLOG.md` §14. All thirteen August-2026 sources were
   fetched and verified real. Twelve are "no": they speed up writing code, which
   has never been the limiting step here. The one that changes anything is the
   AGENTS.md study, and it was reported to us inverted — the harm is in
   **auto-generated repository overviews**, not context files as a class, while
   hand-written instructions are followed and help. Checking our own overviews
   ended the question early: `.claude/context/` is **untracked**, has no
   generator and no freshness gate, was last written 2026-07-05, and its
   `modules.txt` still describes a project with **no `engine_v2`**. `CLAUDE.md`
   §7–§8 ranked that map above ripgrep, and the same paper shows such
   instructions are obeyed. Corrected in place: `rg` and `cargo metadata` now
   rank first because they cannot go stale. Do **not** regenerate
   `.claude/context/` without evidence that a maintained overview helps.

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
