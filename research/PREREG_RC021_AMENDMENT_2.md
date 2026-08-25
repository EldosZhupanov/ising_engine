# PREREG — RC-021 Amendment 2: journal, run and file contract

**Status: REVIEWED PASS; binding upon commit.**

**Review record**, as §C11.6 requires — a verifiable reference, not an assertion:

> 2026-08-25 — Codex — eighth independent read-only review — PASS.

Seven earlier independent reviews returned FAIL; each finding was fixed and
re-reviewed before this one.

**The standing rule is retained: this document may not be cited, and no
instrument may rely on it, until it is committed.** A review that has passed and
a document that binds are different things, and only the commit makes the second
true.

Amendment to `research/PREREG_RC021_HOST_INSTRUMENT.md` (`b090923`) as amended by
`research/PREREG_RC021_AMENDMENT_1.md` (`c895e81`). Documentation only: no code,
no run, no `experiments/rc021/`.

**Reason.** A contract audit found the committed pre-registration under-specified
in ways that let two faithful implementations produce incompatible bytes, and one
way that could change a verdict. Nothing about the frozen design moves: the
seeds, the `0.09` bound, `6 × 3 × 5 = 90`, the fixed denominator, the absence of
retries and the engineering-only meaning of both verdicts are preserved exactly
(§C17).

---

## §C1. Schema stays 24 columns; `block` is defined

No column is removed, added or reordered. **`block` is the phase-local pair
index, `1..5`.** `measurement_index` remains global, `1..90`. The triple
`(session, phase, block)` addresses a measurement; `measurement_index` orders
them across the run. The two are complementary, not duplicates.

The `session` **column** domain is extended to `0..6`, where **`0` denotes a
control journal** and `1..6` a qualification session. This defines a value that
was previously undefined for control journals; it changes no column.

## §C2. `run_uuid` — frozen generation

16 bytes from **`rand::rngs::OsRng`**, lower-case hex, exactly 32 characters.
`rand 0.8.5` and `getrandom 0.2.17` are already in `Cargo.lock`, so no dependency
is added. `OsRng` is used **only** for the run identifier and never enters a
measurement, a seed, or any scientific quantity.

## §C3. `host_fingerprint` — exact sources and normalisation

Six fields, `key=value` followed by a line feed, **in this order**; the
fingerprint is the SHA-256 of the exact UTF-8 bytes of that string.

| key | source | normalisation |
|---|---|---|
| `kernel_release` | `/proc/sys/kernel/osrelease` | single line, trailing newline dropped |
| `kernel_version` | `/proc/sys/kernel/version` | single line, trailing newline dropped |
| `available_processors` | count of directories matching `/sys/devices/system/cpu/cpu[0-9]*` | decimal integer |
| `mem_total_kb` | `/proc/meminfo`, line `MemTotal:` | integer kB only, unit stripped |
| `cpus_allowed_list` | `/proc/self/status`, `Cpus_allowed_list:` | value after the separator, surrounding spaces and tabs stripped |
| `cpu_model` | `/proc/cpuinfo`, **first** `model name` line | value after the colon, internal whitespace collapsed to one space, ends trimmed |

The phrases "WSL version" and "memory size" are removed as ambiguous. All six
sources were verified readable on the target host.

## §C4. `amendment_commits` — exact form

A JSON **array of strings**, each a full 40-character SHA, ordered by amendment
number: Amendment 1 first, Amendment 2 second. Each SHA is the commit that
**first added** the corresponding file. Amendment 2 is a member of the array.

## §C5. `diag_cpu_time` — clock ticks, no conversion

The integer delta of `utime + stime` from `/proc/self/stat` over the measurement
window, **in Linux clock ticks**. Metadata carries
`cpu_time_unit = "linux_clock_ticks"`. **The conversion to seconds is
deliberately omitted**, and the reason is stated correctly rather than
overstated: a safe source does exist — `getconf CLK_TCK` returns 100 on this
host — but reading it introduces another platform route, another parse and
another failure mode into a channel that is diagnostic only. **Raw ticks are
sufficient for this diagnostic.** An earlier draft claimed no safe source
existed; that was false and is corrected here. `libc` and `sysconf` remain
excluded because they would add a dependency, and `AGENTS.md:302` permits
`unsafe` only in the engine hot path.

## §C6. `diag_ctx_switches` and `diag_freq` — fully defined

`diag_ctx_switches` is the integer delta of `nonvoluntary_ctxt_switches` from
`/proc/self/status` over the same window. The field was verified present on the
target host.

`diag_freq` is a **single integer in kHz**: the arithmetic mean, truncated toward
zero, of `/sys/devices/system/cpu/cpu<N>/cpufreq/scaling_cur_freq` over **every**
`N` in `cpu_set`, sampled **once at the end of the window**. If **any** required
file is absent or unparsable the value is `NA` and the `FREQ_MISSING` bit is set.
Partial aggregation is forbidden.

> **Established fact about the target host, not a hypothesis.** No `cpufreq`
> directory exists for any CPU in this WSL2 environment, verified by inspection.
> `diag_freq` will therefore be `NA` on every row here. That is **not** an
> instrument fault and **not** grounds for any refusal.

### §C6.1 `diag_availability`, operationally defined

Declaring only a JSON type and a key order leaves the field meaningless. Frozen:

**Sampling moment.** Availability is probed **once per process invocation**,
during `--init-run` for the manifest and again at the start of each `--controls`
or `--session N` invocation for that journal's header. It is never re-probed
mid-session, so a channel cannot appear or disappear within a journal.

**Condition for each boolean — `true` iff the probe succeeds completely:**

| key | `true` iff |
|---|---|
| `cpu_time` | `/proc/self/stat` is readable and its `utime` and `stime` fields parse as integers |
| `ctx_switches` | `/proc/self/status` is readable and carries a parsable `nonvoluntary_ctxt_switches` |
| `freq` | `scaling_cur_freq` is readable **and parsable for every CPU in `cpu_set`** — partial availability is `false`, per §C6's ban on partial aggregation |

**Manifest and journal need not match, and a mismatch is not a failure.** The
manifest records availability at allocation; each journal records it at its own
start. A channel that disappears between them is a fact about the host, recorded
in both places, and it changes no verdict. **The per-journal value governs that
journal's rows.**

**Relation to the per-row missing mask.** Availability is the *upper bound*, the
mask is the *actual outcome*. If a journal's `diag_availability` reports a
channel `false`, **every row of that journal must set that channel's missing bit
and write `NA`**. If it reports `true`, an individual row may still set the bit —
a single read can fail even when the channel exists — and that is legitimate. The
converse is not: a row may **not** carry a value for a channel the journal
declared unavailable, and such a journal is `JOURNAL-INVALID`.

**`freq = false` and its consequent `NA` are expected on the target host** and are
never grounds for refusal, per the fact recorded above.

## §C7. `diag_flags` — decimal MISSING mask

Bit 0 `CPU_TIME_MISSING`, bit 1 `CTX_SWITCH_MISSING`, bit 2 `FREQ_MISSING`. A
`diag_*` column is `NA` **if and only if** its bit is set. The committed text's
reading of `diag_flags` as an *availability* mask is superseded: it contradicted
the same section's instruction to set a bit on unavailability.

---

## §C8. Row codec

### §C8.1 Serialisation — shortest round-trip only

Every `f64` field is written in the **canonical shortest decimal form equivalent
to Rust `f64::to_string`**. Fixed-precision decimal output — a fixed number of
digits after the point — is **forbidden everywhere**, including `paired_spread`,
`timer_resolution_ms`, the sentinel durations and the load averages.

**This corrects a verdict-changing defect.** Under an eight-digit fixed format, a
`paired_spread` of `0.090000004` — a genuine failure above the `0.09` bound —
serialises to `0.09000000` and parses back to exactly `0.09`, becoming a pass.
Three of four probe values near the bound flip, and every flip is in the
direction that makes qualification easier. Shortest round-trip preserves the bits
exactly.

Requirements: finite values only; locale-independent; a conforming parser must
recover **identical `f64` bits**; `NaN` and `±inf` are forbidden and yield
`JOURNAL-INVALID`. Integer fields remain integers.

### §C8.2 The twenty-four fields

