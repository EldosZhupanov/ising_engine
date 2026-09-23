# LAYA-001: semantic decisions with verified discrete reconciliation

Date: 2026-09-23. Locally preregistered exploratory capability pilot, not external
preregistration, a production release, an accuracy confirmation or a speed claim.
This protocol is frozen before candidate code and task inference. Corrections
require a prospective amendment. Earlier English billing smoke was seen; no
manufacturing pilot outputs have been accessed.

## Question and scope

Can actual local Laya probabilities feed the unchanged UltimateSolver, with
independent feasibility/energy checks? Does existing presolve preserve the optimum
and shrink the residual conflict graph on this tiny semantic task distribution?
No solver-family change, training, CUDA installation or new dependency is needed.

## Fixed pilot

24 synthetic English manufacturing snapshot groups, 8 binary running-status
decisions each. Group seeds 9242300..9242323, Python random.Random. For every pair
i<j draw an edge with probability 0.20 for even group IDs, 0.45 for odd IDs;
conditional on an edge, exclude simultaneous activity with probability 0.6,
otherwise impose i implies j. The all-zero assignment is always feasible.
Enumerate all 256 assignments and uniformly sample one feasible world as truth.
For each bit sample one of these literal message templates, independently:

Positive:
- The {job} operation is running now.
- The {job} operation has started and is still in progress.
- The {job} operation was paused earlier, but it has resumed and is running now.
- The plan was to cancel {job}, but the cancellation was reversed; it is running now.

Negative:
- The {job} operation has not started yet.
- The {job} operation finished yesterday and is not running now.
- The {job} operation was running earlier, but it has been stopped.
- The {job} operation is scheduled for tomorrow; it is not running now.

Job labels are job_0..job_7. No other text augmentation/noise. Ask each isolated
message via noul: "Is the operation running now?". The input contains the message
only: no truth, constraint graph, group ID or difficulty label. One SDK call per
group with 8 questions; each question's instructions includes its own message,
shared state is the fixed string "Manufacturing status classification.".
The input separation prevents model access to constraints; those are shared by
all reconciliation arms. Corpus construction gives clean valid rules and explicit
synthetic truth; usefulness on real text is NOT inferred from this pilot.

Use existing English snapshot 5e7b2b1b8ca2ecdd3f2322d94069c9b6ce7e844b, laya 0.3.7,
CPU, torch threads=2, torch seed=23. Metadata copied into the run directory to
avoid modifying HF originals; weights referenced read-only. Network offline.
Use returned noul probabilities, not confidence. Clip to [1e-4,1-1e-4].
No domain calibration is fitted in this pilot; report Brier score descriptively.

## Objective and mathematical checks

For x in {0,1}^8, unary cost is -sum log P(x_i). Let
h_i=log((1-p_i)/p_i), c=-sum log(1-p_i). Penalty M=1+sum |h_i|.
Exclusion contributes M*x_i*x_j; implication contributes M*x_i*(1-x_j).
The range of the unary function is sum |h_i|. Since at least one feasible state
exists and each violated rule costs at least M, every global minimizer is feasible.
This proof does NOT promise a finite-budget heuristic returns a feasible state.
Evaluate constraints independently of QUBO. Exhaustively check the encoded energy
against direct log-likelihood plus counted violations, tolerance 1e-8.

## Arms, fixed before inference

- raw: independent p>=0.5.
- all_zero: feasible majority-style baseline (no learned scores).
- greedy: start from all zero; consider variables in descending p, index tie;
  set a bit to one only if p>0.5 and every rule remains satisfied. One pass.
- exact: brute-force full penalized QUBO, lowest integer mask on ties.
- ultimate: existing UltimateSolver(10.0,0.05,10,20,seed=1000+group), with_2opt(true).
  Internal built-in presolve/decomposition remains enabled; NOT an unreduced arm.
- reduced_exact: existing first-order fixings (diagnostic) and existing
  full_presolve (operational), then existing connected_components/extract_component;
  exhaustive solution of each residual component and reconstruction.

No hidden repair of ultimate outputs. Invalid assignments are reported as invalid.
Report exact energy gap for every arm. Reduced_exact must match full exact energy,
not necessarily its assignment when ties exist. No standalone timing advantage
is inferred from fewer enumerated states, and presolve work must not be hidden.
Record first-order and full fixing counts, component sizes, sum 2^component_size,
and the full enumeration count. Empty residual graph needs zero enumerated states.

## Metrics, failure and stopping

Primary capability gates: all inputs produce typed finite probabilities; 24 groups
complete; encoding check has zero mismatches; reduced exact has zero optimum
mismatches; every reported solver energy matches Python recomputation.
Feasibility of heuristic output is a measured outcome, not assumed.
Report per-arm bit accuracy, exact-group accuracy, feasible-group fraction,
violation counts, recovered and damaged bits relative to raw (same 192-bit
denominator), and energy gaps. Report reduction distributions. Any accuracy gains
are descriptive; no p-value, superiority/noninferiority or new scientific novelty
claim. Independent units are groups, not questions; templates are shared.
No training or tuning split; all results are an exploratory pilot. The exact
oracle is deliberately stronger/more expensive, not an equal-cost baseline.

Stop after the fixed 24 groups and handoff. Any failed correctness gate stops
scientific interpretation. Preserve failed/partial files; do not overwrite/rerun
after seeing outputs under this protocol. Infrastructure failures may resume only
completed-file checkpoints without repeating successful inference; disclose them.
The next real-domain study requires a new protocol and independent inputs.

## Freeze and reproduction

Commit this protocol before code, then commit tested instrument before pilot.
Build/run from a clean detached checkout of the instrument commit to exclude
unrelated dirty source. Raw answers, input corpus, package/source/weight hashes,
parameters, stdout and analysis are separate result artifacts. Never upload data.
Scientific/practical stop: if simple greedy matches exact on this corpus, or
constraints introduce no useful correction, report that ceiling explicitly;
do not alter templates or noise to manufacture an advantage.
