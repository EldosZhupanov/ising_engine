# H01: Exact boundary-response elimination

Status: KNOWN mechanism; DERIVED repository application.

On graphs with many recursively degree≤2 vertices, replace stochastic interior search by exact conditional response tables and reduce the residual problem enough to repay preprocessing.

## Distinction from this repository

Value-fixing QPBO/probing and disconnected-component solves exist. Conditional elimination with changing coefficients and a lifting stack was not found. Coarse hierarchy was already theorized in AXIOMS §8.9.

## Prior art

[Boettcher & Davidheiser 2008](https://arxiv.org/abs/0802.1941); [Dechter 1999](https://www.sciencedirect.com/science/article/pii/S0004370299000594). These identify equivalent principles; published benchmark advantages are not adopted as verified facts. Absence of an exact search hit does not establish novelty.

## Falsifier

Archive as broadly useful if negligible reduction on general sparse/dense controls or no ≥25% end-to-end advantage at matched quality. Incorrect lift is an immediate correctness failure.

Scores (1 low, 5 high): potential gain 4, mathematics 5, implementation distinction 5, cheapness of first test 5. These are prioritization judgments, not measured effect sizes.
