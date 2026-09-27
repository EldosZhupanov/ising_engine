# ER-001 — real-data reconciliation works; added search benefit not established

Date 2026-09-27. Protocol/instrument freeze `96b0c23`; raw results `a91481c`.
Status: **REPRODUCED capability; H1 PASS, H2 FAIL, H3 PASS**. Development-only,
not an official WDC test score or a novel matching/search algorithm.

## What was built

One canonical research implementation, [core.py](core.py), accepts complete
integer edge scores for up to eight records and returns the exact optimal
partition, selected links, independently recomputed objective and violations.
Unknown/missing edges cannot silently become negative labels. Its JSON boundary
accepts only record IDs and scores, refusing truth labels and duplicate IDs.

The pilot connects label-blind WDC blocking/scoring, simple controls, exact
partition enumeration and the existing native MSC driver. There are no changes
to either production solver family, public APIs, Cargo dependencies or weights.
This uses native cubic HUBO at the research kernel, not UltimateSolver's public
QUBO entry point. Laya was not run: its prior Python environment is absent.

## Results fixed by the protocol

Twelve blocks, six records each; 72 distinct records and 180 within-block pairs.
Ten native seeds per block at two seconds each: **120/120 valid cells**.
All **1,344 incumbents** were independently checked, including late arrivals;
only eligible on-time witnesses count. No invalid run or silent retry occurred.

| Gate | Registered criterion | Observed | Verdict |
|---|---|---|---|
| H1 informative conflicts | >=3/12 inconsistent threshold blocks | 9/12 | PASS |
| H2 semantic development signal | Exact F1 >= best simple control +0.02, and H1 | Exact and greedy identical; difference0 | FAIL |
| H3 native fidelity | Native optimal witness in >=95% of cells | 120/120, endpoint objective gaps all0 | PASS |

| Method | TP | FP | FN | TN | Positive-pair F1 |
|---|---:|---:|---:|---:|---:|
| Independent pair threshold | 65 | 40 | 22 | 53 | 0.677083 |
| Transitive closure | 75 | 59 | 12 | 34 | 0.678733 |
| Greedy objective-based merging | 73 | 47 | 14 | 46 | 0.705314 |
| Exact partition enumeration | 73 | 47 | 14 | 46 | 0.705314 |

Greedy attains the exact objective optimum and the same chosen partition on
**all twelve blocks**. Exact improves over closure by 0.026581 F1 on these pairs,
but the registered comparison was against the BEST simple method, not closure
alone. H2 therefore fails; a favorable subset/contrast cannot replace it.

Native-wrapper F1 averaged 0.709494 across seeds (range0.704762–0.717703).
Different optimal partitions explain that variation; no annotation-based tie
selection was performed. It is not a registered superiority result. Five selected
endpoints retain the feasible fallback because of tied energies; each of those
cells separately contains a native on-time optimal witness. Fallback-only output
was not credited as native success. All gap distribution statistics are zero.

Observed deterministic-control totals across twelve blocks: greedy0.000722s,
exact0.017993s. These are wrapper observations, not calibrated speed ratios.
The native search consumed its fixed240s budget plus startup/checking; there is
no evidence this extra search is useful on these small blocks.

## Scope and limitations

Source: pinned [WDC Products](https://webdatacommons.org/largescaleproductcorpus/wdc-products/)
training-large archive member, 2,841 offers/500 product clusters. Official
validation/test members were downloaded inside the archive but never decompressed
or read. Selected records are 72/2,841; selected within-block positive pairs are
87/8,471 source-positive pairs. **The reported F1 covers only these 180 pairs.**
Cross-block and unselected pairs are unresolved, not credited as true negatives.
No source-disjointness or generalization claim: source URLs are absent here.

Scores are fixed title-token Jaccard utilities, not probabilities, a trained
matcher or Laya output. Labels are the benchmark's identifier-derived reference
clusters, not infallible truth. Twelve blocks are not twelve independent global
worlds, and repeated solver seeds do not create new semantic test examples.
No statistical superiority, world record, learned mechanism, semantic correctness
certificate, presolve benefit or production scalability follows.

## Verification and reproduction

- Python invariant/instrument tests: **14/14 PASS**. Includes all Boolean states
  through five records, all set partitions through eight, truth exclusion, seed
  duplication, clocks, corruption, exact F1 boundary and fallback-only rejection.
- Unchanged Rust example tests: **2/2 PASS**; release build and targeted Clippy
  with `-D warnings` PASS. No production Rust change requiring a full cargo suite.
- Synthetic smoke: separate seed991001, one valid cell/five checked incumbents.
- Independent read-only audit: **PASS**, recomputed without `pilot.analyze`;
  147 main raw hashes, 203 source/Git hashes, binary identity, all120 cells,
  1,344 incumbents, baseline partitions, confusion counts, F1 and gate decisions.
  The smoke archive's six raw hashes and source/binary checks also pass.
- Frozen analysis replay equals [main summary](run/main/summary.json) byte-for-byte.

From repository root, without running new search:

```bash
python3 -m unittest discover -s research/experiments/entity_resolution -p 'test_*.py'
python3 research/experiments/entity_resolution/pilot.py analyze \
  --output research/experiments/entity_resolution/run/main
python3 research/experiments/entity_resolution/independent_audit.py main
python3 research/experiments/entity_resolution/independent_audit.py smoke
```

To reconstruct the input, use `prepare.py --download --archive /a/cache/80multi.zip
--output /a/new/cases.json`; it pins archive/member hashes and refuses overwrite.
The reconstructed input SHA is
`3214b795b55805fe5314830004e0b3a31bb50e21b671a1b5fe503d93e7bfe271`.
Source hashes, exact environment and seeds are in [environment](run/main/environment.json);
raw identities in [manifest](run/main/raw_manifest.json). The archive itself is
kept outside Git; see the protocol's access/license disclosure.
The independent checker's [saved report](independent_audit.json) includes its
source SHA and commands. Its strict runtime check also requires the recorded
binary path/hash; the pilot analysis itself can replay without launching a binary.

Small reusable API example (scores ordered AB, AC, BC):

```bash
python3 research/experiments/entity_resolution/core.py <<'EOF'
{"record_ids":["A","B","C"],"weights":[2,2,-3]}
EOF
```

It returns clusters `[A,C]`, `[B]`, score2 and zero violations. The alternate
partition `[A,B]`, `[C]` has the same optimum; the output is not a truth certificate.

## Decision and next task

Keep the verified reconciliation core and simple/exact controls. **Do not scale
the native search or claim solver advantage from ER-001.** The next design task
is to qualify the scorer and blocking coverage on a newly frozen entity-disjoint
split; compare lexical and model scores with the SAME decoder, retaining greedy
and exact controls. Laya is a candidate scorer, not an assumed improvement.
Its runtime must be restored in a separate scoped environment task. No automatic
training or new evaluation follows; all ER-001 records are opened development data.
