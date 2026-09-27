# ER-001 — real-data entity-reconciliation capability pilot

Local preregistration, 2026-09-27. Research-only; becomes immutable before the
first smoke/main solver outcomes. This is a development pilot, not a held-out
WDC score, a trained matcher evaluation, or a speed/superiority claim.

## Question and scope

Can one label-blind pipeline construct complete scored blocks of real product
records, find and verify consistent partitions, and exercise the existing native
HUBO kernel? A possible later application is semantic scores from Laya followed
by constrained decisions. This pilot freezes a cheap lexical scorer first, because
no working torch/transformers/Laya environment is installed. No weights/training,
heavy dependencies, public API or production solver changes are authorized here.

Directional screening hypotheses, defined before the first scored outcomes:

- H1: at least 3/12 selected blocks have inconsistent independent pair decisions.
  Otherwise this input fails qualification for testing reconciliation.
- H2: exact reconciliation improves pooled positive-pair F1 by at least 0.02
  over the BEST of pairwise, closure and greedy. H1 must also pass. This is a
  descriptive development signal only, not statistical confirmation or superiority.
- H3: at least 95% of 120 native cells attain the independently enumerated optimal
  partition objective, with every endpoint feasible and every returned incumbent's
  energy checked. Passing establishes scoped native fidelity, not algorithmic value.

Any invalid energy, malformed output, missing cell, clock failure or source drift
makes the instrument invalid: retain raw evidence, stop; no silent rerun. Correct
prospectively with a new iteration/amendment. A negative H1/H2 is retained without
retuning the threshold, replacing blocks, or training Laya on the same outcome.

## Input, access and selection

