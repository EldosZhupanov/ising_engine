# Project chronology

This is a navigation timeline, not a substitute for Git or the linked records.
Commit ancestry establishes ordering; calendar dates are descriptive.

## Direction and architecture

| Date | Commit | Event | Durable source |
|---|---|---|---|
| 2026-07-14 | `d3bd6be` | Project direction ratified | [`ISING_ENGINE_CONSTITUTION.md`](../research/ISING_ENGINE_CONSTITUTION.md) |
| 2026-07-15 | `e28fe3b` | Mission recorded | [`SOUL.md`](../SOUL.md) |
| 2026-08-19 | `d18af54` | First repository memory/index layer committed | [`INDEX.md`](INDEX.md) |
| 2026-08-26 | `e3669be` and descendants on `docs/memory-architecture` | Durable-memory authority, catalogue, and startup path established | [`ADR-0011`](../research/architecture/ADR/ADR-0011-durable-project-memory.md) |
| 2026-08-27 | `983497d..d5880e3` | Durable memory integrated into `feat/solver-research-upgrades` after the reviewed Step 6 | [`ADR-0011`](../research/architecture/ADR/ADR-0011-durable-project-memory.md), [`NOW.md`](NOW.md) |
| 2026-08-27 | `e824b7f` | Integrated durable-memory state independently reviewed PASS; current-task authority returned to RC-021 Step 7 | [`NOW.md`](NOW.md) |
| 2026-08-27 | `3493ed8` | RC-021 Step 7 decision, reservation-first finalization and read-only verification completed after adversarial review | [`NOW.md`](NOW.md) |
| 2026-08-28 | `35fb5fa..e9a0450` | RC-021 Step 8 CLI completed; seven review rounds closed every HIGH and MEDIUM finding before execution | [`OPEN_PROBLEMS.md`](OPEN_PROBLEMS.md) |
| 2026-08-30 | instrument commit `e9a0450` | RC-021 closed Class I `INSTRUMENT-INVALID` after mandatory control C10 failed; no qualification session began | [`RC021_INSTRUMENT_INVALID_RECORD.md`](../research/RC021_INSTRUMENT_INVALID_RECORD.md) |
| 2026-08-30 | `165543c`, corrected by `d852b19` | RC-022 census found at least eight occupied architecture cells and selected memory form as the first measured gap | [`RC022_ARCHITECTURE_CENSUS.md`](../research/RC022_ARCHITECTURE_CENSUS.md) |
| 2026-08-30 | `762d1bc` | RC-023 found hard and soft memory forms indistinguishable at the frozen budget; synthesized moves became the next untested axis | [`RESULT.md`](../results/rc023/RESULT.md) |
| 2026-08-30 | `af8da91`, `c110e2b` | RC-021 Class I record published and its final synthetic CLI state-machine coverage closed with mutation proof; terminal evidence remained byte-identical | [`RC021_INSTRUMENT_INVALID_RECORD.md`](../research/RC021_INSTRUMENT_INVALID_RECORD.md), [`OPEN_PROBLEMS.md`](OPEN_PROBLEMS.md) |
| 2026-08-30 | `28710dc`, `4af912e` | RC-024 preregistered and its candidate committed with the four PREREG §5.4 endpoint obligations closed under eight killed mutations | [`PREREG_RC024_PATH_RELINKING.md`](../research/PREREG_RC024_PATH_RELINKING.md) |
| 2026-08-30 | RC-024 registered run | Endpoint-guided path relinking won 69 of 90 paired trials and lost none, at +0.105 % mean — reliably positive, an order of magnitude under the 1 % materiality floor | [`RESULT.md`](../results/rc024/RESULT.md) |
| 2026-08-30 | `8e73ad5` | C10 diagnosed: its 1 % bound sat at the median of its own null, so the recorded failure reason was never established; host resolution measured at ~25 % clean, ~5 % marginal, 1 % unreachable | [`RC021_C10_DIAGNOSIS.md`](../research/RC021_C10_DIAGNOSIS.md) |
| 2026-08-30 | RC-025 registered run | The corpus is exonerated and the mechanism line closes; the first invocation refused on a family-name error before measuring anything, and the correction was amended in before the run | [`RESULT.md`](../results/rc025/RESULT.md) |
| 2026-08-30 | RC-026 registered run | The shape of the comparison was hiding a real effect: embedding raises the gain monotonically over five rungs at an unchanged budget, and 478 wins with 0 losses in 630 trials close the never-worse invariant | [`RESULT.md`](../results/rc026/RESULT.md) |
| 2026-08-30 | wall-time protocol | Incremental relinking measured at 5.72x against the host's own calibrated null, with bit-identity proven by reproducing two artifacts published before the change — the first wall-time claim the project was entitled to make | [`PERF_INCREMENTAL_RELINK.md`](../research/PERF_INCREMENTAL_RELINK.md) |
| 2026-08-30 | benchmark audit | The guide axis is unidentifiable on 54.5 % of 242 public MaxCut instances and G-Set is 30/30 degenerate on both axes; only 60 instances in the whole ecosystem are clean | [`BENCHMARK_DEGENERACY_AUDIT.md`](../research/BENCHMARK_DEGENERACY_AUDIT.md) |
| 2026-09-25 | working-tree audit at `11418e3` | Whole-repository map includes tracked, untracked, and selected ignored evidence. Five purported Market Split solutions failed the official checker and were retained as rejected candidates; the 41 selected inputs themselves publish checker-valid answers. | [`PROJECTS.md`](../PROJECTS.md), [`FILE_MAP.tsv`](FILE_MAP.tsv), [`EXP005 triage`](../research/EXP005_MARKETSPLIT_TRIAGE.md) |
| 2026-09-26 | EXP-TEN-006A-R recheck | Current executable reproduced all 3,360 non-timing raw rows; classical partial accuracy exceeded the candidate at K=16/T=16, but no method solved a whole graph there. Market Split prototype certificate reporting was made fail-closed. | [`EXP006A recheck`](../research/EXP006A_RAW_RECHECK.md), [`NOW.md`](NOW.md) |
| 2026-09-26 | evidence preservation | Experimental lattice source, fundamental-AI raw/source, EXP001, RC027, eight LABS checkpoints, 41 labeled Market Split inputs and provisional audit drafts were committed separately. EXP001's four derived outputs replayed byte-for-byte; LABS energies were recomputed independently, and the checker was found to lack an optimum table above N=66. | [`PROJECTS.md`](../PROJECTS.md), [`FILE_MAP.tsv`](FILE_MAP.tsv), [`NOW.md`](NOW.md) |
| 2026-09-26 | independent prototype review | Review found empty-kernel and timeout defects, fixed with regression tests at `e04301f` and `710898e`; selected gate now includes the standalone fundamental-AI crate and complete raw-design check at `47c7e07`. Lattice module-boundary and arithmetic-proof debt remains open. | [`PROJECTS.md`](../PROJECTS.md), [`ROADMAP.md`](../ROADMAP.md), [`NOW.md`](NOW.md) |
| 2026-09-26 | LABS witness publication fix | `66e9934` closes the intra-process checkpoint race, adds synced atomic replacement and isolated output directories, and checks verifier status synchronously. Four release regression tests and independent review PASS; no new search-performance claim. | [`NOW.md`](NOW.md), [`RESEARCH.md`](RESEARCH.md) |
| 2026-09-26 | LABS-Q002 completed | Protocol `524b61d`, instrument `6668e76`, raw `7449724`: 60 valid cells and independent audit PASS, memetic hunter NOT_QUALIFIED (hits 4/0/0 versus lMAts 10/8/0). No record campaign follows; budget-allocation diagnosis is next. | [Result](../research/experiments/labs_q002/RESULT.md), [`NOW.md`](NOW.md) |
| 2026-09-27 | HUBO representation gate and Laya feasibility | Protocol `7542602`, source `b9b99fe`: exact representation checks 6/6 pass; no timing claim. Laya cached checkpoint/config inspected and a training hypothesis separated from known semantic-loss/decision-focused learning. | [HUBO result](../research/experiments/hubo_representation_gate/RESULT.md), [Laya guide](../research/laya_semantic/README.md) |
| 2026-09-27 | HUBO-C001 completed | Protocol `feb9a76`, source `23d300f`, raw `a196745`: 400 valid cells, 3,487 incumbents; native MSC wins quadratic contrasts but ties native OpenJij 100/100. NOT_QUALIFIED_FOR_ADVANTAGE; next is independent corpus qualification design, not a record hunt. | [Result](../research/experiments/hubo_comparison/RESULT.md), [`NOW.md`](NOW.md) |
| 2026-09-27 | HUBO-Q002 Phase B completed, MIXED | Frozen source `5e9ea62`, raw `d5a2a63`: 240 valid cells, 4,079 incumbents, 6/12 instances and 2/4 strata qualify. Independent audit PASS; >=3/4 criterion unmet, no superiority claim or automatic new campaign. | [Phase B](../research/experiments/hubo_corpus_qualification/RESULT_PHASE_B.md), [`NOW.md`](NOW.md) |
| 2026-09-27 | HUBO-Q002 Phase A completed | Protocol `9438f15`, source `2017661`, certificates `cd2fc6e`: twelve fresh inputs, 24 exact local/global reductions and eight tests PASS. Local penalties are smaller; search improvement unmeasured, Phase B NOT RUN. | [Phase A](../research/experiments/hubo_corpus_qualification/RESULT_PHASE_A.md), [`NOW.md`](NOW.md) |
| 2026-09-28 | this decision record | Owner paused Ising Engine development and began bounded new-product discovery; solver resume point retained, no breakthrough/product claim made | [`NOW.md`](NOW.md), [`HYPODIVE_TRIAGE.md`](../research/HYPODIVE_TRIAGE.md) |
| 2026-09-28 | dual-track desk review | Direct prior art narrowed false-success verification and numerical-recycling ideas; public artifact qualification and end-to-end cost floor are the next kill-tests, with no 4–5x claim | [`HYPODIVE_TRIAGE.md`](../research/HYPODIVE_TRIAGE.md#dual-track-discovery-update--2026-09-28), [`literature`](../research/literature/dual_track_20260928.json) |
| 2026-09-28 | cross-domain problem-first screen | Nine jobs screened for alternative representations and direct prior art; three narrow qualification tracks remain, no novelty, demand or speedup established | [Discovery map](../research/PROBLEM_FIRST_DISCOVERY_20260928.md), [`NOW.md`](NOW.md) |
| 2026-09-29 | frozen SWE-bench public-artifact intake | Six selected cases failed the false-final-claim source gate (0/6 explicit final claims); a separate 3/3 patch-lineage discrepancy needs provenance checking, with no benchmark-error claim | [Intake result](../research/experiments/swebench_artifact_intake/RESULT.md) |
| 2026-09-29 | post-hoc SWE-bench trajectory lineage check | Codex and GPT-5.2-high trajectory listings match by key/ETag/size for 500 objects; three frozen trajectories match bytewise and align with the high entry's patches. No score verdict or product claim follows. | [Addendum](../research/experiments/swebench_artifact_intake/LINEAGE_ADDENDUM.md) |
| 2026-09-29 | REPRO-Bench source-intake protocol frozen | Five outcome-blind IDs and a cheap independent-report stop rule fixed before case inspection | [Protocol](../research/experiments/repro_bench_intake/PROTOCOL.md) |
| 2026-09-29 | REPRO-Bench source-intake result | 4/5 source-level numerical claim/report joins; only 2/5 exact datasets below 20 MB/file; no execution or verifier advantage | [Result](../research/experiments/repro_bench_intake/RESULT.md) |
| 2026-09-29 | REPRO-Bench small-case cost and rights gate | Neither ordinary rerun is recommended now: 51 has unreadable custom data terms; 109 needs licensed Stata for original code. Both have independent reports; estimates only, no code executed. | [Cost review](../research/experiments/repro_bench_intake/COST.md) |

2026-09-27 ER-001: protocol/instrument96b0c23, raw a91481c;120 valid cells,
1,344 checked incumbents, independent executable auditPASS. H1/H3 pass, H2 fails:
greedy equals exact on all12 real-data blocks. Production code untouched;
[capability retained, added-search value unestablished](../research/experiments/entity_resolution/RESULT.md).

## Research-cycle chronology

2026-09-27 application discovery (base `3af7c25`): user redirected to Hypodive
Mode A. Five application hypotheses mapped to actual capabilities and primary
competitors; five exact algebra checks PASS. A rank-one witness narrows CD003's
polynomial-response claim prospectively. No application benchmark, production
change or model training. [Triage](../research/HYPODIVE_TRIAGE.md) selects
entity-resolution intake, with external SAT as fallback.

| Cycle | State | First recorded | What survives | Sources |
|---|---|---:|---|---|
| RC-001 | closed/refuted | 2026-08-19 | Ensemble-consensus thermostat missed its materiality criterion | [`RC001`](../research/RC001_ENSEMBLE_THERMOSTAT.md) |
| RC-002 | closed/partial | 2026-08-19 | Initialization quality/diversity dissociation | [`RC002`](../research/RC002_INITIALIZATION_ERASURE.md) |
| RC-003 | closed/audit | 2026-08-19 | Architectural premise audit | [`RC003`](../research/RC003_ARCHITECTURE_AUDIT.md) |
| RC-004 | closed/proof | 2026-08-19 | Architecture-space impossibility results | [`RC004`](../research/RC004_ARCHITECTURE_SPACE.md) |
| RC-005 | closed/finding | 2026-08-19 | Flip density omitted from the cost model | [`RC005`](../research/RC005_COST_MODEL_BLINDNESS.md) |
| RC-006 | closed/confirmed | 2026-08-19 | Exact gradient-ledger identity | [`RC006`](../research/RC006_GRADIENT_LEDGER.md) |
| RC-007 | closed/finding | 2026-08-19 | Ordering search largely commutative in measured domain | [`RC007`](../research/RC007_OPERATOR_COMMUTATIVITY.md) |
| RC-008 | closed/audit | 2026-08-19 | Low-temperature ensemble collapse | [`RC008`](../research/RC008_MIXING_AND_ENSEMBLE_COLLAPSE.md) |
| RC-009 | closed/confirmed | 2026-08-19 | Capability passport matched backend behavior | [`RC009`](../research/RC009_BACKEND_PASSPORT.md) |
| RC-010 | closed/audit | 2026-08-19 | World-model score narrowed after leakage audit | [`RC010`](../research/RC010_WORLD_MODEL_AUDIT.md) |
| RC-011 | closed/proof | 2026-08-19 | Predictor LOO metric cannot establish transfer | [`RC011`](../research/RC011_PREDICTOR_METRIC_INVARIANCE.md) |
| RC-012 | closed/audit | 2026-08-19 | Early-stop path was non-functional; flag later made to refuse loudly | [`RC012`](../research/RC012_DYNAMICS_EARLY_STOP_AUDIT.md) |
| RC-013 | closed/mixed | 2026-08-19 | P1 refuted, P2 confirmed for kernel floors | [`RC013`](../research/RC013_KERNEL_FLOORS.md) |
| RC-014 | closed/Gate A passed | 2026-08-19 | Exact substitution differs from deletion | [`prereg`](../research/PREREG_RC014.md), [`record`](../research/RC014_COUNTERFACTUAL_SUBSTITUTION.md) |
| RC-015 | closed/confirmed | 2026-08-19 | Cold Metropolis/heat-bath difference traced to tie handling in scope | [`prereg`](../research/PREREG_RC015.md), [`record`](../research/RC015_TIE_HANDLING.md) |
| RC-016 | closed/sign constant | 2026-08-19 | Narrow equal-sweep ordering; no equal-cost or universal selector claim | [`prereg`](../research/PREREG_RC016.md), [`record`](../research/RC016_CYCLE_RECORD.md) |
| RC-017 | closed/instrument abort | 2026-08-19 | No scientific result; seeds burned as recorded | [`prereg`](../research/PREREG_RC017.md), [`abort`](../research/RC017_ABORT_RECORD.md) |
| RC-018 | closed/instrument invalid | 2026-08-19 | No scientific datum | [`prereg`](../research/PREREG_RC018_COST_IDENTIFICATION.md), [`abort`](../research/RC018_PILOT_ABORT_RECORD.md) |
| RC-019 | closed/read-only audit | 2026-08-20 | Instrument-conformance findings through RC-018 | [`audit`](../research/RC019_INSTRUMENT_CONFORMANCE_AUDIT.md) |
| RC-020 | closed/no verdict | 2026-08-22 | Two attempts, no surviving scientific measurement; named seeds burned/reserved per record | [`prereg`](../research/PREREG_RC020_MARGINAL_WALL_COST.md), [`abort`](../research/RC020_PILOT_ABORT_RECORD.md) |
| RC-021 | closed/instrument invalid | 2026-08-23 | Eleven controls passed; C10 failed; complete Class I closure, no qualification sessions and no scientific verdict | [`prereg`](../research/PREREG_RC021_HOST_INSTRUMENT.md), [`A1`](../research/PREREG_RC021_AMENDMENT_1.md), [`A2`](../research/PREREG_RC021_AMENDMENT_2.md), [`record`](../research/RC021_INSTRUMENT_INVALID_RECORD.md) |
| RC-022 | closed/architecture census | 2026-08-30 | Twenty human-designed families occupy at least eight RC-004 cells; our corpus occupies one | [`record`](../research/RC022_ARCHITECTURE_CENSUS.md) |
| RC-023 | closed/outcome (a) | 2026-08-30 | Hard prohibition and soft decaying memory were indistinguishable at the registered budget | [`prereg`](../research/PREREG_RC023_MEMORY_FORM.md), [`result`](../results/rc023/RESULT.md) |
| RC-024 | closed/works but immaterial | 2026-08-30 | Path relinking never lost a paired trial and never gained 1 %; with RC-003 and RC-023 it says the mechanism inventory is not where this corpus is short | [`prereg`](../research/PREREG_RC024_PATH_RELINKING.md), [`result`](../results/rc024/RESULT.md) |
| RC-025 | closed/no evidence | 2026-08-30 | A weight-diversity ladder exonerates the corpus: breaking G-Set's degeneracy does not make either mechanism pay, closing the architecture line after four cycles | [`prereg`](../research/PREREG_RC025_CORPUS_OR_MECHANISM.md), [`amendment`](../research/PREREG_RC025_AMENDMENT_1.md), [`result`](../results/rc025/RESULT.md) |
| RC-026 | closed/helps immaterially | 2026-08-30 | Embedding path relinking k times inside one search gives the line's only significant trend, p=1.0e-4, at gain ∝ k^0.18 against a quadratic per-round cost; the architecture line is closed | [`prereg`](../research/PREREG_RC026_EMBEDDING_LADDER.md), [`result`](../results/rc026/RESULT.md) |

## RC-021 implementation ancestry

| Commit | Step |
|---|---|
| `b090923` | host/instrument pre-registration |
| `c895e81` | Amendment 1 |
| `7685382` | Amendment 2 |
| `d884883` | journal and row codec |
| `2419c2c` | grammar and invariant corrections |
| `a994f3d` | manifest, clock, and provenance |
| `e202c58` | seeds, sentinel, and `/proc` diagnostics |
| `d4eded4` | protocol and gap enforcement |
| `1b855b7` | controls, final reviewed Step 6 |
| `3493ed8` | decision, finalize and verify, reviewed Step 7 |
| `35fb5fa` | public CLI and end-to-end synthetic tests, Step 8 |
| `ad24013` | bind session journals to run identity |
| `e9a0450` | final presence-doctrine fix and exit-5 gap record before execution |

## Update rule

Add one row for a completed milestone, accepted/rejected decision, cycle state
transition, or authority change. Do not log routine edits or duplicate the
scientific content of the linked record.

## Breakthrough investigation

| Date | Event | Source |
|---|---|---|
| 2026-09-12 | Resume saved algorithm audit; preregister conditional elimination and pair-curvature factorial before code/data | [EXP001](../research/breakthrough/EXP001_PROTOCOL.md) |
| 2026-09-17 | CD001: cross-domain mechanism map; exact rank-one response-count counterexample, scientific NO-GO; independent math/code review PASS | [H11](../research/breakthrough/h11/RESULTS.md) |
| 2026-09-19 | CD002: discrete gauge synchronization vs spectral sync; K*|Sd| frustration degeneracy, scientific NO-GO | [H12](../research/breakthrough/cd002/RESULTS.md) |
| 2026-09-19 | CD003: precision-rank bounded response; N_resp <= (2bK+1)^r confirmed, resolves H11 negative result | [H13](../research/breakthrough/cd003/RESULTS.md) |
| 2026-09-19 | CD004: native soft-conflict learning & backjumping; 33-54% node reduction on frustrated spin glasses, GO | [H14](../research/breakthrough/cd004/RESULTS.md) |
| 2026-09-19 | CD005: edge-restricted 2-opt escape; Theorem 1 (0 violations), 93-97% trap escape rate, up to 24.8x speedup, GO | [H15](../research/breakthrough/cd005/RESULTS.md) |
| 2026-09-23 | LAYA-001 completed once on frozen source; 24 synthetic groups, exhaustive reconciliation checks PASS, all variables fixed by existing presolve; no search/speed claim | [Handoff](../research/laya_semantic/HYPODIVE_BUILDER_HANDOFF.md) |
| 2026-09-25 | CD004-R correction: historical backjumping misses an optimum; certified-core recheck exact 120/120 but no node/time win; CD004 integration blocked | [Result](../research/breakthrough/cd004_recheck/RESULT.md) |
| 2026-09-25 | CD005-Q2 equal-time MIS qualification: 18/18 ties; post-result proof that 1-opt makes strict 2-opt inert for this encoding; application NO-GO | [Result](../research/cd005_equal_time/RESULT.md) |
| 2026-09-25 | EXP-007W vertex-weighted MIS qualification preregistered with six frozen synthetic graphs and equal-time CD005 comparison | [Protocol](../research/experiments/exp007_weighted_mis/protocol.md) |
| 2026-09-25 | EXP-007W completed: improving pairs in 58/60 weighted-MIS starts, but 60/60 equal-time best-result ties; no production gain established | [Result](../research/experiments/exp007_weighted_mis/RESULT.md) |
| 2026-09-25 | CD003-MR1 closed: 10/10 Market Split residuals nonempty, but 22,528/22,528 boundary fields distinct within their instances; fixed-split field-cache NO-GO | [Result](../research/experiments/cd003_market_residual/RESULT.md) |
| 2026-09-27 | Optional pinned Graft CLI installed; 12 compatibility checks pass, independent re-review PASS after initial service limit; no productivity claim | [Qualification](../research/HYPODIVE_TRIAGE.md#graft-local-integration--2026-09-27) |
| 2026-09-27 | Prepare current project for default-branch publication; remove fabricated GSET/Gurobi claims, fix Max-Cut endpoint coefficients and tuner energy offset; workspace gates pass | [Correction](../research/HYPODIVE_TRIAGE.md#github-publication-correction--2026-09-27), [Checks](../research/publication_checks/20260927.json) |
| 2026-09-27 | Protocol `f67cbdf`, instrument `2b61068`: MQ-QUAL-001 MQLib adapter qualification PASS: 544 exact state checks, 36 valid candidates, independent audit; no competitive claim | [Qualification](../research/mqlib_qualification/README.md) |
| 2026-09-27 | MQ-SCREEN-001: 240 valid matched-budget cells, all pairwise contrasts 0/80/0; H1 INCONCLUSIVE, 459 witnesses independently audited; no quality advantage established | [Result](../research/experiments/mqlib_screen/RESULT.md) |
| 2026-09-27 | MQ-CAL-001:480valid echo measurements,236messages; overall FAIL, only100ms qualifies both shapes; independent audit PASS, no native speed claim | [Result](../research/experiments/mqlib_timing_calibration/RESULT.md) |
| 2026-09-27 | MQ-NATIVE-001: protocol `fb568c8`, instrument `a25c595`, raw `a435e7a`;960valid cells, overallFAIL;50/100ms pass native diagnostic model paths, independent audit PASS; no solver speed claim | [Result](../research/experiments/mqlib_native_calibration/RESULT.md) |
| 2026-09-28 | MQ-MST2-001 PASS:544 exact states,144valid cells,1,606callbacks,108finals,36expected kills; independent audit PASS; standalone bridge only, no performance claim | [Result](../research/experiments/mqlib_mst2_qualification/RESULT.md) |
| 2026-09-28 | MQ-DIFFICULTY-001 preregisters24 hashed qualification-only inputs, four fixed wrappers and delivered-validated cost rules; generator checked, no optimizer run or difficulty verdict | [Protocol](../research/experiments/mqlib_difficulty_qualification/protocol.md) |
| 2026-09-28 | MQ-DIFFICULTY-001 closed:512VALID preflight cells,13/16control groups PASS, overallFAIL; independent artifact audit/review PASS; main not admitted, hypotheses untested | [Closure](../research/experiments/mqlib_difficulty_qualification/closure/RESULT.md) |
| 2026-09-28 | MQ-QUALITY-002 preregisters admission revision for unchanged2s qualification; all24inputs and foursettings retained, no observations | [Protocol](../research/experiments/mqlib_coarse_quality/protocol.md) |
| 2026-09-28 | MQ-QUALITY-002 completed one admission and one main; 960valid main cells, independent audit/review PASS, H1 21/24 and H2 zero material unique wins | [Result](../research/experiments/mqlib_coarse_quality/closure/RESULT.md) |
| 2026-09-28 | MQ-FIRST-CHUNK-001 prospectively registers first-witness diagnostic after MQ-QUALITY-002 exposed dense512 fallback | [Protocol](../research/experiments/mqlib_first_chunk/protocol.md) |
| 2026-09-28 | MQ-FIRST-CHUNK-001 completed12valid cells; Ultimate q18 first verified output2.962–3.061s; independent audit/review PASS, delivery-only conclusion | [Result](../research/experiments/mqlib_first_chunk/RESULT.md) |
| 2026-09-29 | Public-only discovery triage independently verifies published scope-112 DTS witness; no new record, product demand or 4–5x selection claim; Ising Engine stays paused | [Triage](../research/HYPODIVE_TRIAGE.md#public-evidence-discovery-after-repro-bench--2026-09-29) |
