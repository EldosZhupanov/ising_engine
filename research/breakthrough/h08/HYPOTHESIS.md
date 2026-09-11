# H08: Vertex-equivalent spectral homotopy

Status: KNOWN relaxation family; DERIVED feedback.

Changing curvature between binary vertices can produce informed collective proposals without changing the binary optimum.

## Distinction from this repository

NMF and a spectral descriptor exist; no continuous spectral dynamics found in compiled solver operators. Different from deterministic ANLS/NMF warmstart claims in demos.

## Prior art

[Geometric Landscape Annealing, PRX 2024](https://doi.org/10.1103/PhysRevX.14.031054); [multilevel QUBO](https://arxiv.org/abs/2408.07793). These identify equivalent principles; published benchmark advantages are not adopted as verified facts. Absence of an exact search hit does not establish novelty.

## Falsifier

Reject if random projected directions or equal-cost multistart equal the spectral arm, or spectrum extraction dominates.

Scores (1 low, 5 high): potential gain 4, mathematics 3, implementation distinction 5, cheapness of first test 2. These are prioritization judgments, not measured effect sizes.
