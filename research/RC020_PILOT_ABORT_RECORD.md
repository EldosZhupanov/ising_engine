# RC-020 pilot — abort record

**Status:** RC-020 is **closed without a scientific verdict.** Gate A has **NO
VERDICT**. Two pilot attempts were made; neither produced an artifact. The cycle
is not repaired and not re-run.

**Binding summary.** Nothing in this document is evidence about marginal wall
cost `b(i,o)`, about operator ordering, about equal cost, or about any cost
model. RC-020 measured nothing that survived.

---

## 1. Chronology

### Attempt 1 — refused at the gate, 2026-08-22

The pilot was invoked once. The in-process control set **failed at P3 sentinel
liveness**, and the failure was on P3's *clean* half: two identical back-to-back
sentinel executions drifted beyond the frozen `0.09` bound while the injected
slowdown was correctly caught.

| field | value |
|---|---|
| exit status | **2** (`PILOT REFUSED — controls failed (K1)`) |
| classification | **Class I**, K1 |
| pilot body | **did not run** — controls precede it |
| seeds `10001–10008` | **not executed** |
| artifact | none; `experiments/rc020/` was not created |

**Provenance of attempt 1 — stated plainly because it is weak.** The observed
figures were *clean 2.756 / 2.373 ms, spread 0.1612* against the `0.09` bound,
with the injected arm at `0.7640` correctly flagged. The run log lived in an
ephemeral scratchpad under `/tmp`, which is `tmpfs` and **was destroyed by the
subsequent `wsl --shutdown`**. No log survives. The only other trace is the
orchestration message `msg_c57fccbbc13f`.

`PROJECT_PLAN.md` §S1 correctly refused to let the `16.1%` figure be cited while
unrecorded. **This document is its first and only committed record**, and the
number is recorded here as *an observation by the agent that ran it*, with no
surviving machine-readable source. It may be cited only with that qualification,
and it may not be entered into any threshold, model or descendant.

### Host remediation between the attempts

Performed by the user, at the agent's request, before attempt 2:

- other agents (`codex`, `codex-code-mode`, `agy`, a second `claude`) closed;
- `/mnt/c/Users/…/.wslconfig` changed from `memory=4GB` to `memory=10GB`
  (backup `.wslconfig.bak-20260822-200657`; `processors=4` left unchanged);
- `wsl --shutdown` executed; the VM rebooted at **20:21:21**, after the config
  write at 20:07:17, so the new limit took effect.

Verified after reboot: WSL total **9947 MiB** (was 3917), 1-minute load
average **0.08**, `nproc` **4**, tree clean at `9c48f10`,
`experiments/rc020/` absent.

### Attempt 2 — pilot body ran, session discarded, 2026-08-22

A standalone readiness check was run first. It passed with the host at its
quietest observed state: **`N1` spread `0.0012`**, **`P3` clean spread
`0.0000`** (1.989 / 1.989 ms), load `0.13`.

The pilot was then invoked **once**, with `--dir experiments/rc020`. Its
**in-process** controls — the ones §10.5 makes binding — passed, but at
materially worse margins than the standalone check taken two minutes earlier:

| control | in-process (attempt 2) | standalone, 2 min earlier |
|---|---|---|
| `N1` same-code null | **0.0755** — 84% of the `0.09` bound | `0.0012` |
| `P3` clean half | **0.0397** | `0.0000` |
| `P3` injected half | `0.8764`, correctly flagged | `1.0462`, flagged |
| load average | `0.11` | `0.13` |
| timer resolution | `0.000020 ms` | `0.000010 ms` |

All other controls passed: A1 seed disjointness over 117 named values, P2 all six
corpus hashes, P1 synthetic recovery err `0.0017`, N2 CI containing zero, N3
state/energy/RNG matching for both operators, P4 both arms, §10 provenance,
§10.7 reserved blocks locked, §10.1 schema 22 columns.

Then:

```
RC-020 pilot — seeds [10001 … 10008], 9 repetitions

  repetition 1 discarded: SENTINEL-DRIFT (1 so far)
  repetition 1 discarded: SENTINEL-DRIFT (2 so far)

  PILOT ABORTED — session discarded: repetition 1 exceeded the bound twice
  No artifact written. Record this discard in the results document.
```