| # | field | type |
|---|---|---|
| 1 | `schema_version` | literal `rc021/1` |
| 2 | `run_uuid` | 32 hex, lower case |
| 3 | `repo_commit` | 40 hex, clean `HEAD` only; there is no `+dirty` suffix |
| 4 | `prereg_commit` | 40 hex |
| 5 | `host_fingerprint` | 64 hex |
| 6 | `phase` | `A` \| `B` \| `C` \| `NA` |
| 7 | `session` | integer `0..6`; `0` = control journal |
| 8 | `block` | integer `1..5` \| `NA` |
| 9 | `measurement_index` | integer `1..90` **for qualification sessions only** \| `NA` for every control row (§C11.4) |
| 10 | `monotonic_offset_ms` | integer, at least 0 |
| 11 | `sentinel_first_ms` | round-trip `f64` \| `NA` |
| 12 | `sentinel_last_ms` | round-trip `f64` \| `NA` |
| 13 | `paired_spread` | round-trip `f64` \| `NA` |
| 14 | `timer_resolution_ms` | round-trip `f64` |
| 15 | `load_avg_start` | round-trip `f64` \| `NA` |
| 16 | `load_avg_end` | round-trip `f64` \| `NA` |
| 17 | `cpu_set` | string containing no tab |
| 18 | `thread_count` | integer |
| 19 | `diag_cpu_time` | integer \| `NA` |
| 20 | `diag_ctx_switches` | integer \| `NA` |
| 21 | `diag_freq` | integer \| `NA` |
| 22 | `diag_flags` | integer `0..7` |
| 23 | `status` | closed domain of §5.1 |
| 24 | `reason` | JSON string per RFC 8259, quotes included; empty is the two-character empty JSON string |

Fields are separated by exactly one tab; rows end with a line feed. Because
`reason` is a JSON string, it can never contain a raw tab or line feed: both are
escaped.

### §C8.3 `LOST`, in all three forms

**A physical `LOST` row** is written only when a measurement began and did not
complete correctly. Fields 1–10, 14, 17 and 18 carry values, with **one exception
for control rows**: in the P2 and N3 journals field 9, `measurement_index`, is
`NA` (§C11.4), since the global index numbers qualification measurements only. Fields 11, 12, 13,
**15**, **16**, 19, 20 and 21 are `NA` — **both load averages are `NA`**, since a
measurement that did not complete has no meaningful end-of-window reading and its
start reading is not comparable to a completed one. `diag_flags` is 7,
`status` is `LOST`, and `reason` carries the cause. All 24 fields are thereby
determined.

**An unwritten measurement has no physical row at all.** The finalizer counts it
as a failure **synthetically, in memory**, and **never appends to the journal**:
the session's journal is closed with its process.

**A truncated physical line is never repaired and never rewritten.** The reader
constructs a **logical** `LOST` in memory only. The bytes on disk are untouched,
so the journal's SHA-256 is unaffected by reading it.

### §C8.4 `NA` in lifecycle rows

| row | `NA` fields |
|---|---|
| `SESSION-OPEN` | 6, 8, 9, 11–13, 16, 19–21; `diag_flags` is 7 |
| `SESSION-CLOSE-COMPLETED`, `SESSION-CLOSE-ABORTED` | 6, 8, 9, 11–13, **15**, 19–21; `load_avg_end` carries a value; `diag_flags` is 7 |
| `EXTERNAL-CAUSE` | as `SESSION-OPEN`; `reason` carries the cause |

---

## §C9. Metadata grammar and exact JSON types

Each metadata line is a literal `#rc021_meta`, a tab, the key, a tab, the JSON
value, and a line feed. UTF-8, line-feed endings, compact JSON with no spaces,
RFC 8259 escaping, keys in **fixed order**, followed by the unchanged 24-column
header row, followed by data rows.

| key | JSON type |
|---|---|
| `schema_version` | string, `"rc021/1"` |
| `run_uuid` | string, 32 hex |
| `boot_id` | string |
| `run_start_uptime_ms` | integer |
| `repo_commit` | string, 40 hex |
| `prereg_commit` | string, 40 hex |
| `amendment_commits` | **array of strings**, 40 hex each, ordered Amendment 1 then Amendment 2 |
| `instrument_birth_commit` | string, 40 hex |
| `host_fingerprint` | string, 64 hex |
| `cpu_set` | string |
| `thread_count` | integer |
| `timer_resolution_ms` | number, round-trip |
| `cpu_time_unit` | string, `"linux_clock_ticks"` |
| `command_line` | **array of UTF-8 argv strings**, never one joined string; a non-UTF-8 argument yields `REFUSED-BEFORE-MEASUREMENT` |
| `utc_start` | string, RFC 3339 — **provenance only** |
| `session` | **closed domain**: integer `1..6` for a qualification session, or one of the strings `"P2"`, `"N3"`, `"CONTROL"` — the last for the central control journal |
| `diag_availability` | **object of exactly three booleans in this order**: `cpu_time`, `ctx_switches`, `freq`. Operationally defined in §C6.1 |

A missing key yields `JOURNAL-INVALID`.

**The SHA-256 of a journal covers the whole file** — metadata lines, header and
data rows, as they lie on disk at closure. A post-run hash cannot sit inside the
file it hashes, so §10's clause "in the journal header **and** in the results
document" is **superseded for post-run quantities**: they exist only in the
closure record (§C13.5).

---

## §C10. Canonical file bytes and the run directory

### §C10.1 Writing

The manifest, both markers, `run_invalid.json` and every journal are written
`create_new`, then `write_all`, then `flush`, then `sync_all`, then an fsync of
the parent directory. **Overwrite is impossible anywhere.**

**`rc021_closure.json` is the one explicit exception to that sequence**, and the
exception exists to make the finalize lock checkable. Its path is **reserved
first and written last**:

```
create_new  →  fsync the parent directory  →  keep the File handle open
   … derive the classification, write the permitted artifacts …
write_all   →  flush  →  sync_all  →  fsync the parent directory
```

The gap between reservation and content is deliberate: an **empty reserved
closure path is itself the durable finalize-started marker**, so no separate
marker file is needed and no state is invisible to the next invocation. JSON files are
compact, field order fixed, UTF-8, line-feed endings, with a trailing line feed.

| file damaged or incomplete | status |
|---|---|
| `run_invalid.json` | `JOURNAL-INVALID`, exit 4 (§C12.4) |
| `run.json`, **before `controls_started` exists** | `REFUSED-BEFORE-MEASUREMENT`, exit 2; **no closure is required** — the run never began |
| `run.json`, **after `controls_started` exists** | `JOURNAL-INVALID`, exit 4 — calling it "before measurement" would be false, since the diagnostic controls have already executed |
| `controls_started.json`, `controls_complete.json`, the control journal | `INSTRUMENT-INVALID` |
| `rc021_closure.json` | `JOURNAL-INVALID` |

**A damaged manifest after `controls_started` has two branches, and both are
defined.** Every journal carries the run identity in its own metadata header
(§C9), so the manifest is often reconstructible from the surviving files.

**The complete required identity set is eight values**, and every one must be
recovered — `amendment_commits` included, since a closure must record which
amendments bound the run:

```
run_uuid   boot_id   run_start_uptime_ms   repo_commit
prereg_commit   amendment_commits   instrument_birth_commit   host_fingerprint
```

Every one of the eight is a metadata key of every journal header (§C9), so the
set is recoverable in principle.

**Branch A — reconstructible.** If all eight are recovered from the durable
headers **uniquely and consistently** — every surviving header agreeing on every
value — finalize **writes a `JOURNAL-INVALID` closure** from the reconstructed
identity, and records that fact in the closure field `identity_source` (§C13.4).

**Branch B — not reconstructible.** If any of the eight is missing from every
surviving header, or two headers disagree on any of them, the identity cannot be
asserted and a closure would have to invent it. **No closure is written**, the raw
evidence on disk stands as the record, and the run is `JOURNAL-INVALID` from that
evidence. **This is the only case in which a run that reached `controls_started` ends
without a closure *by design*.** It is scoped deliberately: it covers an identity
that cannot be established, and **not** a closure whose own write or fsync fails.
That is a different event — a durability failure of the closure itself — and it
is handled in §C14.15, which does not appeal to this exception.

### §C10.2 The run directory

