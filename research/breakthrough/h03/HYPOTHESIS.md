# H03: Incumbent-certified conditional freezing

Status: KNOWN bound pruning; DERIVED online allocation.

A good incumbent plus cheaply sharpened conditional lower bounds can certify incorrect branches which unconditional persistency leaves unresolved.

## Distinction from this repository

Existing probing propagates first-order persistencies. The extra information is an incumbent-conditioned lower-bound gap with provenance, not re-running the same QPBO labels.

## Prior art

[Glover, Lewis & Kochenberger](https://arxiv.org/abs/1705.09844); [D-Wave roof duality](https://docs.dwavequantum.com/en/latest/ocean/api_ref_preprocessing/api_ref.html). These identify equivalent principles; published benchmark advantages are not adopted as verified facts. Absence of an exact search hit does not establish novelty.

## Falsifier

Archive if bounds remain too loose on frustrated zero-field controls or bound computation consumes the saved search budget.

Scores (1 low, 5 high): potential gain 5, mathematics 5, implementation distinction 4, cheapness of first test 2. These are prioritization judgments, not measured effect sizes.