| field | value |
|---|---|
| exit status | **5** |
| repetition 0 | completed |
| repetition 1 | discarded twice for `SENTINEL-DRIFT` |
| failed executions | **2** (both of repetition 1) |
| unique discarded repetition IDs | **1** |
| internal discard counter | **2** |
| artifact | **none**; `experiments/rc020/` does not exist |
| Gate A | **NO VERDICT** |
| held-in `11001–11008` | **untouched** |
| held-out `12001–12008` | **untouched** |
| descendant §9 | **still blank and frozen** |

**Attempt-2 log — preserved as committed evidence.** The raw 41-line log is
committed verbatim as `research/RC020_pilot_attempt2_raw.txt`, SHA-256
`03823079a1f6b826a5855092958130193045f7b6238f15a9bf9365ea1e064813`, so every
figure quoted above is independently checkable against committed bytes rather
than against a hash of a file that no longer exists. (The `.txt` extension is
forced by `.gitignore:5`, which excludes `*.log`; the content is byte-identical
to the run's output and the hash is unchanged by the rename.) It contains **zero** per-instance and
**zero** per-seed result lines — verified by search — so preserving it discloses
no science datum.

Attempt 1's log cannot be preserved: it was destroyed before this record was
written, and the caveat above stands for it alone.

## 2. Why the session stopped, and what it is *not*

§5's sentinel rule fired: a repetition whose sentinel drift exceeds `0.09` is
discarded and re-executed **at most once**; if the re-execution also exceeds the
bound, **the session is discarded**. That is exactly what happened at repetition
1.

**`K3` did not fire, and must not be reported as if it had.** `K3`
(`HOST-UNSTABLE`) requires *more than* 2 of 9 repetitions discarded.

Counting matters here, and the naive phrasing "2 of 9 repetitions were
discarded" is **wrong**: both failures belong to the *same* repetition. The
instrument increments `discarded` once per failed *execution*, inside the retry
loop, while `rep` is held fixed. Precisely:

> **failed executions = 2; unique discarded repetition IDs = 1; internal discard counter = 2.** `K3` cannot fire under any reading, because no value exceeds 2.

The session ended on §5's twice-failed-repetition clause, not on the discard
counter.

### Finding — a gap in §8's taxonomy

This outcome matches **none** of `K1`–`K8`:

| criterion | why not |
|---|---|
| K1 | every control passed |
| K2 | the sentinel is present and both fields are in the schema |
| K3 | requires **> 2**; failed executions = 2, unique discarded repetitions = 1, counter = 2 — no reading exceeds 2 |
| K4 | no degenerate-timing bound was exceeded |
| K5 | no forbidden seed executed; held-in/held-out never opened |
| K6 | every §10 provenance requirement passed |
| K7, K8 | both require Gate A to return a verdict; Gate A never ran |

§5 defines the session-discard path but §8 does not classify it. Substantively
it is **Class I** — no result of any kind — and it is treated as Class I here.
But §8 as written does not say so, and that is a **pre-registration defect**, not
an interpretation the reader should have to supply. A successor must close it.

## 3. Seed disposition — conservative

**Seeds `10001–10008` are treated as BURNED and are not reusable.**

This is deliberately the conservative call, and the reasoning is recorded so a
future reader can see it is a *choice*, not a deduction:

- **In favour of burned:** the pilot body executed. Repetition 0 ran to
  completion across all 72 conditions and all eight seeds, and repetition 1 ran
  twice more. The seeds were consumed by real execution.
- **Against burned:** nothing was observed. No artifact was written, the
  accumulated rows died with the process, and the 41-line log contains **zero**
  per-instance and per-seed lines — verified by search. No human or agent ever
  saw an outcome tied to any of these seeds. This differs from both prior burns:
  RC-017's `5001–5008` were burned because summaries **were printed**, and
  RC-018's `7001–7008` because timings **were written to disk and read**.

Because execution and observation genuinely diverge here, and because the cost of
wrongly reusing a seed is far higher than the cost of retiring one, **the
conservative reading governs**. Any successor uses fresh blocks.

The prior prohibitions stand unchanged: `1001–3008` (RC-016), `4001–4008`
(RC-017 pilot), `5001–5008` **burned**, `6001–6008` reserved, `7001–7008`
**burned** (RC-018), and now `10001–10008` **burned**. `11001–11008` and
`12001–12008` remain **reserved and unopened**.

## 4. Cause — hypotheses, none established

**No cause is established.** What follows is recorded as candidate explanations,
each with the evidence for and against.

**H1 — thermal throttling under sustained load.** Repetition 0 executes 576
trajectories; sustained load could reduce clock frequency so that the sentinel
at the *end* of repetition 1 is slower than the one at its *start*.
*For:* repetition 0 passed and repetition 1 failed, twice, which is the pattern a
warm-up-then-throttle mechanism would produce.
*Against, partially:* the **in-process** `N1` paired spread was already `0.0755`
— 84% of the bound — and `P3` `0.0397`, **before the first science repetition**,
whereas a standalone check two minutes earlier gave `0.0012` and `0.0000`.

**This is weaker evidence than it first appears, and the weakening is recorded
rather than glossed.** The control phase is not an idle period: by the time `N1`
is measured the process has already executed a warmup trajectory
(`exp_marginal_cost.rs:947`), two 100 000-resample bootstraps (P1, N2), and it
goes on to run N3, P3 and P4; `run_pilot` then executes twelve further full
warmup trajectories (`:1258`) before repetition 0. So these figures were *not*
taken without sustained load.

The correct statement is therefore: **elevated spread existed already in the
control phase, so warming from repetition 0 alone is insufficient as an
explanation — but the control phase itself loads the CPU, so H1, H2 and H3 are
not distinguished by these data.**

**H2 — bursty host variance.** The host's timing behaviour is episodic;
a standalone check can land in a quiet window and pass at `0.0000` while a run
minutes later starts at `0.0755`.
*For:* the standalone-vs-in-process gap above; the fact that attempt 1 failed the
same guard outright.
*Against:* not independently characterised. **No distribution was estimated at
all** — each figure is a paired spread over two measurements, which supports no
variance claim.

**H3 — CPU contention.** `nproc` is 4 and the protocol pins the instrument to
CPUs `0-3`, so the measurement shares every available core with the agent
process, shells and system tasks. There is no core the instrument does not
compete for.
*For:* mechanically certain to add jitter; the host has 16 logical processors and
WSL was given 4.
*Against:* load average was `0.11`–`0.13` throughout, which is low.

**Method note, binding on any successor.** These hypotheses may not be settled by
re-running the pilot until it passes. Repeating a run until a drift guard admits
it selects an atypically quiet moment and biases the very timing quantity the
cycle measures. **The instrument must work reliably, not pass once by luck.**
A successor must first characterise the host's timing distribution as its own
object of study, with a pass criterion fixed in advance.

## 5. Instrument defect — discard rows are lost on session abort

**Severity: real, and it caused this document to be harder to write than it
should have been.**

In `src/bin/exp_marginal_cost.rs`, `run_pilot` accumulates output in an in-memory
`String`. A discarded repetition **is** appended to that buffer via
`fmt_discard`. But both abort paths — the `HOST-UNSTABLE` return and the
twice-failed-repetition return — `return Err(...)` **before** the single
`std::fs::write` that persists the buffer. The accumulated rows, including every
discard row, are therefore **discarded with the process**.

§5 requires that a discarded repetition's reason **"is written to the artifact"**
and that the session discard **"is recorded — never silently dropped."** The
instrument printed the reason to stdout, which is not nothing, but it wrote
nothing, and under `AGENTS.md`'s recording rule an unrecorded run did not happen.
**§5 was violated.**

### The underlying pre-registration defect

This is not only a coding slip. §5 requires the discard to be recorded, while
**§10.1 requires the TSV to be "written only on successful completion."** On an
aborted session the two clauses demand opposite things, and a single-artifact
design cannot satisfy both. The instrument obeyed §10.1 and violated §5.

Recorded here as a defect of the pre-registration; a successor must resolve it
explicitly — for example by separating the guard/discard journal from the
observation artifact, so that a session which produces no observations still
leaves a complete record of why. **No such change is made here**, and no
threshold, bound or constant is altered by this document.

## 6. What this record does not do

It repairs nothing, re-runs nothing, proposes no seed numbers, and contains no
successor code. It does not lower or reinterpret the `0.09` bound. It does not
convert a Class I outcome into a Class II null. It writes no descendant, and §9
of the pre-registration remains blank and frozen.