Source: [WDC Products](https://webdatacommons.org/largescaleproductcorpus/wdc-products/),
official `80multi.zip` (2,448,480 bytes), SHA256
`5385e2491e82c7f6c208d8eac7ec777a4a44180769f3a339c99803ff2b068981`.
Member `wdcproductsmulti80cc20rnd000un_train_large.json.gz`, SHA256
`152ee17e30c2488bb6b87bd3e9e6fdd4e3f47a5f1181449f47c0c60ff5d24dde`.
There are 2,841 records for 500 benchmark product clusters, each size 3..11.
Source ID and cluster are integer fields; `label` equals `cluster_id`.

Intake opened training-small schema/one title and cluster-size counts (all pairs),
then training-large schema and cluster-size counts, to establish task suitability.
The archive was downloaded, but validation and official test members were NOT
decompressed/read. This is prospective relative to pilot metrics, not a claim
that the development data is an untouched holdout. No learned parameters or
source/entity transfer claims. Source URLs are absent here, so source-disjointness
cannot be verified. Benchmark labels originate from identifier-based clustering;
they are reference annotations, not infallible semantic truth.

`prepare.py` strips all columns except ID/title BEFORE blocking/scoring. IDs serve
only deterministic ordering and bookkeeping. Unicode casefold/alphanumeric tokens;
title token-set Jaccard J. Score `round_half_up(200*J)-100`; empty-title pair is -100.
These signed integer utilities are not model probabilities. No threshold fitting.

Disjoint blocks: in lexicographic ID order, take the first unused anchor and up
to five unused records with J>=0.1 to the anchor, ordered by decreasing J then ID.
All rows are assigned; singleton/pair blocks are counted but not optimized in this
pilot. Select the FIRST twelve blocks of size >=3, without looking at labels,
conflicts or outcomes. All selected blocks have six records. Every within-block
edge is explicitly scored, including edges below blocking threshold. Cross-block
pairs are unscored/unresolved, never asserted to be nonmatches. Therefore the
reported F1 is restricted to selected within-block pairs, not whole-corpus F1.

Frozen IDs, scores, evaluation labels and coverage denominators: [cases.json](cases.json),
SHA256 `3214b795b55805fe5314830004e0b3a31bb50e21b671a1b5fe503d93e7bfe271`.
Reconstruct from the public archive and hashes; no raw text redistributed here.
The upstream code repository has BSD-3-Clause licensing; no separate dataset
license was located on the benchmark page. Public download is not treated as a
blanket redistribution grant. Keep the archive in local cache, retain only the
small derived input manifest in Git.

## Objective and invariants

For six records let x_ij=1 mean the same predicted entity. Maximize sum w_ij*x_ij
over partitions. Every triangle has penalty `ab+ac+bc-3abc`, equal to one exactly
when two of three links are selected. Complete-graph absence of these triangles
is equivalent to an equivalence relation (reflexivity implicit, symmetry built in).

Minimize `-sum(w*x)+M*sum(triangle penalties)`, M=1+sum(abs(w)). All-zero links
are feasible with energy0. Any infeasible state has energy at least
M-sum(max(w,0))>0, hence every global minimizer is feasible. The exact oracle
enumerates 203 set partitions directly, independently of penalty minimization.
Tie-breaking: lexicographically lowest edge bitstring among optimal partitions.
Different optimal partitions can have different annotation quality; native ties
are retained, not chosen using truth.

Both submitted binary and spin forms equal eight times the application objective;
this clears denominators in x=(s+1)/2. No discarded constants. Application energy,
polynomial energy and spin energy must match exactly. Independent verification
uses the original edge list and triangle counts for every native incumbent,
including late events. Truth is evaluation-only and is never passed to Rust.

## Arms, resources and seeds

Same frozen scores for all arms:
1. Independent pairwise w>0, deliberately allowed inconsistent.
2. Connected-component closure of that threshold graph.
3. Deterministic greedy merging by maximum positive intercluster score gain.
4. Exact set-partition enumeration: strongest objective control on this small size.
5. Existing native MSC `research/examples/hubo_compare.rs`, unchanged, with the
   audited Q002 nonblocking supervisor. This is NOT public UltimateSolver.

No new ILP dependency: exhaustive partitions prove the optimum in this size range.
No QPBO/presolve claim: this pilot exercises native cubic search directly. The
Laya binary bridge's 16-variable cap cannot accommodate general quadratic lifts
of six-record blocks. Later presolve/ILP/scorer comparisons require a new protocol.

Native seeds 990001..990010 (ten per block, 120 cells); synthetic smoke seed991001
on the separate three-record scores [2,2,-3]. Blocks run in manifest order, seeds
in increasing order. Same seed across blocks is pairing, not extra independence;
seeds within a block measure algorithmic variability, not new semantic examples.
No p-values, confidence claims or TTS extrapolation from these selected blocks.

Each native cell: two seconds after worker READY, including request transfer,
temperature calculation, child launch and search. Model construction is measured
separately and shared with all native seeds; do not call this an end-to-end timing
comparison. Blocking/intake observation about four seconds is not a calibrated
benchmark. Process-start overhead, construction and termination are retained;
termination tolerance .2s, READY deadline30s. Late witnesses are verified but not
credited. A feasible all-zero fallback is emitted and explicitly counted; it is
not evidence that the native kernel solved a case. Deterministic controls run once
per block with observed times; no runtime-ranking claim across these wrappers.

One sequential worker, Rayon/OMP/OpenBLAS/MKL threads1, no core pinning. Host:
AMD Ryzen7 170 with Radeon Graphics; Python3.14.4; rustc1.95.0; release example,
existing `.cargo/config.toml` target-cpu=native. Record exact OS, source hashes,
binary hash, inherited RUSTFLAGS, clocks and CPU in environment.json. Planned
search budget240seconds plus two-second smoke and startup/validation overhead.

## Analysis and evidence

Record every raw event, failed attempt and endpoint before validating. Persist
baseline witnesses/times, submitted model/construction time and source metadata.
Primary gates H1/H2/H3 above; separately report pooled TP/FP/FN/TN, precision,
recall,F1 for each deterministic arm, native F1 by seed, optimal-hit fraction,
objective gap mean/median/population SD/min/max/quartiles and fallback count.
Report selected-record coverage and source-positive-pair coverage; do not treat
omitted pairs as correct negatives. Preserve conflicting cases even if all
methods fail. No abstention-based accuracy improvement claim.

First freeze protocol, inputs, code and tests in Git; smoke first, then run the
unchanged main pilot if it passes. Result commit is separate. Python standard
library only; exact tests cover penalty equivalence, independent optimum, label
blindness, missing/invalid witnesses and retained corruption. Independent reviewer
must inspect finished artifacts and reproduce the analysis. No automatic external
submission, training or scaling campaign follows these development outcomes.
