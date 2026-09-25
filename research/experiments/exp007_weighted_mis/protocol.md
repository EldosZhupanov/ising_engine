---
id: exp007w-protocol
kind: research-protocol
status: binding
authority_scope: EXP-007W weighted-MIS CD005 qualification
created: 2026-09-25
immutable: true
---

# EXP-007W: vertex-weighted MIS, CD005 at equal wall time

## Question and prior access

The earlier [CD005-Q2 result](../../cd005_equal_time/RESULT.md) proved that
the current strict two-flip finisher cannot improve a complete 1-opt minimum
for *unit vertex rewards* and edge penalty greater than one. This protocol
tests a different objective: positive, unequal vertex rewards. Before this
protocol, a two-vertex mathematical witness was computed: with weights `(1,2)`
and edge penalty `3`, `(1,0)` is 1-opt stable at energy `-1`, but the two-flip
to `(0,1)` has energy `-2`. This is known local-exchange territory, not a
novelty claim. An exploratory enumeration of 759 graphs with varied edge
penalties and unit vertex rewards found no improving pairs; none of the six
instances below has been run through a solver or local-search analysis.

The six synthetic instances were generated deterministically before this
protocol solely to freeze their bytes and metadata. Only their names, sizes,
edge counts and hashes were inspected. No outcome-based selection or tuning
was performed. The [generator](generate_instances.py) uses SHA-256 keyed draws
and the resulting [instance file](instances.json) has SHA-256
`3c901eaaa0098cc273427e6942227fb5c0c5010f459423e5686819d94878e520`.

| Graph | n | Edge probability | Edges | Instance SHA-256 |
|---|---:|---:|---:|---|
| `er_n64_p08_g0` | 64 | 8% | 158 | `5ead121b372e97a91f3b5582cde1747fea0fde12d0d947dc71bf72f67a44dfca` |
| `er_n64_p16_g1` | 64 | 16% | 325 | `6a2debb5eebbfb423b19c0af8fca0014784177f011755a9dd1f546a921053156` |
| `er_n96_p08_g2` | 96 | 8% | 392 | `5ffa8a662d8677254f6a8b093ac59080fcb5aa77718d74f9f0cbffc9e4ee1f5c` |
| `er_n96_p16_g3` | 96 | 16% | 709 | `0ed8ab261b1964ea2fa3ff90e23d3d53ddc78f24aa36f2ccc40e5a640fc4e791` |
| `er_n128_p08_g4` | 128 | 8% | 646 | `4a0961f05032de2d1142e65a36eb7edb684dd3f49b0a9d924d0e876d112e19ef` |
| `er_n128_p16_g5` | 128 | 16% | 1282 | `525317dc1a65753d66dd714056c7c823cd12034031ca704f5960c81aa168471c` |

## Model, hypothesis and arms

For each graph, integer vertex weights lie in `1..=10`. Build the same QUBO
for both arms:

`E(x) = -sum_i w_i*x_i + 11*sum_(i,j) in edges x_i*x_j`.

Store each undirected edge symmetrically in the repository `CsrMatrix`
convention. Since `11 > max(w_i)`, any complete 1-opt local minimum has no
selected edge; no repair is permitted. The baseline is existing
`UltimateSolver::new(2.5, 0.05, 10, 5, Some(seed)).with_2opt(false)`;
the candidate is identical except `.with_2opt(true)`. Each calls
`solve(model, &[])`; internal presolve and decomposition remain enabled.
No other solver options, graph preprocessing or warm starts may differ.

**Hypothesis H1 (mechanism):** At least one of the 60 specified post-presolve,
post-1-opt states below admits a strict improving edge two-flip. This shows
headroom in the tested residuals; failure is mechanism NO-GO on this corpus.

**Hypothesis H2 (practical):** The candidate obtains a higher best feasible
vertex-weight sum than baseline within equal wall time in a material number
of paired cells. The exact practical gate is below. H1 does not imply H2.

