# HUBO-C001 — native representation helps, solver advantage unqualified

2026-09-27. Closed research record on commit. Protocol `feb9a76`, evaluated
instrument/analysis `23d300f545ce027e7ad56eac7e895e920b804712`, raw results
`a196745`. No production solver changes or campaign reruns.

**Registered decision: NOT_QUALIFIED_FOR_ADVANTAGE.** All 400 cells completed.
MSC native beat both quadratic arms in every paired cell, but tied OpenJij native
in all 100 pairs. The native-versus-native contrast is nondiscriminating at this
budget; this is neither proof of equivalence nor a global optimum certificate.

## Evidence and estimand

[Protocol](protocol.md), [raw manifest](raw_sha256.json),
[environment](run/environment.json), [frozen analysis output](run/summary.json),
[instrument checks](instrument_checks.json). Ten synthetic n=32 signed spin
polynomials (five cubic, five quartic), ten solver seeds, four arms, two-second
warm-runtime budgets. Parsing, reduction, initialization, search and reporting
are charged; Python/library import is excluded and recorded. Both native arms
returned the same energy on every seed of each instance. Optima are unknown.

Each final witness is a `.sol` beside its complete `.json` event stream.
All 3,487 main-run incumbents were checked in two original-objective forms by
the recorder; no invalid cell, discarded run or replacement seed exists.
Smoke used a separate n=6 case and two seeds: 8/8 valid, 25 incumbents, all final
energies -4. [Independent audit](audit/result.json): **PASS**, recomputed all 3,487 main
incumbents, all 400 final witnesses and every summary field without importing
the campaign/worker implementation. It also checked consistent auxiliary
extensions, 202 source hashes, 554 package files and both executable hashes per
manifest. Local file-write order matched the balanced schedule; mtimes are
mutable local evidence and are not preserved by Git.

| Arm | Mean original energy, degree 3 | Mean original energy, degree 4 | All cells mean |
|---|---:|---:|---:|
| MSC native | -66.40 | -69.20 | -67.80 |
| MSC quadratic | -51.44 | -50.44 | -50.94 |
| OpenJij native | -66.40 | -69.20 | -67.80 |
| OpenJij quadratic | -42.36 | -30.56 | -36.46 |

Lower energy is better. These are descriptive means across heterogeneous cases;
inferential comparisons use ten instance-level averages, not 100 independent seeds.

| MSC-native comparator | Wins / ties / losses | Median normalized instance gain | Exact p | Holm p |
|---|---:|---:|---:|---:|
| MSC quadratic | 100 / 0 / 0 | 0.13203125 | 0.001953125 | 0.005859375 |
| OpenJij native | 0 / 100 / 0 | 0 | 1 | 1 |
| OpenJij quadratic | 100 / 0 / 0 | 0.24453125 | 0.001953125 | 0.005859375 |

Gain means `(E_other-E_MSC_native)/128`, averaged over seeds per instance.
It is not percentage speedup or a relative optimality gap. Both quadratic
contrasts pass the registered effect/statistical/family thresholds; the native
external contrast fails. The conjunction therefore fails. Per-instance arrays,
quartiles, standard deviations and degree-family statistics remain in summary.json.

## What this establishes and does not establish

- **SUPPORTED:** direct MSC HUBO has better two-second endpoint quality than
  the registered dimod quadratic reduction under the same MSC driver on this
  corpus. The same endpoint ordering also appears in the external arms.
- **INCONCLUSIVE:** a competitive advantage over native OpenJij. All native
  paired differences are zero; additional seeds on these opened cases do not
  create evidence of a distinct solver advantage.
- **HYPOTHESIS:** auxiliary variables and the conservative penalty/schedule
  combination explain much of the quadratic deficit. This causal explanation
  has not been separated by an ablation.

Quadratization expanded 32 variables to 92–154 (mean 123.1); the global penalty
was 769–4,497. Mean transformation time was about 11.2 ms, versus about 0.13 ms
for native preparation. The representation-derived hot-temperature ranges were
291–1,492 native and 23,382–234,008 quadratic. These simultaneous changes prevent
attributing the effect to variable count, kernel speed or penalties alone.
They do not discredit tighter reductions or better-tuned quadratic solvers.

Maximum termination time was 2.029968 s; witnesses received after 2.0 s were not
credited. Average excluded startup was about 0.67 s for every arm. The host was
AMD Ryzen 7 170 / WSL2, with no affinity pinning; this is exploratory warm-runtime
quality, not cold-service latency, throughput or a universal benchmark.

No SOTA, quantum advantage, novel native-HUBO method, TTS99, optimum hit rate,
world-record or production-UltimateSolver claim is licensed. The driver uses
the existing MSC kernel, not the complete public UltimateSolver pipeline.

## Verification and reproduction

Pre-data independent review passed; Python 14/14 and Rust example 2/2 tests
passed, as did example Clippy/build, fmt and memory/diff checks. Tests exercise
integer objective equivalence, exhaustive small reduction, incremental engine
energies, fixed-seed replay, seed/order balance, malformed/late events, process
cleanup and instance-level statistics. Review defects were repaired before the
source freeze. Production `src/`, dependencies and both solver families stayed
unchanged; the full production Cargo suite was not rerun for this isolated adapter.

Replay the preserved analysis without launching solvers:

```bash
benchmark-env/bin/python3 research/experiments/hubo_comparison/campaign.py analyze research/experiments/hubo_comparison/run
```

Independent audit command on the recorded host:

```bash
PYTHONHASHSEED=0 benchmark-env/bin/python3 research/experiments/hubo_comparison/audit/independent_audit.py
```

The frozen analyzer validates archived absolute worker/engine paths. The complete
independent audit also requires recorded installed runtime paths and local raw
file mtimes. A relocated clean clone can inspect/hash witnesses and source, but
these full local audit commands are not promised to pass unchanged there. No
raw paths or timestamps should be rewritten to conceal this portability limit.
For a fresh computational reproduction, check out `23d300f`, recreate the recorded
Python/package environment, build with
`cargo build --release -p research --example hubo_compare`, then invoke
`campaign.py smoke NEW_EMPTY_SMOKE_DIR` and `campaign.py run NEW_EMPTY_RUN_DIR`.
These are new wall-clock samples, not bit-identical reproductions of endpoint
timing, and must not replace this frozen campaign.

## Disposition

Keep the correctness gate and scoped representation result. Do not launch a
record hunt, integrate a production route or tune on these ten exposed cases.
Next design question: can a separately preregistered, independent harder corpus
distinguish the two native solvers while controlling quadratic penalty/schedule
quality? Qualify that measurement before adding operators. No follow-up campaign
or Laya training was run in this cycle.