`--init-run` **requires `D` to exist and to be empty of every reserved RC-021
path.** It does not create `D`; a mistyped path must fail, not silently succeed
in a fresh directory.

The **only** paths `--init-run` may create are `<D>/control/` and `<D>/run.json`.
If **any** reserved RC-021 path already exists — `run.json`, `control/`,
`controls_started.json`, `controls_complete.json`, `run_invalid.json`,
`rc021_closure.json`, any `rc021_journal_s*.tsv`, `rc021_observations.tsv`,
`RC021_RESULTS.md`, or anything under `control/` — the mode exits 2 and creates
nothing.

The **canonical absolute path** of `D`, from `std::fs::canonicalize`, is recorded
in the manifest. Every other mode canonicalises its `--dir` and requires an exact
match. A symlink resolving elsewhere, or any path mismatch, is a refusal.

### §C10.3 Reserved paths

```
<D>/run.json
<D>/controls_started.json
<D>/controls_complete.json
<D>/rc021_closure.json
<D>/run_invalid.json
<D>/rc021_journal_s{1..6}.tsv
<D>/rc021_observations.tsv
<D>/RC021_RESULTS.md
<D>/control/rc021_control_journal.tsv
<D>/control/rc021_journal_p2.tsv
<D>/control/rc021_journal_n3.tsv
```

---

## §C11. Controls

### §C11.1 Preflight is `P6` alone

**Only `P6` runs before any marker.** An earlier draft placed `P5`, `P4` and `P3`
there too, which was wrong: all three are **Class I `INSTRUMENT-INVALID`**
conditions, so putting them before the marker meant a failure left **no durable
evidence** and permitted an unlimited repeat of `--controls` — a retry-until-pass
route through the one control class that must never allow one.

`P6` is pure: it reads only and creates nothing. A `P6` refusal yields
`REFUSED-BEFORE-MEASUREMENT`, exit 2, **no marker**, and `--controls` may be
repeated after the provenance defect is fixed. That repeat is legitimate because
`P6` is a `REFUSED-BEFORE-MEASUREMENT` condition by construction: no measurement
ran and no byte was written.

`P5`, `P4` and `P3` move **after** the marker. Their failure is durable, exits 3,
and forbids repetition permanently, like every other Class I control.

### §C11.2 The exact starting order

```
1. P6                                  pure preflight, creates nothing
2. create + fsync controls_started.json     (strictly before the journal)
3. create + fsync the control journal
4. write the retrospective P6 PASS row at ordinal 1
5. P5, P4, P3                          ordinals 2, 3, 4 — each writes immediately
6. P2, P8, N2, N1, N3, P1, P7, C10     ordinals 5..12 — each writes immediately
7. fsync every row as it is written
8. create + fsync controls_complete.json
```

`P6`'s result is recorded **retrospectively**, after the journal exists, because
the journal cannot exist before the preflight that authorises creating it. Its
ordinal is nonetheless 1, preserving execution order. Every other control writes
its row **immediately on completion**, so no control's outcome depends on a later
one surviving.

**Crash semantics, frozen:**

| observed state | status at finalize |
|---|---|
| `controls_started` exists, control journal absent | `INSTRUMENT-INVALID` — the marker asserts controls began, the missing journal proves they left no evidence |
| control journal exists but carries **no `P6` row at ordinal 1** | `INSTRUMENT-INVALID` — the journal was created without the preflight that authorises it |

**Any failure after `controls_started` irreversibly forbids repeating
`--controls` in this run directory.**

### §C11.25 Precedence: a control's own class wins

The generic post-marker rule above says a failure exits 3 as
`INSTRUMENT-INVALID`. That must not silently override a class the base document
assigns to a **specific** control. §6 of the pre-registration gives **`P2` the
class `JOURNAL-INVALID`**, because P2 tests whether evidence survives a process
death, and a P2 failure means the journal contract failed — not that the
instrument is generically invalid.

**Frozen precedence, in this order:**

1. If a control has an explicit failure class in §6, **that class governs**.
   `P2` therefore fails as **`JOURNAL-INVALID`, exit 4**.
2. Otherwise the generic post-marker rule applies: `INSTRUMENT-INVALID`, exit 3.

This amendment **does not supersede** any §6 class, and no future amendment may
override one implicitly — only by naming it.

**Finalize classifies from evidence, never from absence alone.** A missing
`controls_complete.json` does not by itself mean `INSTRUMENT-INVALID`: finalize
must **read the central control journal and the P2 journal** and classify from
the durable rows it finds there. If the control journal records `P2 FAIL`, the
terminal status is `JOURNAL-INVALID`, exit 4, whatever the markers show. Only
when no control evidence distinguishes the case does the generic
`INSTRUMENT-INVALID` apply.

### §C11.3 The control journal

`<D>/control/rc021_control_journal.tsv`, append-only, `create_new` and fsynced.
Markers alone are insufficient: they are binary and do not say **which** control
failed.

It carries the §C9 metadata grammar with metadata `session` set to the string
`"CONTROL"` — never `"NA"`, which is not in the §C9 domain —
then a header, then one row per control in a closed five-column schema:

```
control_id   ordinal   status   monotonic_offset_ms   detail
```

`control_id` is one of `P6 P5 P4 P3 P2 P8 N2 N1 N3 P1 P7 C10`; `ordinal` is the
fixed position `1..12`; `status` is `PASS` or `FAIL`; `detail` is a JSON string.
**Each control writes exactly one row and fsyncs it immediately on completion**,
so a failure leaves durable evidence before any closure exists.

### §C11.4 The P2 child, and the P2 journal schema

The child creates **only** `<D>/control/rc021_journal_p2.tsv`, writes and fsyncs
**at least one** row, and terminates by `std::process::abort()` — no
`SESSION-CLOSE` row.

**The P2 journal uses the identical 24-column schema and metadata grammar of §C8
and §C9**, with no implicit borrowing. It contains exactly one `SESSION-OPEN` row
followed by at least one row whose status is `OK` or `LOST`, and it **must not**
contain any `SESSION-CLOSE` row.

**Control coordinates are frozen, because the global index belongs to
qualification sessions alone.** `measurement_index` numbers the 90 measurements
of the six sessions; a control row that borrowed a value from that range would
collide with a real measurement in every downstream count.

| journal | metadata `session` | `session` column | `phase` | `block` | `measurement_index` |
|---|---|---|---|---|---|
| P2 | `"P2"` | `0` | `A` | `1` | **`NA`** |
| N3 | `"N3"` | `0` | `A`, `B` or `C` | `1..5` | **`NA`** |
| central control journal | `"CONTROL"` | — | — | — | — (five-column schema, §C11.3) |

The parent waits for the child and requires **abnormal termination** — a signal,
not exit code 0 — checked through `std::os::unix::process::ExitStatusExt::signal`,
which is standard library and adds no dependency. It then reads the bytes and
classifies the session `ABORTED`. **P2 does not repeat within a run.**

The `N3` journal uses the same schema and the coordinates tabled above.

### §C11.45 CPU affinity and thread policy

**The instrument never sets affinity and never chooses a CPU set.** It records
and it checks; the operator establishes the environment. This makes §4.3's "a
fixed, recorded CPU set, identical in every session" operational without adding a
process-control surface.

- `--init-run` records the observed `Cpus_allowed_list` and `thread_count` in the
  manifest, unchanged.
- **The two measuring modes — `--controls` and `--session N` — perform this check
  before doing any work and before creating any file of their own**: an **exact
  match** of `Cpus_allowed_list`, `thread_count`, `host_fingerprint` and
  `repo_commit` against the manifest. A mode that has already created a file
  cannot un-create it, so the check precedes creation.
- **`--finalize` applies no live configuration or provenance qualification
  gate.** It does **not** compare `Cpus_allowed_list`, `thread_count`,
  `host_fingerprint` or `repo_commit` against the manifest, because it must
  remain able to close a run *after* a mismatch has been recorded; re-checking
  would fail on the very mismatch it exists to record, leaving the run
  permanently unclosable. An earlier draft said finalize "reads no live host
  state at all", which was too broad — it must read a little, and the exact
  narrower rule is below.
- **What finalize may read live, and only for stamping:** the current UTC, the
  current `boot_id`, and the current uptime. These stamp `finalized_utc` and
  `finalized_monotonic_offset_ms`; none of them gates anything.
