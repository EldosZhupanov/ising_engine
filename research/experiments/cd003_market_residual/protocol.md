---
id: cd003-market-residual-protocol
kind: research-protocol
status: binding
authority_scope: cd003-market-residual-structural-gate
created: 2026-09-25
immutable: true
---

# CD003-MR1: nonempty residual boundary-field compression

This is a deterministic structural feasibility gate, not an optimizer or
timing comparison. This protocol is committed before the probe implementation
and before any presolve/projection result is observed. Before freezing, the
authors inspected the corpus filenames, dimensions and two instance text
samples (`ms_03_050_002`, `ms_13_050_000`); no full presolve or projection
counts were run. Do not change the selection or thresholds after the run.

## Hypothesis and decision

H1: in at least **three of the ten** fixed official Market Split instances,
the production `full_presolve` leaves at least eight free variables and the
number of distinct boundary-induced internal field vectors is at most half
the raw `2^b` boundary assignments. This is only a GO for building a later
exact-elimination instrument, subject to independent energy equivalence and
matched-cost tests. Otherwise the decision is NO-GO for this corpus and split.
Any invalid row, checksum mismatch, or timeout makes the gate INCONCLUSIVE,
not a conveniently excluded failure. All ten rows must be valid for a GO or
NO-GO. No inferential p-value is appropriate for this fixed deterministic set.

## Inputs

Paths are relative to the repository root. Files are read in lexicographic
order. SHA-256 is part of the input identity:

| File under `benchmarks/qoblib/marketsplit/instances/` | SHA-256 |
|---|---|
| `ms_03_050_002.dat` | `63c20e7146a3807f935c4be2478c4028ddd0a9c190b6e485c5376375f0ae0d2e` |
| `ms_03_050_005.dat` | `eb466cc3d49a385bff22f9920a8fc108f9885ef503209e50e0509380ad472d54` |
| `ms_03_050_007.dat` | `ec24fe50b1bca3c170126ef807835b7b5090fc913dfc2f8d778c38216f26df5c` |
| `ms_03_050_009.dat` | `72bad55ffce20187ec1290abfafee407bb6f2733d8a7e1e54b1c7b429c5d930e` |
| `ms_03_100_001.dat` | `a003697b274970998f1f986c1d71ae339b489c19e5344875cc346ac13d383924` |
| `ms_03_100_012.dat` | `f8aee6493d86f8a37fc25b471755eeee5ae1ae65463bdc23e15d8c9bdb954f93` |
| `ms_04_050_001.dat` | `a6884b557d99f3292faeae2bcdf0073ebf6f9f707f30bae5d16845ee4b52e9cc` |
| `ms_04_050_003.dat` | `899fa884ceac5f233e6dfae49b33a2027db7cd478294453ac7515ac360392646` |
| `ms_04_050_004.dat` | `dc0289a1603d0ca0a714cbd76a87f06b14568330ffe5af1b988f907ddda7609f` |
| `ms_04_050_005.dat` | `cb6264c9e00135dae941558d9100117e856c7362047d1279d6b5b4963a168405` |

## Fixed procedure and baseline

For each instance, parse `m n`, exactly m coefficient rows of n integer
coefficients plus RHS, and form the same QUBO as
`src/bin/qoblib_marketsplit_benchmark.rs`: `E(x)=sum_k(A_k x-b_k)^2`.
Invoke the existing `ising_engine::presolve::full_presolve(model, &[])` once
per instance, single threaded, with a **60-second process timeout**.
Validate fixed indices and values, then define the sorted free-variable list.
The boundary B is the last `min(12, floor(free_count/2))` free variables;
internal I is every other free variable. There are no discretionary graph
partition choices. If fewer than eight free variables remain, record the row
as valid but H1-ineligible.

For each of the `2^b` boundary assignments z, calculate the integer
cross-field vector `f_i(z)=sum_(j in B) J_ij z_j`, for every i in I, where
`J_ij=2 sum_k A_ki A_kj`. Count distinct vectors exactly with integer
arithmetic. Equal field vectors induce identical conditional internal
objectives; compare `N_field` with the direct-context baseline `2^b`.
No response optimization, optimality claim, or runtime gain follows from this
count. The field fraction `N_field/2^b`, number of fixed/free variables,
wall-clock presolve time, and timeout state are recorded per instance.

## Integrity and independent checks

The runner checks every input hash before launching the compiled probe.
The probe records the fixed assignments but does not compute field counts.
The analyzer independently parses the original `.dat` bytes, computes J using
integers, checks the probe's dimensions/fixings, enumerates field vectors,
and writes a per-instance row and aggregate decision. A second implementation
or reviewer must check at least one complete row directly and verify all input
hashes and aggregate counts before accepting the result. An invalid fixing
claim on a small instance is checked by exhaustive energy minimization if
feasible; otherwise correctness is attributed only to the existing tested
presolve contract, with this limitation stated.

## Environment, seeds, artifacts

Host: WSL2 Linux x86_64, AMD Ryzen 7 170 with Radeon Graphics, four visible
CPUs, 8.7 GiB RAM; Rust 1.95.0. Build `cargo build --release --bin
exp_cd003_market_residual` without custom RUSTFLAGS. One process at a time;
set `RAYON_NUM_THREADS=1`. No RNG or seeds are involved. A 60-second timeout
per instance bounds this ten-instance diagnostic by approximately ten minutes
plus build/analysis overhead. Store raw JSONL, analyzer output, stdout/stderr,
source commit, binary hash, input hashes, environment and commands under
`results/run001/`; do not overwrite a completed run. Freeze the tested
instrument in a commit before the run. One run only.
