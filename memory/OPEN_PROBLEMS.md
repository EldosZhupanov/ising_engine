# memory/OPEN_PROBLEMS.md — pointer + findings that live nowhere else

**Authoritative sources**
- `../research/RESEARCH_GAPS.md` — research gaps
- `../ROADMAP.md` §"Technical debt / unfinished" — engineering debt

The two sections below are **not** in either source and must not be lost: they
invalidate entire classes of future experiment on the default benchmark.

---

## 0. Open decisions awaiting the maintainer (all trajectory-changing)

Four findings are verified and recorded but **deliberately not applied**, because
each alters plan ranking or run trajectories and is therefore behaviour-changing
under ADR-0004 (needs approval + A/B at identical seeds). Full detail:
`../research/RESEARCH_INVENTORY.md`.

| # | Finding | Proposed change | Cost of leaving it |
|---|---|---|---|
| 1 | **RC-005** — `cost_model` is shape-only; true cost carries flip density φ with a regime change at φ=1/3 | adopt `W = W_scan + W_flip·min(φ,⅓)` | Decision Engine **overcharges cold regimes up to 2.1×**, penalising low-temperature refinement |
| 2 | **RC-007** — 28/28 operator pairs commute in distribution (ρ < 1.4) | narrow the Evolution Engine's ordering search | compute spent on distinctions that do not exist at pair level |
| 3 | **RC-011** — the predictor's LOO Spearman is provably invariant to instance features | add instance×schedule interaction terms, **or** score across folds | no metric currently able to detect instance-conditional transfer |
| 4 | **RC-002** — all 18,570 runs start all-zeros; diverse init is worth 0.09–0.46% | one `random_flip_sweep` before the thermal phase | measured quality left on the table |
| 5 | **RC-012** — `--early-stop` cannot activate (6 rows < fit's 20-row floor), and even a fitted model's ε-gate is vacuous (predicts ≡0) | **HALF DONE 2026-08-19: the flag now refuses loudly and exits 2** (trajectory-neutral — it never activated). Still open: fix bootstrap AND training-target pathology **together**; fixing only the bootstrap ships a silent 50% truncation | ~~a shipped flag that does nothing~~ **resolved**; the *capability* remains absent |

**Why none was applied:** changing any of them breaks bit-identical replay of the
recorded corpus, or changes which plans the platform selects. That is the
maintainer's call, not an autonomous one.

Also open, non-trajectory: **`src/core/simd_utils.rs`** — orphaned, declared in no
`mod.rs`, never compiled, self-described as "a theoretical implementation".
Delete it or implement it; leaving it reads as a performance claim the code does
not support. **RC-013 evidence leans DELETE:** measured floors show the sparse
kernel is bookkeeping/RNG-bound in cache and gather-bound at DRAM scale —
neither is the f64 SAXPY the file targets. (Scope: engine_v2 backend measured;
ultimate.rs's MSC kernel was not.)

## 0b. RC-021 instrument findings — reviewed, classified, some deliberately open

Recorded 2026-08-28 from an independent read-only review plus an automated
review pass over the uncommitted Step-8 diff. **Every item below was verified
against the code, not accepted from a report.** The three marked FIXED are
mutation-verified; the rest are open by decision, with the reason, so that no
future session re-derives them or "fixes" them without deciding the same
question.

### Fixed, with a killed mutation each

| # | Defect | Class it violated | Fix |
|---|---|---|---|
| A | `--session N` returned `HOST-NOT-QUALIFIED` (exit 1) when a predecessor session was absent, damaged, or aborted — a §7.2 **result** published by a mode that measured nothing, with no closure record (§C14.1 makes the two inseparable) | Class II leaking out of a precondition refusal | `PredecessorFault` splits "not completed" (exit 2, operational) from "journal damaged" (exit 4, §C13.7). `session_preflight`'s own `PredecessorNotClosed → HostNotQualified` mapping was the same defect one layer down and is now exit 2, matching its own doc comment |
| B | A host that changed **mid-session** (`boot_id` moved, or uptime fell below the run's start) was mapped to `JournalWriteFailed` → `JOURNAL-INVALID`, exit 4, **writing no `run_invalid.json`** — a sound journal declared broken, and the record of *why* the run stopped never written | §C12.3 | New `SessionOutcome::HostChangedMidSession` → `INSTRUMENT-INVALID`, exit 3, and it is named in `writes_run_invalid()` so the module, not the caller, owns which outcomes write a record |
| D | **What exit 4 means — decided 2026-08-28.** `execute_session` mapped every mid-session failure to `JournalWriteFailed` → `JOURNAL-INVALID`: the sentinel refusing, a work block failing, `/proc/loadavg` unreadable. Exit 4 had become "something went wrong", which destroys the only signal that says *the record is unusable* | `JOURNAL-INVALID` is now reserved strictly for journal create/append/flush/fsync. Apparatus failures get `SessionOutcome::MeasurementFailed` → `INSTRUMENT-INVALID`, exit 3, which is what §6 and §C11.25 class a failing instrument; the journal stays intact and readable, the session ends without a close row, and `--finalize` derives the run's class from the journals as it does for any abort. **The same sweep found the §C12.3 clock fact is read at three points — before `SESSION-OPEN`, per measurement, and before `SESSION-CLOSE` — and the earlier fix had corrected only the middle one.** All three now report `HostChangedMidSession` |
| C | The durable P6 provenance row recorded `build_flags=<none>` on every run. `option_env!` reads the compile-time **environment**; this workspace sets `-Ctarget-cpu=native`, `+avx2,+fma`, `-Copt-level=3` in `.cargo/config.toml`, which cargo passes as rustc arguments. The instrument was asserting "no flags were used" about the one input most able to move the timings it exists to certify | honesty of a durable record | An unobservable value is now `UNAVAILABLE`, never absence, and the *effective* `target_feature` set — which `cfg!` can actually see — is recorded beside it. `build_profile` states `debug_assertions=on/off` instead of naming a cargo profile it cannot determine |

### Round three — build provenance was false in three different ways

| # | Defect | Fix |
|---|---|---|
| K | **The release guard was dead code, and had been since Step 6.** It was `cfg!(debug_assertions)`. **Measured on this workspace: that macro is `false` under `cargo test`, under `cargo test --release`, and in every build** — `.cargo/config.toml`'s rustflags are in play. So the guard could never fire, in either measuring mode, and RC-021 had **no protection at all** against producing its ninety scientific timings from an unoptimised binary. An earlier fix of mine extended this guard to `--session`; it extended a guard that never fires | `build.rs` now forwards `PROFILE`, `OPT_LEVEL`, `DEBUG` and `CARGO_ENCODED_RUSTFLAGS` — which cargo does give a build script — and `is_optimised(profile_opt_level, flags)` decides from those two facts, with rustflags overriding the profile (this workspace's own case: a `dev` profile compiled at level 3). The predicate is separated from its environment lookup **because on this workspace the broken proxy and the correct check agree**; they differ only on the unoptimised build this workspace never produces, so `is_optimised_build()` cannot be falsified here but `is_optimised(..)` can, exhaustively |
| L | `build_flags` recorded `<none>`, then `UNAVAILABLE`, for a binary built with `-Ctarget-cpu=native`. Both were wrong: the first was false, the second gave up on an observable fact | `env!("RC021_CARGO_ENCODED_RUSTFLAGS")` from the build script now carries the real flags, beside the effective `target_feature` set |
| M | A missing `rustc` refused the **entire qualification** at exit 2: `provenance_detail` propagated the error, P6 failed, `run_controls` refused. The recommended measuring host is a quiet machine that need not carry a toolchain, and a version *string* is not a qualification criterion | `rustc_version_fact` records `UNAVAILABLE(<why>)`, matching the rule already applied to build flags |
| N | The `OK`/`LOST` acceptance floor was `RESOLUTION_MULT × timer_resolution_ms` from the **per-invocation** observation. §C6.1 freezes per-invocation sampling for `diag_availability` only — it says nothing about timer resolution — and nothing cross-checked the header against the manifest. A clocksource degrading before session 4 would send its fifteen rows to `LOST` against a floor the other five never faced: **the acceptance criterion moving mid-run, with nothing to notice** | `execute_session` takes the manifest and derives the floor from it. The observed resolution is still what each row and header record — that is provenance, not a criterion |
| O | `--finalize` printed only `variant_name()`, which is `"Wrote"` for every terminal status and `"DurabilityFailed"` for every durability point — discarding the run's verdict and the failing step | `FinalizeOutcome::describe()`, matching `SessionOutcome::describe()` |
| P | The P6 provenance row re-read G11 from disk to hash it, certifying bytes the sentinel may never have executed if the file changed after `load_sentinel_instance` | It records `protocol::G11_SHA256_PREFIX`, the frozen value the executed instance was verified against |

**The recurring shape, stated once.** Every defect in this round was *an
instrument asserting something it had not observed*: flags it could not see, a
profile it could not determine, a compiler it could not run, a floor it had not
frozen, a hash it re-read after the fact. The rule that would have caught all
six: **before a durable record states a fact, ask what would happen if the fact
were unobservable — and make that case say so.**

### Round four — six more, and one that made the suite red

| # | Defect | Fix |
|---|---|---|
| Q | **`cargo test` was red.** A test written in round three asserted the optimisation evidence appears in `build_profile_fact()`, but under the dev profile that reads `profile=debug opt_level=0`; the `-Copt-level=3` that actually optimises this workspace lives in `build_flags_fact()`. It passed under `--release` and failed under `cargo test`. **Cause: only `cargo test --release` had ever been run** | The assertion reads both facts. **Both profiles are now gated** — `cargo test`, `cargo test --release`, `cargo clippy` and `cargo clippy --release` |
| R | `build.rs` — added in round three, and now the sole source of `RC021_OPT_LEVEL`/`RC021_CARGO_ENCODED_RUSTFLAGS`, which decide the gate admitting both measuring modes — sat **outside `INSTRUMENT_FILES`** and was untracked. An edit making it emit `RC021_OPT_LEVEL=3` would leave the tree "clean", pass P6, and have the closure certify a build that never happened | `INSTRUMENT_FILES` is 10 and includes `build.rs`. Appended rather than prepended: the provenance fixtures index the array, and inserting at the front moved the birth commit |
| S | `is_optimised` matched only the joined `-Copt-level=`. `CARGO_ENCODED_RUSTFLAGS` separates *arguments*, so `["-C", "opt-level=0"]` arrives as `-C opt-level=0` — unmatched, falling through to the profile, reporting a release build explicitly compiled at level 0 as **optimised**: the exact false negative the guard exists for | `last_opt_level` reads both spellings, last occurrence winning as it does for rustc |
| T | `--controls` discarded every failure diagnostic. `RefusedBeforeMeasurement`'s contract is "not one byte created", so an unprinted reason exists nowhere at all | `ControlsOutcome::describe()`, matching the two already added for sessions and finalize |
| U | `finalize` read the clock **twice** — once for the preview closure that renders `RC021_RESULTS.md`, once for the real closure — so the published Markdown and the canonical closure could disagree about when the run was finalized, and `verify` could not catch it because it hashes the Markdown without re-reading its content | One `FinalizeStamp`, taken once per finalization and passed to both |
| V | `any_artifact_present` used `.exists()`, which maps EACCES to `false`, contradicting the module's own `path_present` doctrine — "an unreadable path is not an absent one". §C14.16's guard would be skipped, the closure path reserved (locking the run irreversibly), and only then the artifact write failed | It uses `path_present` |

**Two mutations survived and were closed by testing the reachable half.** The
wiring of `--controls`' diagnostic cannot be exercised: the refusal fires in
`report_status` long before the arm that prints `describe()`, and reaching that
arm needs `provenance::check` to pass, which it cannot on a working tree. The
pure part — that `describe()` never drops a detail — is covered, as is the
presence doctrine, via a directory made untraversable so a path errors instead
of being absent.

### Round five — no HIGH; three MEDIUM, and one finding rejected on the norm

| # | Defect | Fix |
|---|---|---|
| W | A record labelled `sentinel_sha256` held `G11_SHA256_PREFIX` — **twelve** hex characters, which is what the gate compares, not a digest. It is fsynced into P6's detail column and republished in `RC021_RESULTS.md`, so an auditor running `sha256sum` finds the record disagrees with the file | `VerifiedSentinelInstance` now carries the full digest of the bytes it was built from, captured at verification. No re-read: re-reading certifies whatever is on disk *now*, not what the sentinel ran |
| X | `--verify` discarded every diagnostic. `Mismatch { what }` names which of the fifteen inventory entries failed — the only useful thing `--verify` produces — and exit 4 arrived with empty stderr. The fourth mode; the other three were hardened in the same change | `VerifyOutcome::describe()` |
| Y | `--controls` read the wall clock twice: once inside the metadata builder for the control journal, once for the N3 and P2 headers. A run straddling a second stamped them differently, and nothing in `metadata_binds` or `ControlsContext::bind` compares `utc_start`, so it failed silently | One `invocation_utc()` per mode, passed to both. It is now the only direct wall-clock read in the file, asserted by test |
| Z | `LiveEnvironment::new` fails on exactly one thing — an unparsable `cpu_set` — with nothing durable written, yet `--controls` called it exit 2 and `--session` exit 3 | Both correctable, as the surrounding comment already claimed |

**Rejected, with the citation — do not "fix" this later.** The review called it
a MEDIUM that `--controls` reports a changed `boot_id` as exit 2 with no record
while `--session` reports the same fact as exit 3 with `run_invalid.json`.
**§C12.3's table freezes exactly that distinction**
(`PREREG_RC021_AMENDMENT_2.md:571-573`): a mismatch detected *before*
`controls_started` is `REFUSED-BEFORE-MEASUREMENT`, exit 2, nothing written;
*after* it, exit 3 with the record. `--controls` checks before the marker
exists, `--session` after. The modes differ because the norm makes them differ,
and writing the record in `controls_mode` would violate the frozen table.

**Left open by decision.** The optimisation guard still does not require the
release *profile*. `.cargo/config.toml` gives every profile `-Copt-level=3`, so
a dev build passes the optimisation test while differing in codegen-units (256
against 16) and incremental compilation, both of which move timings. Adding the
conjunct was tried and reverted: the guard sits at the entry of both measuring
modes, so under `cargo test` (`PROFILE=debug`) it refuses every one of them and
roughly ten tests can no longer reach the branch they exist to check. The fact
is not lost — `build_profile_fact()` records `profile=` in the durable P6 row,
so a run made from a dev binary says so in its own record. Closing it properly
needs a build-fitness seam on both modes, the way `SessionHost` was added for
the live world.

### The commit itself was a test — and it found one

Committing Step 8 made the working tree clean for the first time, which changed
what the suite exercises: `provenance::check` refuses any uncommitted tree, so
while the change was in flight **every branch behind it was unreachable**, and
`cargo test` on a clean tree runs code no earlier run had touched.

One test failed on that first clean run.
`only_host_facts_write_the_record_and_the_modes_agree_on_the_rest` called the
real `session_mode`; while the tree was dirty the provenance gate refused first
and the assertion passed without reaching any decision. On a clean tree the same
call ran on to `check_configuration`, where the synthetic fixture manifest
legitimately mismatches the real host and writes `run_invalid.json` — so the
assertion "a non-host failure must not poison the run" fired on a *host* fact.

**A test whose outcome flips on `git commit` is testing the tree, not the code.**
That half was removed; the CLI behaviour is covered through the `SessionHost`
seam by `an_unreadable_sentinel_refuses_without_poisoning_the_run`, which does
not depend on tree state.

**Standing consequence.** Any test calling `session_mode`/`controls_mode`/
`init_run_mode` directly is tree-state-dependent by construction. New tests for
those paths go through the seam. Both regimes are now verified: 682/0 dirty and
682/0 clean, in both profiles.

### Round six — the HIGH that would have wasted the single permitted run

| # | Defect | Fix |
|---|---|---|
| AA | **Session journals were never bound to the run identity** — the six files the verdict is derived from, and the only durable inputs not bound. `read_ledger` took no identity; `session_grammar` checked only that the header named the right session number. `Row::validate` cross-checks each row against *its own file's* header, so a journal from another run or another host parses cleanly, classifies `COMPLETED` with fifteen valid measurements, and counts toward the ninety. `--finalize` could publish `HOST-QUALIFIED` from it, and `--verify` re-derives through the same path, so it would confirm rather than catch | `classify_session` takes `&RunIdentity` and rejects a journal whose §C10.1 values are not this run's, routing to `JOURNAL-INVALID` through the existing `any_journal_invalid` branch — the same rule the control journal and both markers already used |
| AB | `parsed_row_count` for `CONTROL_TSV`/`OBSERVATIONS_TSV` was returned as `data_row_count`, so the two were equal by construction. A control journal whose last write was torn recorded `parsed_row_count: 12` beside `status_counts: {PASS: 0, FAIL: 0}` — self-contradictory, in the artifact that exists to be audited | An unterminated final line is data but was parsed by nobody |
| AC | `artifacts_consistent` used `.exists()`, the third site to break the module's own presence doctrine | `path_present` |
| AD | The crate-level `allow(dead_code)`, stale since Step 8 wired the module to the CLI, hid a live defect: `IdentityOutcome::Unreconstructible(String)` builds a real diagnostic naming which header disagreed, and every consumer discarded it with `_` | Removed. `finalize` carries the reason in `describe()`, `verify` in `Mismatch` |

**Why this one matters beyond its own fix.** It was found in the sixth round,
hours before the first real execution, and §2 permits **no retries and no
"repeat in a quieter moment"**. A run started that morning would have spent the
single permitted attempt on an instrument that could not tell its own evidence
from another run's — and nothing in the run would have revealed it, because
`--verify` shares the defective path.

**The lesson, and it is the same one six times over.** Every defect this cycle
was an instrument trusting something it had not checked: a caller's ordering, a
proxy for the build, a snapshot of the host, a label on a digest, a line it
never parsed, a journal it never bound. The discipline that caught them is not
review volume — it is asking, of each durable statement, *what would make this
false, and would we see it?*

### Round seven — no HIGH, and one finding that lands on the preregistration

**Fixed.** `path_present` used `std::fs::metadata`, which **follows** symlinks,
so a dangling symlink returned NotFound and the function reported "absent" —
precisely the reading its own comment forbids, in the **fourth** site where this
doctrine broke and the subtlest, because the offender was the doctrine's own
implementation. §C14.16's guard depends on it, so a dangling `RC021_RESULTS.md`
made finalize reserve the closure path instead of refusing before reserving
anything. `main.rs::path_present` and `controls::path_state` already used
`symlink_metadata`. Also removed a branch in `last_opt_level` that could never
execute, in a module that exists partly to delete guards that cannot fire.

**Against the preregistration, not the code — for the maintainer.**

`INCONCLUSIVE-UNDERPOWERED` (exit 5) is **unreachable**. §8.1 clause 3 makes it
reachable *only* through an `EXTERNAL-CAUSE` row "written and fsynced to the
session journal before the abort it justifies", and §C13's state machine freezes
**exactly five public modes**, none of which records one. `RowContext::
external_cause` therefore has no production caller. This is not an implementation
defect: adding a sixth mode would violate the frozen table, so the instrument
cannot close it. §8 defines six run statuses and one of them cannot occur.

The consequence is operational and should be understood before any run. A
transient instrument fault mid-session — one EIO on `/proc/loadavg` — ends the
session without a close row. `--session` reports INSTRUMENT-INVALID, exit 3, but
at finalize §8.2's chain finds no control failure, no journal-integrity failure
and no `EXTERNAL-CAUSE` row, so it falls to §7.2, the unwritten measurements
count as failures, and the run is published as **HOST-NOT-QUALIFIED** — a Class
II verdict about the host, from a fault in the instrument.

That is §7.2 working exactly as written ("every planned execution stays in the
denominator whatever it returns", and no repeat in a quieter moment), so the
instrument must not paper over it. But it means **a single transient fault costs
the single permitted run and publishes a negative result about a healthy host**.
Closing it needs an amendment under §15 — either a sixth mode that records an
external cause, or a rule sending an in-session apparatus failure to Class I.

**Also low, recorded not fixed.** When the results-Markdown write fails at
`sync_all` with the bytes already on disk, finalize records Class I while
`--verify` recomputes Class II and returns `Mismatch("terminal_status")`. Both
exit 4, so the operator is not misled about whether the run is broken; only the
diagnostic is imprecise. §C14.15 makes the asymmetry deliberate — a later reader
classifies from surviving bytes, never from a past syscall error.

### Open by decision — do not "fix while here"

| # | Finding | Why it is open |
|---|---|---|
| E | **Fixed 2026-08-28 as part of J.** `run_invalid.json` for a mid-session host change recorded the *pre-session* snapshot — which by construction had already passed the preflight — so the record said `observed == expected` ("nothing changed") beside a reason saying the host changed, and stamped a `monotonic_offset_ms` recomputed from a stale uptime where §C12.3 requires `null`. `write_run_invalid` now re-observes `boot_id`, fingerprint and uptime at the point of failure, and says so in the reason if the host cannot be read at all | — |
| I2 | Two state-machine rows now have CLI tests — the terminal lock for `--controls` and `--session`, both codes, both writing nothing — and the other two are item J. Item I is closed | — |
| E | `LiveClock` is anchored to a `/proc/uptime` sample read before `provenance::check` runs its `git` subprocesses, so `controls_complete.monotonic_offset_ms` understates elapsed uptime by roughly that duration. Session rows re-read `/proc` per row, so the two sit on slightly different axes | Sub-second against a 600 000 ms floor. Precision, not a wrong verdict. Fixing it moves a durable offset, so it is behaviour-changing under ADR-0004 |
| F | The frozen session-journal filename has two spellings: a literal `format!("rc021_journal_s{session}.tsv")` in `execute_session`, and `decision::session_journal_name` everywhere else | One convention, two owners. Currently consistent; a change to the helper would silently make the executor write a file nobody reads |
| G | `--verify` on a directory with no closure path returns exit 2. Amendment 2's §C14.2 table gives cells for *valid complete*, *empty or partial* and *damaged manifest*, but **none for absent** | Exit 2 is a defensible inference ("the precondition is unmet, nothing was done"), but it is an inference. Recorded so the next reviewer does not re-open it as a bug, and so that an amendment can settle it if the question ever matters |
| H | `jnum`'s fallback returns `v.to_string()` when no candidate round-trips, which for `NaN`/`±inf` emits invalid JSON | Unreachable through validated spreads. The failure is loud rather than silent (the closure fails its own reader and becomes `DerivedClosureInvalid`), but the diagnostic points at the wrong thing |
| I | Three rows of Amendment 2's state-machine table have no test: `--controls` with a complete or empty closure path; `--session` short-gap refusal at CLI level; `--init-run` with a reserved path already present | Coverage debt, not a defect. Listed so it is closed deliberately rather than discovered again |
| J | ~~Three CLI branches untestable behind the provenance gate~~ **CLOSED 2026-08-28.** The cause was not the branches but the absence of a seam: `live_facts` runs `provenance::check`, which refuses any uncommitted tree, so everything behind it returned the gate's own code. Three tests written against those branches passed for the wrong reason and were removed; a fourth mutation survived a test that looked like it covered it. A `SessionHost` trait (`main.rs`) now injects `repo_root`/`facts`/`observations`/`sentinel`/`exe`, matching `FinalizeIo`, `SessionEnvironment`, `ControlRunner` and `Clock`. Four branches are now genuinely covered: §C11.45's short gap, §C12.3's clock fact through the CLI, an unreadable sentinel, and a missing repository. **Still open: `--init-run`'s reserved-path refusal**, which has no seam of its own — the rule is covered in `manifest::tests` | The lesson, recorded because it recurred four times in one day: a test that passes for the wrong reason is worse than no test. Every new branch test here must be shown to fail under a mutation of the branch it names, not merely to pass |

---

## 0c. Soup/LoRA pilot — an infrastructure result whose control arm is unconstructible

Recorded 2026-08-28, corrected same day. Kept here because the *reason* it
cannot be fixed cheaply is the reusable lesson.

**What the pilot established.** Qwen2.5-Coder-7B was QLoRA-fine-tuned end to end
on a 6 GB laptop GPU: NVMe layer streaming, ~2.07 GB peak VRAM, 2 523 136 of
7 615 616 512 parameters trainable (0.03%), one epoch in ~25 min, weights pinned
by revision and verified by SHA-256, with a deterministic reproducibility
receipt. That is real and it is Soup's achievement, not the engine's.

**What it did not establish.** On the 64 held-out items the adapter scored 3/64
against the base's 1/64 — McNemar exact **p = 0.625**, and `both correct = 0`.
It answered `E=-1` on **64 of 64** items: it learned the output format, not the
function, and scores *below* the best constant predictor (`E=7`, 7/64 = 10.9%).

**Why the obvious control cannot be run.** The Ising selector chose 64 of 128
inputs for training and the held-out set is the *complement*. Any other training
set of 64 therefore overlaps the held-out set, so a "random-64" arm would be
scored partly on its own training data; giving it its own complement instead
makes the two arms' evaluation sets different and their accuracies
incomparable. **The split makes the pilot's own control arm unconstructible.**
A redesign is required: a shared held-out set neither arm trains on, both arms
choosing from the remainder, equal budget and seed policy, the selector's own
compute charged against it, and several random draws rather than one.

**The lesson.** The control arm has to be designed into the split, before any
data is generated. Adding it afterwards was not merely skipped here — it was
made impossible.

---

## 0d. RC-023 — the form of memory does not matter, and the pretty pattern is not a finding

`results/rc023/RESULT.md`. Preregistered before code (`74e425e`), operator
`34eab7c`. Two-sided Wilcoxon, 30 paired G-Set instances: p = 0.87 registered,
p = 0.63 with the print-precision artifact removed. Hard prohibition with an
aspiration criterion does nothing a soft decaying bias cannot, at this budget on
this corpus.

Three things this cost us that are worth keeping:

**The harness printed away its own resolution.** `--op-benchmark` prints
`{norm:.3}`, which manufactured six exact ties out of thirty — and Wilcoxon
*discards* ties. The registered analysis was therefore run on 24 pairs without
anyone having decided that. Re-printing at 9 dp (same deterministic runs, only
the format changed) recovered four of them. The conclusion did not move, so
nothing was rescued; but a display precision silently became an analysis
decision. **Check the resolution of the number before ranking it.**

**The twin only isolates one variable if the RNG agrees.** `tabu_sweep` mirrors
`history_field` including the `let _u: f64 = rng.gen();` on the accept-by-descent
branch. Three of the four correctness tests died under their mutation
immediately; the fourth mutation — deleting that draw — **survived**, because
nothing tested it. Without a test, the comparison would quietly have been
"different memory *and* different randomness". `the_rng_stream_stays_aligned_with_the_twin`
now pins it. **In a twin experiment, the shared parts need tests more than the
differing part does — the differing part is what you are watching.**

**The perfect separation is the shape of our last retraction.** The sign of the
difference splits 6/6 vs 3/24 on whether the instance has negative weights,
Mann–Whitney U = 0, p = 0.0001. Density is genuinely excluded — G48/G49/G50
match G11/G12/G13 on topology *and* m/n and show ~0. But the matched control is
**saturated**: nine of nineteen operators sit exactly at its optimum, so a null
difference there is a ceiling, not evidence. Law 2 was retracted for exactly
this (an empty control plus a confound). Recorded as a hypothesis needing its
own preregistration, whose first obstacle is a control corpus that is not
already solved.

---

## 1. Benchmark degeneracy — G-Set cannot test two questions *in principle*

Not a statistical-power problem. An **identifiability** problem: no quantity of
additional G-Set runs fixes either one.

**(a) The guide axis is unidentifiable.** Every G-Set instance is unweighted
(J ∈ {−1,+1}); `dimacs_maxcut/sg3dl051000.mc` likewise. For J ∈ {±1}, with V
violated and S satisfied edges, E = V − S and V + S = |E|, so

    E = 2V − |E|

an affine bijection. Ranking by violation count **is** ranking by energy, so
energy-guided and constraint-guided search are one method written two ways.
All 18,570 recorded experiments ran on such instances and could not have
detected a difference between them even if one existed.

**(b) The frustration axis is confounded.** G-Set contains **zero unfrustrated
triangles** — most instances are all-positive-weight so every coupling is
antiferromagnetic and every triangle is frustrated by construction; the ±1
instances (G11–13, G32–34) are toroidal grids with no triangles at all. So
"frustrated triangle" and "any triangle" denote the same edge set, and a
frustration split has **no control group**. It also correlates with degree by
definition (69.3 vs 26.3 mean local degree), so any such comparison measures
density.

**Both need `biqmac`** (`be100.1.sparse` has 145 distinct weights, −100…100),
not G-Set.

**Method rule this produced:** before interpreting any grouped comparison, check
that the control group is non-empty and that the grouping variable is not forced
by graph structure to correlate with degree.

## 2. Empty capabilities and the orphaned SIMD file

**Three of six declared capabilities have zero implementing operators** —
`ExactInference`, `Approximate`, `Warmstart`. A capability query for them can
never return anything, silently. Note RC-002 (see [RESEARCH.md](RESEARCH.md))
gives a measured reason not to rush `Warmstart`: what an initialization
contributes is ensemble *entropy*, not solution *quality*.

**`src/core/simd_utils.rs`** exists (1,545 B) but is declared in no `mod.rs`
(`src/core/mod.rs` lists only `csr_matrix`, `hubo`, `anls`) — orphaned, never
compiled, self-described as "a theoretical implementation … a foundation for
replacing the serial iteration in `ultimate.rs`". **Decision needed: delete it
or implement it.** Leaving it reads as a performance claim the code does not
support, and it is the first thing an external reviewer would find.

## 3. From the authoritative sources (summary only — read them for detail)

- ~~`cargo doc` broken by rustdoc-link errors~~ **RESOLVED 2026-07-27** — 18 sites
  across 9 files, doc-comments only; now a `docs` job in CI with
  `RUSTDOCFLAGS: -D warnings`.
- ~~CI never checked the `research/` workspace member~~ **RESOLVED 2026-07-27** —
  root cause of the surviving E0063 break, see §4 below.
- Cloud routing recommended but no API key available
- World-filtered ideation and Dynamics early-stop are opt-in only
- `--early-stop` bootstraps one Dynamics model from `instance[0]`
- Consensus/theory facts publish at support=1 → low graph confidence
- Multi-instance `investigate` confidence stays low until run across many instances
- `experiments/` is generated state (gitignored, regenerable)

## 4. OPEN — the operator cost model omits its dominant variable (RC-005)

Every `cost_model` in the 18-operator library is a function of instance **shape**
only. But `SparseBitSlice::apply_flips` (`sparse_bitslice.rs:265`) dispatches on
**flip count**: scattered `O(deg·flips)` below φ=1/3, dense `O(deg·r)` above.
So the true cost carries a hidden parameter — **flip density φ = flips/r, which
equals the acceptance rate** — and the declared model is blind to it.

Measured (`src/bin/exp_cost_model.rs`, 4 G-Set instances, interleaved
temperatures + median so host drift cannot fake a trend): declared work constant,
actual ms varies **1.61–2.09×**. Corrected law
`W(φ) = W_scan + W_flip·min(φ, 1/3)` fits **R² 0.93–0.97**; the flip term is
**57–79% of cost at saturation**. The `min` cap is provably inert on the two
instances that never reach φ=1/3 (capped R² identical to uncapped to 4 d.p.) and
lifts R² by 5–8 points on the two that cross it — the signature of a correct
functional form, not a free parameter.

**Consequence:** the declared model is the corrected law with φ ≡ 1/3 hard-coded,
so it **overcharges by up to 2.1×, worst in the cold regime**. The Decision
Engine ranks plans by utility per unit cost, so low-temperature refinement is
systematically penalised — the same regime where the
[RC-001 evidence](../research/RC001_ENSEMBLE_THERMOSTAT.md) says the best
operator changes.

**Not fixed.** Correcting `cost_model` changes plan ranking and therefore
trajectories → behaviour-changing under ADR-0004, needs approval + A/B at
identical seeds. Full record: `../research/RC005_COST_MODEL_BLINDNESS.md`.

## 5. Resolved 2026-07-27 — the two gates that did not cover what they claimed

Both were the same failure class: a green pipeline that structurally could not
see the thing it was supposed to protect. Recording the mechanism, because the
pattern will recur.

**(a) CI excluded the `research/` workspace member.** `cargo metadata` shows
`workspace_default_members` = the **root package alone**, so `cargo build
--all-targets` (CI's command, no `--workspace`) never compiled `research/`.
That is why `research/src/bin/test_engine.rs` sat broken (E0063, missing
`energy_offset`) indefinitely — CI reported green because it never looked.
Fixed by adding `--workspace` to build/clippy/test and `--all` to fmt.

**(b) No documentation gate at all.** 18 rustdoc errors had accumulated across
9 files. Fixed, and a `docs` job added with `RUSTDOCFLAGS: -D warnings`.

**Verification standard used:** every one of the 7 `run:` commands in
`.github/workflows/ci.yml` was extracted and executed locally — all pass.
Widening CI scope is only safe once the newly-covered code is known to build,
so the debug-profile commands were run first, separately.

**Rule this produced:** a passing gate proves nothing until you check its
*scope*. For a cargo workspace, confirm `workspace_default_members` before
trusting any `cargo` command in CI. Note also that rustdoc aborts per crate, so
doc errors cascade — one clean run after a fix only proves the first failure is
gone; re-run until the count stops changing.