- **`finalized_monotonic_offset_ms` is computed only when both hold:** the
  **current** `boot_id` equals the manifest's, **and** no durable evidence
  invalidates the axis — in particular `run_invalid.json` does not record a
  `boot_id` mismatch. Otherwise the field is **null**, because a post-reboot
  uptime is not on the same axis as `run_start_uptime_ms`.
- **Finalize never creates `run_invalid.json`.** It is written exclusively by
  `--controls` and `--session`, and a second one is impossible under the
  no-overwrite rule in any case.
- The **P2 child inherits** the parent's affinity and sets nothing of its own.
- A mismatch **before `controls_started`** is a refusal, exit 2, and may be
  corrected and retried. A mismatch **after `controls_started`** writes
  `run_invalid.json` (§C12.3) and is `INSTRUMENT-INVALID`, exit 3.

**The short gap below 600000 ms is the only correctable operational refusal after
`controls_complete`.** Every host or configuration mismatch is **terminal**: the
environment has changed under a protocol whose whole premise is that it did not.

### §C11.5 The frozen order is part of the thermal protocol

The order determines how much work precedes the first session. The two heaviest
controls are placed late: **`N3` at ordinal 9**, followed by `P1`
and `P7`, with **criterion 10 executing last at ordinal 12**, immediately before
the marker. An earlier draft said `N3` and criterion 10 "sit last, immediately
before the marker", which was false — `P1` and `P7` run between them.

**The ten-minute gap before session 1 is measured from the
`monotonic_offset_ms` recorded in `controls_complete.json`.** Sessions 2 through
6 measure their gap from the previous session's `SESSION-CLOSE` row.

### §C11.6 Review status must be evidenced

No RC-021 document may assert that it has been reviewed, or that a review
returned PASS, unless the assertion carries a verifiable reference: a commit
containing the review record, or a report line with its date and verdict. This is
kill criterion 12. It exists because the claim was made prematurely twice in this
cycle.

---

## §C12. Clock and uptime parsing

`/proc/uptime` is **not parsed as an `f64`**. The frozen decimal parser takes the
first whitespace-delimited token, splits it on the point, reads the integer part
as seconds from decimal digits only, **truncates the fractional part after the
third digit** and zero-pads it on the right if shorter, and returns
`seconds × 1000 + milliseconds`. Truncation, never rounding.

```
monotonic_offset_ms = current_uptime_ms − run_start_uptime_ms
```

Values are comparable across processes because uptime is shared by the whole
boot. A changed `boot_id`, or a current uptime below `run_start_uptime_ms`, is
detected **before any new journal is created** — but its status depends on how far
the run has got (§C12.3). UTC is recorded as provenance only and gates nothing.

### §C12.3 `run_invalid.json` — terminal host or provenance mismatch

Calling a `boot_id` or fingerprint mismatch `REFUSED-BEFORE-MEASUREMENT` is false
once `controls_started` exists: the diagnostic controls have already run. The
distinction is frozen:

| when the mismatch is detected | status |
|---|---|
| **before** `controls_started` | `REFUSED-BEFORE-MEASUREMENT`, exit 2, nothing written, correction and retry permitted |
| **after** `controls_started`, before a new session journal | write `<D>/run_invalid.json`, `INSTRUMENT-INVALID`, exit 3 |

`run_invalid.json` is written `create_new` and fsynced, is **immutable**, and
carries, in this order: `schema_version`, `run_uuid`, `boot_id_observed`,
`boot_id_expected`, `host_fingerprint_observed`, `host_fingerprint_expected`,
`reason` string, `command_line` array, `monotonic_offset_ms` integer or null when
the clock itself is untrustworthy, and `utc`.

Once it exists it **forbids all further `--controls` and `--session`**.
`--finalize` accepts it and writes a **Class I closure** recording it.

**The short gap below 600000 ms is the only non-terminal exit 2 that writes no
`run_invalid.json`**, because nothing about the host is wrong — the operator
simply has to wait.

### §C12.4 If `run_invalid.json` itself cannot be written

A failure of its `create_new`, `write_all`, `flush` or `sync_all` yields
**`JOURNAL-INVALID`, exit 4**. Execution stops immediately: no further controls
and no further sessions may run, and no partial `run_invalid.json` is repaired or
retried. The raw evidence already on disk stands as the record. An instrument
that cannot durably record why it is stopping must stop rather than continue
undocumented.

---

## §C13. File schemas and finalization

### §C13.1 `run.json` — ordered fields

| # | field | JSON type |
|---|---|---|
| 1 | `schema_version` | string `"rc021/1"` |
| 2 | `run_uuid` | string, 32 hex |
| 3 | `boot_id` | string |
| 4 | `run_start_uptime_ms` | integer |
| 5 | `run_dir` | string, canonical absolute path |
| 6 | `repo_commit` | string, 40 hex |
| 7 | `prereg_commit` | string, 40 hex |
| 8 | `amendment_commits` | array of strings |
| 9 | `instrument_birth_commit` | string, 40 hex |
| 10 | `host_fingerprint` | string, 64 hex |
| 11 | `host_fields` | object, the six §C3 keys in §C3 order, all strings |
| 12 | `cpu_set` | string |
| 13 | `thread_count` | integer |
| 14 | `timer_resolution_ms` | number |
| 15 | `cpu_time_unit` | string |
| 16 | `command_line` | array of strings |
| 17 | `utc_start` | string, RFC 3339 |
| 18 | `diag_availability` | object of three booleans |

### §C13.2 `controls_started.json` — ordered fields

| # | field | JSON type |
|---|---|---|
| 1 | `schema_version` | string |
| 2 | `run_uuid` | string |
| 3 | `boot_id` | string |
| 4 | `monotonic_offset_ms` | integer |
| 5 | `utc` | string, RFC 3339 |
| 6 | `command_line` | array of strings |

### §C13.3 `controls_complete.json` — ordered fields

| # | field | JSON type |
|---|---|---|
| 1 | `schema_version` | string |
| 2 | `run_uuid` | string |
| 3 | `boot_id` | string |
| 4 | `monotonic_offset_ms` | integer — **the origin of session 1's gap** |
| 5 | `utc` | string, RFC 3339 |
| 6 | `control_journal_sha256` | string, 64 hex |
| 7 | `control_count` | integer, must be **exactly 12** and must equal the number of data rows in the central control journal; any disagreement is `INSTRUMENT-INVALID` |
| 8 | `all_pass` | boolean, must be true |

### §C13.4 `rc021_closure.json` — ordered fields

| # | field | JSON type |
|---|---|---|
| 1 | `schema_version` | string |
| 2 | `run_uuid` | string |
| 3 | `boot_id` | string |
| 4 | `finalized_monotonic_offset_ms` | **integer or null** — null when the monotonic coordinate is untrustworthy, for instance after a `boot_id` mismatch |
| 5 | `finalized_utc` | string, RFC 3339 |
| 6 | `repo_commit` | string |
| 7 | `prereg_commit` | string |
| 8 | `amendment_commits` | array of strings |
| 9 | `instrument_birth_commit` | string |
| 10 | `host_fingerprint` | string |
| 11 | `identity_source` | string, closed domain `"MANIFEST"` or `"RECONSTRUCTED_HEADERS"` — **mandatory in every class** |
| 12 | `terminal_status` | string, one of the six §8 statuses |
| 13 | `exit_code` | integer |
| 14 | `session_states` | array of exactly 6 elements, each **string or null** — null when the session cannot be classified, for instance because its journal is damaged |
| 15 | `verdict_rules` | **object or null** — null whenever the terminal status is Class I, since no verdict may be derived |
| 16 | `failures_total` | **integer or null** |
| 17 | `failures_by_session` | **array of 6 integers or null** |
| 18 | `control_outcomes` | array of the **rows that actually exist**, `0..12` objects: `control_id` string, `ordinal` integer, `status` string, `monotonic_offset_ms` integer |
| 19 | `integrity` | array of objects, one per file of §C13.5 |
| 20 | `leave_one_out` | **object `{full_90, omissions[6]}` exactly as frozen in §C13.44, or null** — computed only from valid data, and null for every Class I outcome |
| 21 | `artifacts` | object: `observations_sha256` string or null, `results_md_sha256` string or null |

Each `integrity` element carries, in this order:

