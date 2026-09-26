# LABS-Q002 — later memetic hunter qualification

Status: binding local preregistration, 2026-09-26. Freeze this document before
instrumentation or qualification data. This is a new experiment; LABS-Q001 and
its source/records are unchanged.

## Question, hypothesis and prior access

Does the later `labs_record_hunter` reliably recover known ordinary LABS optima
at a ten-second single-worker budget, and how does its quality compare with the
retained specialist lMAts at the same deadline?

Primary directional hypothesis: the memetic hunter reaches the known optimum
in at least 8 of 10 runs at **each** N=40,50,60, with zero invalid witness or
budget-enforcement failures. This is an operational qualification screen, not
a confidence statement that its population success probability is >=0.8.
An integrity-valid campaign failing the hit threshold at any length means NOT
QUALIFIED; no larger record run follows from this experiment automatically.

Prior access: LABS-Q001's earlier PT lost to lMAts at N50/60; later N67..74
checkpoint energies, including N74=357, are known. The new qualification seeds
have not been evaluated in this cycle. The present hunter's checkpoint race and
CLI/file publication defects were fixed at `66e9934`. Its search efficacy
remains unknown. No production UltimateSolver or engine_v2 claim is tested.

## Frozen cases, seeds, arms and budget

- Objective: ordinary aperiodic LABS, E=sum_d(sum_i s_i*s_(i+d))^2, spins +/-1.
  N=40,50,60; targets 108,153,218. No input solution, warm start or skew-symmetry
  restriction. `cases.json` SHA-256:
  `dfa43a17e8e6966faac1572e95cb758d54068c75005e45536637d6c26bfb1b1a`.
- Both arms use seed labels 740001,740002,740003,740004,740005,740006,740007,
  740008,740009,740010. Labels block runs; they do not imply identical random
  streams between C and Rust. 30 runs per arm, 60 cells total.
- Candidate: retained memetic hunter, 20 replicas, geometric temperatures
  0.12..35, 25 sweeps per exchange, existing tabu/crossover/shake schedules and
  prefix-biased initialization. No parameter tuning or algorithm changes.
  Run trajectory thread ID 0 directly, with the exact run seed and no warm
  start. Add observation-only `--qualify N SEED`, emitting flushed `INC E BITS`
  lines, skipping checkpoint I/O and internal record stopping. Report the best
  initial replica without inserting it into collective memory; subsequent
  improvements update collective memory as in normal mode. Fixed-step tests
  must prove observation does not change the search result/state.
- Baseline: lMAts ordinary-sequence memetic tabu from `borkob/git_labs`, commit
  `1aa123407636e48630174e0bf122037e369c1d68`; retained source archive SHA-256
  `0a154a91ef32c050c50edf8babee8cb07278c88c3c2fb8499b35446f9a37bf5a`,
  logging-only patch SHA-256
  `69659afc1f1aa606879ff7351d832f720ad80055fbd80ec125e4ff82a5e10d06`.
  Defaults: population 100, crossover .9, mutation 1/N, tournament 2. One worker,
  internal target 0 and 3600-second limit; no new external search modifications.
- External wall budget: 10.0 seconds, starting before process creation,
  including initialization and flushed logging. Receipt of a complete valid
  sequence line determines its eligibility; no output after the deadline or
  stop is credited. Terminate process group at deadline or verified optimum.
  Termination tolerance 0.1s is not extra eligibility time. Do not rerun failures.
- Execute N-major then seed-major, alternating the first arm by parity of
  N-index+seed-index. Runs are sequential, never simultaneous. Complete all
  60 cells even if qualification fails early. No budget extension or tuning.

## Environment and pre-data gates

Host observed at preregistration: Linux 6.6.114.1-microsoft-standard-WSL2,
AMD Ryzen 7 170 exposed as 4 logical CPUs (2 cores), 9,153,664 kB RAM;
rustc 1.95.0 (59807616e 2026-04-14). Release build uses retained `.cargo/config.toml`
native CPU, +avx2,+fma, opt-level 3. No affinity. Set RAYON_NUM_THREADS,
OMP_NUM_THREADS, OPENBLAS_NUM_THREADS and MKL_NUM_THREADS to 1. Capture actual
CPU/OS/compiler, config/env flags, binary hashes and load before each run.
Do not infer hardware-independent speed from this virtualized host.

