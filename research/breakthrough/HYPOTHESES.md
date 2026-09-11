# Ten mechanism-level hypotheses

Scores 1–5, higher is better. Cheapness=5 means a small exact-oracle prototype; 1 means expensive multi-model research. Ranking is judgment before results. H01 and H07 have the best immediate information/cost ratio; H02 and H04 are next mechanistically different lines.

| ID | Mechanism | Classification | Gain | Math | Distinction | Cheapness |
|---|---|---|---:|---:|---:|---:|
| [H01](h01/HYPOTHESIS.md) | Exact boundary-response elimination | KNOWN mechanism | 4 | 5 | 5 | 5 |
| [H02](h02/HYPOTHESIS.md) | Boundary-conditioned exact tree moves | KNOWN subgraph inference | 4 | 5 | 5 | 3 |
| [H03](h03/HYPOTHESIS.md) | Incumbent-certified conditional freezing | KNOWN bound pruning | 5 | 5 | 4 | 2 |
| [H04](h04/HYPOTHESIS.md) | Gauge-aligned basin-conditioned distributions | DERIVED | 4 | 4 | 4 | 3 |
| [H05](h05/HYPOTHESIS.md) | Constraint-tangent moves from penalty structure | KNOWN feasible-space principle | 5 | 4 | 5 | 3 |
| [H06](h06/HYPOTHESIS.md) | Frustration-cycle coordinates | KNOWN duality | 5 | 4 | 5 | 2 |
| [H07](h07/HYPOTHESIS.md) | Cross-curvature barrier audit and 2-opt escape | KNOWN 2-flip formula | 3 | 5 | 4 | 5 |
| [H08](h08/HYPOTHESIS.md) | Vertex-equivalent spectral homotopy | KNOWN relaxation family | 4 | 3 | 5 | 2 |
| [H09](h09/HYPOTHESIS.md) | Basin-conditioned restart hazard | KNOWN restart theory | 3 | 4 | 3 | 3 |
| [H10](h10/HYPOTHESIS.md) | Shared certified response memory | NOVEL HYPOTHESIS composition | 5 | 4 | 5 | 2 |

## Multilevel route

H01 is the exact minimal multilevel construction: QUBO → eliminable boundary analysis → conditional response tables → residual coarse QUBO → unchanged optimizer → reverse lifting → original scoring. H02 extends to bounded-treewidth conditional refinement; H10 amortizes repeated boundary computations. Correlation-based contraction u_i=r_i z_C alone restricts the feasible set and can lose the optimum: it is a proposal subspace unless a certificate proves the relation. Weak coupling does not imply a small boundary or low treewidth. H04 provides statistical proposals rather than certificates.

Mathematical equivalence search covered exact elimination/star-triangle, bucket elimination, HFS/treewidth, EDA/hBOA, feasible-subspace mixers, cycle codes, r-flip formulas, spectral/analog landscape annealing, first-passage restart and context caching. Follow the linked primary papers and implementations in each hypothesis. This is a scoped novelty search, not exhaustive priority adjudication.