| field | type |
|---|---|
| `path` | string, relative to the run directory |
| `kind` | string, closed domain: `JSON`, `JOURNAL_TSV`, `CONTROL_TSV`, `OBSERVATIONS_TSV`, `MARKDOWN` |
| `sha256` | string of 64 hex **or the literal `MISSING`** |
| `byte_count` | integer when the file exists; **null when absent** |
| `physical_line_count` | integer when the file exists **and** its kind is tabular; **null when absent**; **null** for `JSON` and `MARKDOWN` |
| `metadata_line_count` | integer when the file exists **and** its kind is `JOURNAL_TSV` or `CONTROL_TSV`; **null when absent**; null for every other kind |
| `header_line_count` | **`0` or `1`** when the file exists **and** its kind is tabular; **null when absent**; null for non-tabular kinds |
| `parsed_row_count` | integer when the file exists **and** its kind is tabular; **null when absent**; null otherwise |
| `data_row_count` | integer when the file exists **and** its kind is tabular; **null when absent**; null otherwise |
| `status_counts` | object when the file exists **and** its kind is `JOURNAL_TSV` or `CONTROL_TSV`; **null when absent**; null for every other kind — see below |

**Existence governs first, kind second.** Every counter and `status_counts` is
`null` for an absent file, whatever its kind, which is what the fixed-inventory
rule requires: a `MISSING` entry carries `sha256 = "MISSING"` and nulls
throughout. Only for a file that exists does its `kind` decide whether a counter
is an integer or null.

**`status_counts` is per file, because §5.4 requires a count by status for each
journal and a single global object cannot satisfy that.** Its domain depends on
`kind`:

| `kind`, **for a file that exists** | `status_counts` domain |
|---|---|
| `JOURNAL_TSV` (six sessions, P2, N3) | all six journal statuses: `OK`, `LOST`, `SESSION-OPEN`, `SESSION-CLOSE-COMPLETED`, `SESSION-CLOSE-ABORTED`, `EXTERNAL-CAUSE`. For **P2 and N3** the lifecycle rows and any logical `LOST` are counted exactly as for a session journal; a control row that was never written is **not** synthesised into a journal status, because no row exists to count |
| `CONTROL_TSV` (central control journal) | `PASS`, `FAIL` |
| `JSON`, `MARKDOWN`, `OBSERVATIONS_TSV` | **null** — the first two are not row-structured by status, and observations carry no status column |
| **any absent file, of any kind** | **null** |

**Exact counter definitions**, so two readers agree byte for byte:

- `byte_count` — the file's size in bytes.
- `physical_line_count` — the number of physical lines, **counting a final line
  that lacks a trailing line feed**.
- `metadata_line_count` — the number of `#rc021_meta` lines.
- `header_line_count` — **`1`** when the first non-metadata physical line is a
  complete, line-feed-terminated line that matches the frozen header for that
  kind **exactly, field for field**; **`0`** otherwise, including an empty file,
  a file of metadata lines only, and a header line that is misspelled or
  truncated. **A valid TSV requires `header_line_count = 1`**; `0` makes the file
  `JOURNAL-INVALID` for a journal kind, and makes an artifact a failed write
  under §C13.9.
- `data_row_count` — physical lines after the header, **including a truncated
  final line**.
- `parsed_row_count` — data rows that parsed successfully; a truncated final line
  is **not** among them, so `data_row_count − parsed_row_count` is the count of
  unparsable rows.

**When the header is missing or truncated**, the counters are still defined and
must be reported rather than nulled:

| file state | `metadata_line_count` | `header_line_count` | `data_row_count` | `parsed_row_count` |
|---|---|---|---|---|
| empty file, zero bytes | `0` | `0` | `0` | `0` |
| metadata lines only | actual count | `0` | `0` | `0` |
| metadata plus a truncated or wrong header | actual count | `0` | **`0`** — with no valid header there is no "after the header", so no line is a data row | `0` |
| valid header, then data | actual count | `1` | physical lines after the header, truncated final line included | successfully parsed subset |

A file with `header_line_count = 0` therefore reports zeros for both row
counters and a real `byte_count` and `sha256`: its bytes are still evidence.

**`status_counts` uses the reader's logical interpretation**, not the raw text: a
truncated final line contributes `1` to `LOST`, exactly as §C8.3 classifies it,
even though no row on disk carries that status.

**Synthetic failures are never counted here.** An unwritten measurement has no
physical row (§C8.3), so it contributes nothing to any counter and nothing to
`status_counts`. It is counted only in `failures_total` and
`failures_by_session`, which are verdict quantities and not file quantities.

The five line counters are **not** applicable to `JSON` and `MARKDOWN`; an
earlier draft demanded them of every file without defining their meaning there,
and that is corrected by the `kind` field and the nulls above.

### §C13.44 Nested closure objects, frozen key by key

"Canonical JSON" is insufficient for nested objects: two writers agreeing on
compactness can still disagree on keys and order. Each is frozen below, keys in
the stated order.

**`verdict_rules`** — object, exactly six keys in this order:

| key | type |
|---|---|
| `rule1_all_completed` | boolean |
| `rule2_failures` | integer |
| `rule2_pass` | boolean |
| `rule3_max_per_session` | integer |
| `rule3_pass` | boolean |
| `rule4_controls_pass` | boolean |

**`session_states`** — array of exactly six elements, **ordered `s1` through
`s6`**, each either null or one of the closed domain `"NOT STARTED"`,
`"STARTED"`, `"COMPLETED"`, `"ABORTED"`. No other string may appear.

**`control_outcomes`** — array **ordered by ascending `ordinal`**, one element
per row that exists in the central control journal, each an object with exactly
four keys in this order: `control_id`, `ordinal`, `status`,
`monotonic_offset_ms`. **The `detail` column is deliberately not copied here**:
it is free-form JSON text whose byte-for-byte reproduction adds nothing a
verifier needs, and the control journal itself is hashed in `integrity`.

**`status_counts`** — object containing **every key of its domain in fixed
order, including keys whose count is zero**, so that presence never varies with
data. For `JOURNAL_TSV`, in order: `OK`, `LOST`, `SESSION-OPEN`,
`SESSION-CLOSE-COMPLETED`, `SESSION-CLOSE-ABORTED`, `EXTERNAL-CAUSE`. For
`CONTROL_TSV`, in order: `PASS`, `FAIL`.

**`leave_one_out`** — object with exactly two keys in this order:

| key | type |
|---|---|
| `full_90` | one summary object |
| `omissions` | array of exactly six summary objects, ordered by `omitted_session` ascending |

Amendment 1 §A5 requires the four quantities to be reported "for the full 90
alongside" the six omissions. An earlier draft of this amendment carried only the
six, which did not satisfy it; the `full_90` key restores the requirement.

Each **summary object** has exactly eight keys in this order:

| key | type |
|---|---|
| `omitted_session` | integer `1..6`, or null in `full_90` |
| `failure_count` | integer |
| `max_per_session` | integer |
| `median_spread` | number in the §C8.1 round-trip form, or **null** when no valid `paired_spread` remains |
| `p95_spread` | number in the §C8.1 round-trip form, or **null** on the same condition |
| `excluded_lost` | integer — physical and logical `LOST` rows excluded from the two quantiles, per Amendment 1 §A5 |
| `excluded_unwritten` | integer — **synthetic** measurements that have no row at all, from `NOT STARTED` and `ABORTED` sessions |
| `valid_spread_count` | integer — the exact denominator of both quantiles |

Both quantiles use the **nearest-rank** definition without interpolation, as
Amendment 1 §A5 froze, and both are serialised by §C8.1's round-trip rule — never
at fixed precision.

**Synthetic unwritten measurements, and why the denominator must be explicit.**
A `NOT STARTED` session contributes 15 measurements that have no row, and an
`ABORTED` session contributes one for every measurement it never reached
(§C8.3). **They count as failures** in `failure_count`, `failures_total` and
`failures_by_session`, exactly as §C13.7 requires — **and they carry no
`paired_spread`**, so they cannot enter a quantile.

The three counters make every quantile denominator checkable by arithmetic
rather than by trust. For each summary object, over its own measurement set:

```
valid_spread_count + excluded_lost + excluded_unwritten = 90   (full_90)
valid_spread_count + excluded_lost + excluded_unwritten = 75   (each omission)
```