## Structural probe

For graph index `g=0..5`, use 10 independently seeded starts
`9_000_000 + 1_000*g + s`, `s=0..9`. Run `full_presolve(model, &[])` once per
graph; initialize free bits from `ChaCha8Rng(seed)` and set fixed bits to their
derived values. Apply the existing `steepest_descent_1opt` to completion over
free variables. Independently recompute energy and every free single-flip
delta; any negative delta below `-1e-9` invalidates the instrument. Scan free
edge pairs with direct graph energy differences and record whether any has
delta below `-1e-9`, plus the best pair/delta and state. Store all 60 rows,
including zero-headroom rows. If H1 fails, the timed campaign still runs;
there is no posthoc instance exclusion.

## Equal-time campaign

For each graph `g` and campaign `c=0..9`, both arms use the solve seed stream
`7_000_000 + 100_000*g + 10_000*c + k`, `k=0,1,...`. Each arm gets 2.000
seconds from its first solve start. Start another solve only while time
remains. Count its solution only when it completes by the deadline; record a
late completion separately. Candidate runs first iff `(g+c)%2 == 1`.
Campaigns run sequentially on the same host; no concurrent load is introduced
by this experiment. No retries or exclusions are allowed. If an arm has no
counted solution, the cell is invalid and H2 is NO-GO. Save every counted
bitstring, seed, model-computed energy, duration, and arm/cell timing.

The independent analyzer reconstructs graph weights and edges from the frozen
instance file and checks binary state, zero selected edges, vertex-weight sum,
and direct energy `-weight_sum` within `1e-9` for every counted solution.
Invalid solutions invalidate that cell and the H2 practical gate. Analyzer
must not trust solver self-reported feasibility or energy.

## Outcomes and decision rules

**Primary:** across all 60 graph-campaign pairs, compute
`best_candidate_feasible_weight - best_baseline_feasible_weight` from
within-deadline solutions. Report wins/ties/losses and per-graph differences.
The scoped practical **GO** requires all 60 cells valid, at least 10 candidate
wins across at least three graph names, at most three losses, and a one-sided
exact paired sign-test `p < 0.05` after dropping ties. Otherwise H2 is
**NO-GO for demonstrated equal-time benefit in this pilot**. A tie is not
evidence of equivalence; this gate does not make a SOTA or general MWIS claim.
There is one primary hypothesis test and no adaptive threshold choice.

Secondary: structural H1 hit count; presolve fixed counts; complete and late
solves; same-seed differences where both solves finish; actual arm elapsed
time. Do not report a percentage change in signed QUBO energy. No graph,
seed, statistic or solver setting may be selected from observed outcomes.

## Environment, freeze order and stopping

Planned host: Linux `6.6.114.1-microsoft-standard-WSL2`, x86-64;
AMD Ryzen 7 170 with Radeon Graphics, four visible logical CPUs,
9,373,351,936 reported RAM bytes; `rustc 1.95.0 (59807616e 2026-04-14)`;
release build, default `RUSTFLAGS`, one sequential experiment process.
Record exact runtime versions, CPU, RAM, `RUSTFLAGS`, thread settings, Git
commit and dirty status in `metadata.json` alongside raw output. Budget is
60 cells × 2 arms × 2 seconds, about four minutes plus late completions,
build/analysis time excluded.

Freeze this protocol and inputs in Git before instrument implementation or
solver access to the six inputs. Calibrate only on a separate, coded two-vertex or small synthetic
instance. Freeze the instrument and analyzer in a second commit before any
structural or timed holdout run. Run the 60 structural probes and 60 timed
cells once. Save raw data, `metadata.json`, `README.md` replay instructions,
analysis and SHA-256 hashes in a third commit. A required change after data
access must be disclosed in a prospective amendment and newly labeled
evaluation; never edit this protocol or overwrite raw results.
