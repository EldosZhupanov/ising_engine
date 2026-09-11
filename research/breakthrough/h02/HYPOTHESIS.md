# H02: Boundary-conditioned exact tree moves

Status: KNOWN subgraph inference; DERIVED selection rule.

Large low-treewidth conditional moves can cross local barriers more efficiently than covariance flips if blocks are chosen by benefit per boundary cost.

## Distinction from this repository

Houdayer exchanges overlap clusters; synthesis flips one binary group. Exact tree inference searches 2^|B| assignments by DP and can change only part of a block.

## Prior art

[Selby 2014](https://arxiv.org/abs/1409.3934); [reference implementation](https://github.com/alex1770/QUBO-Chimera). These identify equivalent principles; published benchmark advantages are not adopted as verified facts. Absence of an exact search hit does not establish novelty.

## Falsifier

Kill the selection heuristic if shuffled/random trees tie at counted work; keep inference only if it beats stronger baseline after analysis cost.

Scores (1 low, 5 high): potential gain 4, mathematics 5, implementation distinction 5, cheapness of first test 3. These are prioritization judgments, not measured effect sizes.
