# H07: Cross-curvature barrier audit and 2-opt escape

Status: KNOWN 2-flip formula; DERIVED diagnostic use.

At production 1-opt minima, exact pair cross-terms identify barriers invisible to individual Δ values; determine whether this cheap missing neighborhood matters.

## Distinction from this repository

Production finishes at 1-opt. Path relinking can visit two flips but only endpoint-directed paths; covariance moves do not exhaust graph-edge two-flips. Historical per-variable learning has no cross-curvature certificate.

## Prior art

[Alidaee et al. r-flip formula](https://www.mdpi.com/1999-4893/16/12/557); [Hao et al. QUBO metaheuristics](https://leria-info.univ-angers.fr/~jinkao.hao/papers/MetaHeuristicsQUBOBook2022.pdf). These identify equivalent principles; published benchmark advantages are not adopted as verified facts. Absence of an exact search hit does not establish novelty.

## Falsifier

Archive as material improvement if all full-solver effects remain below 1% normalized quality and no matched-time benefit; retain barrier counts as a diagnostic rather than pretend a breakthrough.

Scores (1 low, 5 high): potential gain 3, mathematics 5, implementation distinction 4, cheapness of first test 5. These are prioritization judgments, not measured effect sizes.
