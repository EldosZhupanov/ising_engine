# H09: Basin-conditioned restart hazard

Status: KNOWN restart theory; DERIVED observables.

Estimate survival of improvement events conditional on return to the same basin and spend restarts when marginal continuation value is low.

## Distinction from this repository

Luby restart and Dynamics early-stop prediction exist; the new estimand is censored target hazard conditioned on repeated basin visits, with reset overhead accounted.

## Prior art

[First-passage restart](https://arxiv.org/abs/1512.01600); [Completion probabilities and parallel restarts](https://pmc.ncbi.nlm.nih.gov/articles/PMC5061357/). These identify equivalent principles; published benchmark advantages are not adopted as verified facts. Absence of an exact search hit does not establish novelty.

## Falsifier

Reject if restart advantage disappears against Luby/random restarts or fitting cost; never learn and evaluate hazard on the same seed histories.

Scores (1 low, 5 high): potential gain 3, mathematics 4, implementation distinction 3, cheapness of first test 3. These are prioritization judgments, not measured effect sizes.
