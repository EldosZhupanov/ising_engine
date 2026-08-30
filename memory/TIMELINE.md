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

## Research-cycle chronology

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