A summary whose three counters do not sum to its set size is invalid.
`median_spread` and `p95_spread` are **null** exactly when
`valid_spread_count` is `0`.

### §C13.45 Which closure fields are mandatory, by terminal class

An earlier draft demanded fully computed scientific fields of every closure. That
made a Class I closure **unwritable**: a run with no control journal has no
twelve control rows, a damaged session journal admits none of the four session
states, and an `INSTRUMENT-INVALID` or `JOURNAL-INVALID` run has no verdict to
compute. The schema is therefore nullable where the data may not exist, and the
obligations are stated per class.

**Every one of the twenty-one top-level fields appears exactly once below**, so
no field is left without an obligation.

| # | field | Class II (`HOST-QUALIFIED`, `HOST-NOT-QUALIFIED`) | Class I (`REFUSED-BEFORE-MEASUREMENT`, `INSTRUMENT-INVALID`, `JOURNAL-INVALID`, `INCONCLUSIVE-UNDERPOWERED`) |
|---|---|---|---|
| 1–3 | `schema_version`, `run_uuid`, `boot_id` | **mandatory** | **mandatory** |
| 4 | `finalized_monotonic_offset_ms` | **present, integer or null** | **present, integer or null** |
| 5 | `finalized_utc` | **mandatory** | **mandatory** |
| 6–13 | `repo_commit`, `prereg_commit`, `amendment_commits`, `instrument_birth_commit`, `host_fingerprint`, `identity_source`, `terminal_status`, `exit_code` | **mandatory** | **mandatory** |
| 14 | `session_states` | 6 strings, all non-null | 6 elements, each string **or null** |
| 15 | `verdict_rules` | **mandatory object** | **must be null** |
| 16–17 | `failures_total`, `failures_by_session` | **mandatory** | **must be null** |
| 18 | `control_outcomes` | 12 rows | the `0..12` rows that exist |
| 19 | `integrity` | **mandatory, full fixed inventory** | **mandatory, full fixed inventory** |
| 20 | `leave_one_out` | **mandatory** | **must be null** |
| 21 | `artifacts` | hashes when created, else null | **both null** |

**Field 4 is present in both classes and is never omitted**; it is `null` exactly
under §C11.45's invalid-axis condition — a current `boot_id` differing from the
manifest's, or durable evidence that the axis is broken — and an integer
otherwise. **Field 5 is mandatory in every closure**: a closure is always written
at a knowable wall-clock moment, whatever else is unknowable.

**A Class I closure that carries a non-null `verdict_rules`, `failures_total`,
`failures_by_session` or `leave_one_out` is itself invalid**, because it would
publish a derived scientific quantity for a run that earned none. This is the
schema-level expression of the standing rule that a Class I outcome is never
written up as a Class II result.

**The closure record does not contain its own hash**, which is impossible; a
verifier recomputes it from the file itself.

### §C13.5 Integrity inventory — the exact file list

The `integrity` array is a **fixed ordered set of fifteen elements, always all
fifteen**, in this order:

```
 1 run.json
 2 controls_started.json
 3 controls_complete.json
 4 run_invalid.json
 5 control/rc021_control_journal.tsv
 6 control/rc021_journal_p2.tsv
 7 control/rc021_journal_n3.tsv
 8 rc021_journal_s1.tsv
 9 rc021_journal_s2.tsv
10 rc021_journal_s3.tsv
11 rc021_journal_s4.tsv
12 rc021_journal_s5.tsv
13 rc021_journal_s6.tsv
14 rc021_observations.tsv
15 RC021_RESULTS.md
```

Fifteen elements, and **every one is present in every closure**, including
`run_invalid.json` and both artifacts. **A file that does not exist appears with
`sha256` set to the literal `MISSING`, `byte_count` null and every counter
null.** An earlier draft wrote "when created" of the two artifacts and then, one
sentence later, that a non-existent file is present anyway; the two statements
contradicted each other and the fixed set removes the ambiguity.

### §C13.6 Finalize preconditions

`--finalize` is permitted when `controls_started.json` exists, **even without
`controls_complete.json`**. Otherwise a control failure would leave a run with no
closure, contradicting §C14.1.

`controls_started` present and `controls_complete` absent is **not classified
from that absence alone** (§C11.25). Finalize reads the central control journal
and the P2 journal and takes the class of the **first `FAIL` row by ordinal**,
using §C11.25's precedence: a `P2 FAIL` yields `JOURNAL-INVALID`, exit 4; any
other `FAIL`, or no distinguishing evidence at all, yields `INSTRUMENT-INVALID`,
exit 3. The closure records the control outcomes actually present and every hash
or `MISSING` then available.

**Sessions are permitted only when `controls_complete.json` exists.**

### §C13.7 Journal classification

| case | status |
|---|---|
| damaged header or damaged interior row | `JOURNAL-INVALID` |
| truncated **final** line | logical `LOST`; the journal is valid |
| **absent** session journal | `NOT STARTED`; its 15 measurements count as failures; §7.2 rule 1 fails, giving `HOST-NOT-QUALIFIED`; `sha256` is `MISSING` |

**§5.4 is superseded only for the `NOT STARTED` case**, where its rule that a
missing SHA implies `JOURNAL-INVALID` would otherwise turn an ordinary
non-start into an instrument fault and contradict §5.2 and §7.2.

### §C13.8 Output artifacts

`rc021_observations.tsv` is created **only** for a valid Class II result with all
six sessions `COMPLETED` and valid controls and journals. A
`HOST-NOT-QUALIFIED` arising from a `NOT STARTED` session receives **no**
observations, because the six-`COMPLETED` condition is false.

The results Markdown is created for **both** Class II verdicts. **A Class I
outcome receives a closure record but never a scientific results Markdown and
never observations.**

### §C13.85 `rc021_observations.tsv`, frozen

The artifact is now mandatory for a permitted Class II result, so its bytes must
be as determined as the journals'.

**Header, exactly these eight columns in this order**, then a line feed:

```
session   phase   block   measurement_index   monotonic_offset_ms   sentinel_first_ms   sentinel_last_ms   paired_spread
```

| column | type |
|---|---|
| `session` | integer `1..6` |
| `phase` | `A`, `B` or `C` |
| `block` | integer `1..5` |
| `measurement_index` | integer `1..90` |
| `monotonic_offset_ms` | integer |
| `sentinel_first_ms`, `sentinel_last_ms`, `paired_spread` | numbers in the §C8.1 round-trip form — **never fixed precision** |

**Row inclusion.** Exactly the rows whose journal status is `OK`. Lifecycle rows,
`LOST` rows, logical `LOST` from a truncated line, and synthetic unwritten
failures are **all excluded**. **This is why the file has no `status` column**:
every included row has the same status by construction, so a column carrying one
constant would be noise. The earlier assertion that observations carry no status
column is therefore correct and is now explained rather than merely stated.

**Ordering.** Strictly ascending `measurement_index`. Since the index is global
and unique across the run, the order is total and no tie rule is needed.

**Serialisation.** One tab between fields, a line feed after every row including
the last, UTF-8, no metadata lines and no comment lines — the file is a plain
header plus data, and its provenance lives in the closure.

**The row count is the sum of `OK` measurement rows across the six session
journals.** For six `COMPLETED` sessions that is **90 minus the number of `LOST`
measurement rows** — physical and logical — and **not** 90 minus all failures.
The distinction matters: an `OK` row whose `paired_spread` exceeds `0.09` is a
**failure for the verdict but is still present in the observations**, because it
is a real measurement with a real spread. Only rows that produced no spread are
absent. The count is checkable against `status_counts` in `integrity`.

### §C13.9 Artifact write order, frozen

The order is fixed for every class, because the closure must hash artifacts that
already exist while its **path** must be claimed before any of them:

```
1. read-only preflight            — read manifest, markers, journals; ESTABLISH
                                    IDENTITY; write nothing
2. create_new rc021_closure.json  — EMPTY; fsync the parent; keep the handle open
3. derive the remaining terminal
   and scientific classification  — only now, with the path reserved
4. observations TSV               (only when permitted by §C13.8)
5. results Markdown               (Class II only)
6. compute SHA-256 and byte_count of both
7. write_all the complete compact closure JSON through the already-open handle
8. flush → sync_all → fsync the parent directory
```

