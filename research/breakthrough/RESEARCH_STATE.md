# Breakthrough research state

**2026-09-25 correction:** [CD004-R](cd004_recheck/RESULT.md) invalidates the
CD004 exactness and acceleration claim below. Legacy backjumping can miss the
optimum; an exact core-cache repair had no node or time win in 120 new cases.
The synthesis proposal is historical. CD003 and CD005 are unaffected by this
specific check. Follow [NOW](../../memory/NOW.md) for the active decision.

**2026-09-19: CD005/H15 (Option D: Edge-Restricted 2-Opt Escapes) completed and confirmed.**
Proved Theorem 1 (scanning strictly $|E|$ edges is 100% complete for finding all improving 2-flips at 1-opt minima, 0 violations).
Escaped 93.3% to 96.7% of false 1-opt traps on frustrated spin glasses with up to 24.8x speedup over O(N^2) pair scanning.
Status: CONFIRMED ALGORITHMIC ADVANTAGE (GO).
All candidate ideas preserved in [research/CANDIDATE_IDEAS_LEDGER.md](../CANDIDATE_IDEAS_LEDGER.md).
Next: synthesize CD003, CD004, and CD005 into unified platform architecture.

**2026-09-19: CD004/H14 (Option B: Native Soft-Conflict Learning in Ising) completed and confirmed.**
Evaluated deletion filtering and non-chronological backjumping on frustrated Ising spin glasses.
Achieved 33.1% to 54.4% reduction in explored search nodes with 100% exact optimality.
Status: CONFIRMED ALGORITHMIC ADVANTAGE (GO).

**2026-09-19: CD003/H13 (Option A: Precision-Rank Bounded Response) completed and confirmed.**
Proved and verified that conditional ground state response count is bounded by $N_{\text{resp}} \le (2bK+1)^r$.
Resolves the H11 negative result: low rank compresses response count polynomially if and only if bit precision
$p \ll b/r$. Verified on 14 grid configurations (0 violations). Status: CONFIRMED THEOREM.

**2026-09-19: CD002/H12 (Option C: Discrete Gauge Synchronization) completed and falsified.**
Adversarial cycle witness proved $K \cdot |S_d|$-fold ground-state degeneracy on cycles,
and zero advantage over classical Spectral Synchronization on triangulated graphs. Verdict: NO-GO.

**2026-09-17: explicitly resumed by the user for cross-domain mechanisms and
Ising with both HYPODIVE skills.** Prior stop statements below are historical.
CD001/H11 completed an exact constructed falsification: 18 rows / 16,380 energies,
rank-only response-count claim NO-GO. Git freeze completed in commit `20ce993`.

**2026-09-17: audit stopped at the user's explicit request.** Follow
[memory/NOW.md](../../memory/NOW.md); automatic goal continuations do not reopen
review or EXP002. EXP001 has a completion marker for 4,200 rows; the old run-in-
progress statements below describe the earlier checkpoint and are superseded.
The uncommitted result/protocol drafts remain pending. No new benchmark or
scientific validation was performed to record this stop; no superiority claim.

**Resumed 2026-09-12 through the goal control**, which is active after the prior block. Preregistration committed as `aee6ea8`; the isolated prototype is implemented and independently reviewed PASS before data.

Project handoff authority remains [memory/NOW.md](../../memory/NOW.md). This file is the requested research ledger, not a second project-task authority.

Baseline: unchanged `UltimateSolver` from `fdec0df`, with existing QPBO/probing, random initialization, PT and 1-opt finish. No experimental candidate has yet established superiority. Best validated method remains the baseline; no breakthrough claim.

Completed: [algorithm audit](AUDIT.md), [10 hypotheses](HYPOTHESES.md), [prospective symmetry-scope correction](GAUGE_SCOPE.md). Independent Explorer checked historical algorithms and cached branches. `engine_v2` is explicitly covered, including state, kernels, registry, runtime, selector and research models.

| Experiment | Hypotheses | Design | Status |
|---|---|---|---|
| EXP001 | H01 + H07 | full 2³ factorial: leaf response, degree-2 response, 2-opt refinement | preregistered `aee6ea8`; prototype: 9 mathematical + 3 analysis tests pass; instrument `526ddd3`, independent review PASS; single registered run in progress |

Previously weak/rejected directions: consensus freezing (RC-001), generic covariance/co-flip move synthesis (RC-003), hard-vs-soft memory spelling (RC-023), naive relinking additions (RC-024–026). Their scoped negative records are preserved. No new hypothesis is experimentally rejected yet.

Promising untested: conditional elimination, exact tree moves, gauge-conditioned distributions, constraint-tangent moves. Unexplained candidate outcomes: none yet. Audit anomaly: published “marginal vacuity” corollary is broader than its premises; exact counterexample derived. Benchmark anomaly: calibration null drift is large, so small timing advantages will not be claimed.

Next: finish the single run already in progress; analyze all 4,200 frozen rows, failures and compute accounting; independently review results. Do not relaunch into `exp001/`. Then select H02 or H04 based on information gained, not the most flattering result.
