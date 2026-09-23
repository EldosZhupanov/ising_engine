# LABS-Q001 qualification protocol

Frozen before implementation and qualification data; local preregistration.
Date: 2026-09-23. User authorized N=40,50,60 qualification and a specialist comparison.

## Scope and prior access

Qualify the standalone LABS binary, not UltimateSolver or engine_v2 selection.
The existing external-comparison protocol governs selector waves / BiqMac; this
separate LABS experiment makes no selector or production superiority claim.
Prior exploratory outcomes known: N20=26, N30=59, N40=124. No new qualification
seed outcomes inspected. The prototype's replica-swap acceptance sign is reversed:
for beta_cold > beta_hot and E_cold < E_hot, acceptance must be less than one.
Correct that bug before freezing code; the evaluated algorithm is the corrected
prototype, not an exact rerun of the historical binary.

## Fixed design

- Ordinary aperiodic LABS: E=sum_{d=1}^{N-1}(sum_i s_i*s_{i+d})^2, s in {-1,+1}.
- N=40,50,60; verified QOBLIB optima 108,153,218, respectively.
- Ten independent run seeds per N: 730001..730010 for both arms. Same seed labels
  are blocking labels, not identical random streams across implementations.
- Arms: corrected existing PT and published lMAts memetic tabu search from
  borkob/git_labs commit 1aa123407636e48630174e0bf122037e369c1d68,
  solvers/lMAts-lRRts. Original default population=100, crossover=.9,
  mutation=1/N, tournament=2; one worker. No skew-symmetry restriction.
- PT: 10 replicas, temperatures .2..20 geometric, sweep/exchange parameters
  (35,30),(45,35),(55,40) for N40,50,60. Repeat these existing restart units
  sequentially, seed = run seed + restart_index*104729. No tuning.
- Both arms: one search worker, common external 10.0 second wall budget,
  measured before process creation, including initialization and logging.
  No CPU affinity. Sequential runs, N-major then seed-major, alternate first arm
  by (N_index+seed_index) parity. No simultaneous solver runs by this experiment.
- Both publish every incumbent improvement with full sequence; supervisor timestamps
  complete received lines. Only valid witnesses received by the common deadline
  count. Processes are terminated at deadline or after a verified optimum.
  Record drain-after-deadline output but never count it. Use process groups to
  clean up workers. Budget enforcement tolerance 0.1s for termination only;
  no late witness enters the endpoint. Record failures, never silently rerun.
- lMAts instrumentation only adds incumbent sequence logging and uses target=0
  to prevent internal target stopping. External supervisor gives both equal
  optimum-stop access. Original solver math and search defaults stay intact.
- Wall budget equality is the comparison object; evaluation counts are not equal
  because kernels differ. Report quality at this budget, not pure-kernel speed,
  TTS speedup, hardware-independent superiority, or contemporary SOTA dominance.

## Verification and gates

Before data: exhaustive N<=10 flip/delta tests; small seeded repeated flip tests;
explicit exchange sign regression; observer vs ordinary solver fixed-seed equality;
strict independent Python sequence validation and direct integer energy;
compile and run official QOBLIB checker on saved witnesses; invalid/tampered
sequence tests; supervisor late-line/failure tests. Smoke N20 seeds 810001,810002,
<=1 second each (excluded). Full root source gates and independent review.
No new heavy dependencies. Preserve unrelated work.

## Metrics, decision, stopping

For every arm/N: optimum hits /10 with Wilson 95% interval; all ten energies,
median and range, relative energy gaps; paired win/tie/loss counts by seed.
Anytime incumbent histories and time-to-first-optimum (censored at10s) are
observational only, not a speed superiority claim. No significance testing.
Qualification requires >=8/10 hits at EACH of N40,N50,N60 and zero witness or
budget-validity failures. This is an operational screen, not proof p>=.8.
All 60 cells run once, even if an early length fails. No budget extension,
parameter tuning, seed replacement, or record attempt after seeing outcomes.
Missing/invalid rows fail qualification, are counted separately, and never excluded
from the denominator. Energy below a proven optimum is an integrity failure.
No restart of confirmatory data after source changes: preserve evidence and
write a prospective amendment/new iteration if needed.

## Evidence and freeze

Freeze source, external archive/patch hash, runner, analysis and tests in Git
before executing. Record evaluated commit, binary hashes, versions/CPU/environment,
commands and process results. Save stdout/stderr, every accepted sequence,
independent energy and QOBLIB checker output. Commit raw evidence separately;
then publish all outcomes, limitations, NOW/catalogue update and handoff.