**Identity is established in step 1, before anything is reserved.** Step 1 reads
and validates the manifest; if it is damaged, step 1 decides between §C10.1's two
branches. **Branch A** recovers the eight identity values from the durable
headers, sets `identity_source` to `"RECONSTRUCTED_HEADERS"`, and continues to
step 2. **Branch B — unreconstructible — exits `JOURNAL-INVALID`, exit 4,
*without reserving the closure path***, exactly as §C10.1 requires: a closure
that cannot name whose run it is must not be created at all, not even as an empty
reservation.

Step 3 therefore derives only what remains: the terminal status and, for Class
II, the scientific classification. Identity is never derived after reservation.

**The closure's path is reserved first and its bytes are written last.** Step 2
is what makes the lock observable: from that moment a path exists on disk, so a
later crash is visible to the next invocation instead of leaving a state nothing
can detect. Steps 3 onward happen only after the reservation, so no artifact is
ever created by an invocation that has not claimed the closure.

**If an artifact write or fsync fails**, including a partially written
observations file or Markdown:

- **if the already-reserved closure is subsequently written successfully**, its
  `integrity` element for that artifact records the artifact's **actual bytes and
  actual hash**, not `MISSING` — a partial file exists and its bytes are evidence;
- **if the closure write also fails**, no such record can be made, and the
  **empty or partial closure path is itself the terminal evidence** (§C14.16),
  alongside the partial artifact on disk;
- `terminal_status` is **`JOURNAL-INVALID`**;
- **no Class II verdict is published**, and by §C13.45 `verdict_rules`,
  `failures_total`, `failures_by_session` and `leave_one_out` are all null;
- **a repeat `--finalize` is forbidden by the existing reserved artifact path**
  under §C10.1's no-overwrite rule, **even if the closure could not be written**.
  Otherwise a failed finalize could be retried until one succeeded, which is
  retry-until-pass at the last step of the cycle.

---

## §C14. Closure obligation, terminal lock and verification

### §C14.1 When a closure is obligatory

A closure record is **obligatory** for every allocated run that reached
`controls_started` **or** created at least one session journal, and was then
finalized.

**A pure preflight failure before `controls_started` does not create a closure**,
and neither does CLI misuse: the run never began. **After `controls_started`, any
failure is finalized by a separate `--finalize` invocation** — `--controls`
itself writes durable failure evidence to the control journal and **never writes
a closure**.

**`--controls`' own exit code follows §C11.25's precedence, not a blanket
value.** A `P2` failure exits **4** as `JOURNAL-INVALID`; every other post-marker
control failure exits **3** as `INSTRUMENT-INVALID`. An earlier draft of this
section said `--controls` "exits 3", which silently overrode the class the base
§6 table assigns to `P2`.

### §C14.15 When a closure cannot be durably written

If any of the closure's `create_new`, **the fsync of its parent directory after
reservation**, `write_all`, `flush`, `sync_all`, or **the fsync of its parent
directory after writing** fails, the outcome is **`JOURNAL-INVALID`, exit 4**,
and no partial closure is repaired or retried. This is a durability failure, not
the §C10.1 identity exception, and the two must not be conflated: one is "we do
not know whose run this is", the other is "we know, and the disk refused".

**Once `create_new` has succeeded, any later failure or crash leaves a closure
path that is empty, partial or complete.** In all three states the run is locked
and **no retry may write anything**. `--verify` classifies an **empty or
partial** closure as **`JOURNAL-INVALID`, exit 4**; only a **valid, complete**
closure means finalization succeeded.

### §C14.155 When `create_new` itself fails

An earlier draft permitted a retry on four conditions, one of which required the
durable inputs to carry "the same SHA-256 they had at the failed attempt". **That
was uncheckable**: a `create_new` failure leaves no closure path, and therefore
no stored baseline to compare against. A condition no observer can evaluate is
not a condition, and the whole four-part policy is **withdrawn**.

**The replacement is the reservation itself.** Because the closure path is
claimed before anything is derived or written (§C13.9 step 2), the two cases are
distinguishable by a read-only check that needs no history:

| observed | meaning | permitted |
|---|---|---|
| **no closure path exists** | `create_new` never succeeded, so **no finalize-owned byte was written** — no artifact, no closure, nothing | the invocation **may be repeated safely** |
| **a closure path exists**, empty, partial or complete | the reservation succeeded; the run is locked | **no retry may write anything** (§C14.16) |

**No historical SHA comparison is claimed anywhere.** The check is the presence
or absence of one path, evaluated now.

**What is and is not deterministic.** An earlier draft claimed finalization as a
whole is a deterministic function of the durable bytes. That is **false**: the
closure carries `finalized_utc`, and `finalized_monotonic_offset_ms`, which
differ between invocations by construction. The narrower true statement is
retained:

> The **scientific counters, the verdict and the terminal classification** are
> deterministic functions of unchanged durable inputs. `finalized_utc` and the
> final monotonic stamp are **provenance**, not results, and may differ between
> invocations.

That is what makes a repeat after a failed reservation harmless: it cannot change
a counter, a verdict or a classification — only the moment stamped on them.

### §C14.16 Terminal evidence lock

**Under the frozen §C13.9 order, an instrument-created artifact can never exist
without a closure path.** The closure path is reserved at step 2 and artifacts are
written at steps 4 and 5, so every artifact this instrument writes is preceded by
a reservation. An earlier draft described "no closure but an artifact already
present" as a reachable state; under reservation-first ordering it is not one.

**An artifact present with no closure path is therefore external or corrupt
legacy state** — something outside the instrument put it there, or the directory
was reused. It is **`JOURNAL-INVALID`, exit 4, with no writes**, and it is not
treated as a stage of a run this instrument conducted.

**Any existing closure path is the normal terminal evidence lock.** A subsequent
`--finalize` performs no writes of any kind: it does not overwrite, complete or
remove an artifact, and it does not attempt the closure again. The reason is the
standing one — retrying a finalize until one succeeds is retry-until-pass at the
last step of the cycle, and the reserved-path rule of §C10.1 exists to forbid it.
The bytes on disk — the journals, any partial artifact, the control evidence and
the closure path itself — are the record.

### §C14.2 Terminal lock

A closure **path** and a **valid complete closure** are different states, and
they lock the run differently. The distinction is frozen here and propagated to
every state-machine row.

| observed state | `--controls`, `--session`, `--finalize` | `--verify` |
|---|---|---|
| **valid, complete closure** — finalization succeeded | refuse with `REFUSED-BEFORE-MEASUREMENT`, **exit 2**; no writes, no measurements | **verifies normally** (§C14.3) |
| **empty or partial closure path** — the reservation succeeded, the content did not | the run is **`JOURNAL-INVALID`**: no writes, no measurements, **exit 4** | no writes, **exit 4** |

The difference is not cosmetic. A **complete** closure means the run reached a
terminal result and there is nothing left to do — exit 2, "refused because it is
finished". An **empty or partial** path means the run's own terminal record was
never durably written, so the run is not merely finished but **broken** — exit 4,
and even `--verify` cannot verify against a record that does not assert a
completed finalization.

In both states no measurement runs and no byte is written, so no run can continue
past a terminal result either way.

### §C14.3 `--verify`

It re-reads the manifest, `run_invalid.json` when present, the control evidence,
every journal and the closure. **An empty or partial closure is
`JOURNAL-INVALID`, exit 4** — only a valid, complete closure can be verified
against, because only that one asserts that a finalization actually completed.

**It is class-aware, because a Class I run has no scientific verdict to
recompute** and demanding one would contradict §C13.45, which requires those
fields to be null.

**Always recomputed, for every class:** every `sha256`, every `byte_count`, every
**per-file `status_counts`**, the five line counters, the full fifteen-element
inventory, the session states, the control outcomes and the **terminal
classification** itself.

**Recomputed only when the closure's `terminal_status` is Class II:**
`verdict_rules`, `failures_total`, `failures_by_session` and `leave_one_out`.

**When the closure's `terminal_status` is Class I**, `--verify` instead
**asserts that those four fields are null**, and a non-null value among them is a
mismatch. Any mismatch exits 4.

A **damaged or unreadable manifest** encountered by `--verify` is exit 4:
verification cannot proceed against a run whose identity cannot be established.

It **reads no live host state** and **writes no bytes**.