Read-only dependency map: all hunter declarations/call sites are in
`src/bin/labs_record_hunter.rs`; no external HuntContext/hunt_trajectory callers
were found. `research/labs_qualification/campaign.py` supplies the retained
supervisor, integer energy oracle, witness parser and checker invocation
(SHA-256 `d36cbd1f1c462255f7405fa43da0a49488f6c1ff8ac5f9d8f20e726656dfa785`).
Reuse these functions without editing the old file. Build reference through
its retained `build_reference.py`; QOBLIB checker remains a separate process.
Do not touch solver families, core/compiler, dependencies, Laya or old protocols.

Before qualification: exhaustive N<=10 flip/delta consistency tests; fixed-seed
multi-flip and local-search invariants; fixed-step observer on/off equivalence;
retained supervisor late/invalid-line tests plus new complete-design/statistics
tests; external logging neutrality check or verify its retained evidence hashes;
all root source gates; independent review. Smoke only N20 seeds 820001,820002,
<=1 second per arm, excluded from qualification and retained separately.
If a mathematical defect requires an algorithm change, stop data execution and
write a prospective amendment/new version; do not quietly change the candidate.

## Endpoints and falsification

Primary: per-length optimum hits/10 and Wilson 95% intervals; qualify only at
>=8/10 for all three lengths with no integrity failures. Report all final
energies, mean, sample SD, median, min/max, linear-interpolated quartiles/IQR,
relative gaps and paired candidate win/tie/loss counts. Missing/invalid cells
remain in the denominator; never discard them. Any witness, checker, launch,
completeness or budget-integrity failure in either arm makes the entire campaign
INSTRUMENT_INVALID: retain descriptive observations but issue no qualification
or superiority verdict. Finish remaining planned cells without reruns.

Secondary descriptive TTS: report observed hit times and explicit right
censoring at 10s. The fixed-budget restart estimate is
10*max(1,ceil(log(.01)/log(1-p_hat))) seconds for 0<p_hat<1; p_hat=1 gives 10s,
p_hat=0 gives undefined/infinite (JSON null). This plug-in estimate is not a
guarantee of 99% success and not an estimate of censored mean runtime.

Secondary paired quality test: exact two-sided Wilcoxon signed-rank by
enumerating sign assignments to nonzero paired energy differences; average
ranks for tied magnitudes, zeros omitted. Its null is a difference distribution
symmetric about zero with independently exchangeable signs conditional on
absolute ranks; do not treat it as an assumption-free median test. Holm
correction across three lengths.
Only describe candidate superiority on a length if all pairs are valid,
corrected p<.05 and median paired relative gain (E_lmats-E_candidate)/E_lmats
is at least .05. Any such statement concerns these compiled implementations,
settings and this host only. Otherwise use descriptive outcomes, not superiority language.
No universal/SOTA/quantum/world-record claim is licensed.

Attempt falsification by independent Python energy for every emitted incumbent,
the QOBLIB checker for every final saved witness, rejection of below-optimum
energies, replay of analysis, missing-cell and invalid-checker injection tests.
Checker exit 20 at these known-optimum sizes may indicate a valid suboptimal
sequence: require the printed energy to agree as well as an allowed exit 0/20.

## Freeze and evidence

Commit protocol before instrumentation; commit reviewed source/tests/runner
before any qualification seed. Runner requires clean tracked tree and a new
output directory, pins source/binary hashes, and preserves every stdout/stderr,
timestamped sequence, return code, checker output and failure. Save all 60 cells
once; commit raw evidence separately from interpretation. A flawed instrument
or interrupted execution stays as evidence; use a new prospective iteration,
never overwrite or silently resume the confirmatory campaign. Publish result,
limitations and next decision in this experiment directory and update NOW,
ROADMAP, RESEARCH and the catalogue.