---

## §C15. Chronology by strict ancestry

```
instrument_birth_commit = git log --follow --diff-filter=A --format=%H \
    -- src/bin/exp_rc021_host_qualify/main.rs
```

**Exactly one result is required**; zero or more than one is a refusal. Required:
the pre-registration and all its amendments are **strict ancestors** of that SHA;
that SHA is an ancestor of `HEAD`; every instrument file and binding document is
tracked and clean. Verified with `git merge-base --is-ancestor` only — never by
comparing dates, which rebase rewrites, cherry-pick changes and clock skew
corrupts. Timestamps remain provenance and gate nothing.

## §C16. Architecture

Exactly `src/bin/exp_rc021_host_qualify/` with `main.rs` and the private modules
`journal.rs`, `manifest.rs`, `provenance.rs`, `host.rs`, `protocol.rs`,
`controls.rs`, `decision.rs`. No line limit is imposed: `AGENTS.md` contains no
such rule, and an earlier draft of this amendment invented one.

## §C17. Test isolation, and what is preserved

A P2 attempt suffix is **forbidden**: the real P2 runs once under refuse-if-exists.
Test repeatability comes from a **unique `--dir`** built from the process id, an
atomic counter, the test name and a cleanup guard. Any other scheme reintroduces
retry-until-pass through a back door.

**Preserved exactly, and restated so no reader has to infer it:**

- the diagnostic seeds `31001`, `31002`, `31003`, `31004` and the reserved band
  from `31001` to `31099`;
- the **inherited `0.09`** bound, neither lowered nor reinterpreted;
- `6 × 3 × 5 = 90`, the phase order A then B then C, and the **fixed denominator
  of 90**, with lost, truncated and below-floor measurements counting as
  failures;
- **no retries**, no discarded-and-rerun sessions, no seventh session, and
  `INCONCLUSIVE-UNDERPOWERED` terminal with zero restarts;
- every prohibition of §1 and of Amendment 1: no marginal cost, no operator
  ratio, no equal-cost budget, no energy, cut, quality or outcome quantity, no
  operator ranking, no sensor sufficiency, no scheduler or policy claim, no
  superiority claim, no external benchmark, product or breakthrough claim;
- the **engineering-only meaning** of both verdicts. `HOST-QUALIFIED` means only
  that the host and instrument passed this frozen protocol on this configuration
  on this occasion. `HOST-NOT-QUALIFIED` means only that they did not pass it.
  Neither states anything about a true rate `p`, and neither is a statistical
  certification.

---

## State machine

`--dir` is mandatory in every mode. No path is hard-coded.

| mode | preconditions | effect | refusal |
|---|---|---|---|
| `--init-run --dir D` | `D` exists; **no reserved RC-021 path exists** | creates `control/` and `run.json` | exit 2 |
| `--controls --dir D --run-id R` | manifest readable — damaged **before** `controls_started` exits 2, **after** it exits 4 — with `R`, canonical path, `boot_id`, affinity, thread count, fingerprint and `repo_commit` all matching; **no `run_invalid.json`**; **no closure path** — a *complete* closure exits 2, an *empty or partial* one exits 4 (§C14.2); neither marker exists | `P6`, then `controls_started`, then the control journal, then the retrospective `P6` row, then controls 2–12, then `controls_complete` | `P6` before the marker exits 2 and may be repeated; after the marker every failure leaves durable evidence and forbids repetition permanently, with the class from §C11.25 — **`P2` exits 4 as `JOURNAL-INVALID`**, every other control exits 3 as `INSTRUMENT-INVALID` |
| `--session N --dir D --run-id R` | all of the above; **no `run_invalid.json`**; **no closure path** — complete exits 2, empty or partial exits 4 (§C14.2); `controls_complete` exists; affinity, thread count, fingerprint and `repo_commit` match; sessions `1..N−1` are `COMPLETED`; gap at least 600000 ms; no journal `N` | creates journal `N` with `create_new`, runs 15 pairs | a short gap exits 2 **before the journal is created** and may be retried after waiting — it is an **operational refusal, never a terminal run-level verdict**; a host or provenance mismatch writes `run_invalid.json` and exits 3 |
| `--finalize --dir D --run-id R` | **`controls_started` exists OR `run_invalid.json` exists**; **no closure**. It applies **no live qualification or configuration gate**; it may read only the current UTC, the current `boot_id` and the current uptime, and only to stamp the closure (§C11.45). It never writes `run_invalid.json`. A manifest damaged after `controls_started` is `JOURNAL-INVALID`, exit 4, and splits: **reconstructible** identity ⇒ a closure is written; **not reconstructible** ⇒ §C10.1's sole no-closure exception | artifacts in the §C13.9 order, then the closure last; Markdown for Class II; observations only for six `COMPLETED` | `controls_complete` absent is **never classified from that absence alone** — finalize reads the control and P2 journals and applies §C11.25, so a recorded `P2 FAIL` exits 4 and any other `FAIL` exits 3; `run_invalid.json` yields a Class I closure; **a complete closure exits 2; an empty or partial closure path exits 4** (§C14.2), both with no writes; **an artifact with no closure path is external or corrupt state**, `JOURNAL-INVALID`, exit 4 (§C14.16); if **no** closure path and no artifact exist, no finalize-owned byte was written and the invocation may be repeated safely (§C14.155) |
| `--verify --dir D` | a closure path exists. A **valid complete** closure verifies normally; an **empty or partial** one is `JOURNAL-INVALID`, exit 4, with no writes; a damaged manifest exits 4 | **terminal classification and integrity are always recomputed; the Class II scientific verdict and leave-one-out only when the closure is Class II** — for Class I those four fields are asserted null (§C14.3) | any mismatch exits 4 |
| hidden P2 child | invoked only by P2 | an fsynced `SESSION-OPEN` row, then **at least one** fsynced measurement row with status `OK` or `LOST` at `phase = A`, `block = 1`, `measurement_index = NA`, then `abort()` — no `SESSION-CLOSE` row | — |

**Exit codes.** `0` is `HOST-QUALIFIED` or mode success. `1` is
`HOST-NOT-QUALIFIED` — **a result, not an error**. `2` is
`REFUSED-BEFORE-MEASUREMENT`. `3` is `INSTRUMENT-INVALID`. `4` is
`JOURNAL-INVALID`. `5` is `INCONCLUSIVE-UNDERPOWERED`.

---

## Implementation plan — at most ten green commits

An independent read-only review of this amendment precedes commit 1.

**Every source commit — 2 through 9 — must pass the full `AGENTS.md` gate set
before it is made**, not only the integration commit:

```
cargo check
cargo test --release
cargo build --release --bins
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

An earlier draft deferred the full set to step 8. That is exactly how a commit
that "looks green" reaches the history without being green, and it is corrected
here.

| # | commit |
|---|---|
| 1 | `docs(research): amend RC-021 journal and run contract` — a strict ancestor of the instrument's birth |
| 2 | `feat(rc021): journal and row codec` — 24 fields, round-trip serialisation, the three forms of `LOST`, create-new and fsync discipline |
| 3 | `feat(rc021): manifest, clock and provenance` — `OsRng` identifier, fingerprint, canonical path, integer uptime parser, `boot_id`, ancestry gate |
| 4 | `feat(rc021): seeds, sentinel and /proc diagnostics` — the four seeds, disjointness, paired measurement, the resolution floor, ticks, context switches, kHz with `NA` |
| 5 | `feat(rc021): protocol and gap enforcement` — the frozen counts, phase order, warmup, load block, affinity, gap checked before journal creation |
| 6 | `feat(rc021): controls` — preflight, both markers, the control journal, the frozen order, the P2 child |
| 7 | `feat(rc021): decision, finalize and verify` — the §7.2 predicate, the statuses, the descriptive table, the five counters, closure, terminal lock, read-only verify |
| 8 | `feat(rc021): CLI and end-to-end synthetic tests` — five modes, transitions, exit codes, an end-to-end run over synthetic journals, full gates |
| — | **independent reviewer** |
| 9 | *optional* `fix(rc021): review corrections`, followed by a **repeat** review |
| 10 | `docs(research): record RC-021 instrument review` — **only after PASS** |

If the first review passes, step 9 is skipped and the plan is nine commits. The
real six-session run is **not** part of this plan.
