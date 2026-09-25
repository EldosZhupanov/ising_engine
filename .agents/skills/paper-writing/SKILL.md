---
name: paper-writing
description: Epistemic precision, evidence alignment, and vocabulary rules for scientific papers and manuscripts.
---

# Paper Writing & Scientific Manuscript Protocol

This skill enforces academic rigor and epistemic calibration in all manuscripts, papers, and public reports.

## Epistemic Hierarchy of Verbs

Never blur the boundary between deductive proof, empirical observation, and heuristic conjecture. Use precise verbs:

- **"We prove"**: Reserved strictly for deductive mathematical proofs with complete steps and verified lemmas.
- **"We show empirically"**: Used when a hypothesis is supported by preregistered experiments across multiple seeds, surviving statistical hypothesis tests ($p < 0.05$) and independent verification.
- **"We observe"**: Used for exploratory data patterns or empirical phenomena where the governing causal mechanism is not yet proven.
- **"We hypothesize"**: Used for tentative explanations proposed to account for observed phenomena, subject to future falsification.
- **"We conjecture"**: Used for mathematical propositions believed to be true based on strong heuristic evidence, but lacking formal proof.

## Prohibited Hyperbole

The following words are prohibited unless accompanied by direct, comprehensive benchmark evidence against all published baselines under standardized hardware protocols:
- ❌ `state-of-the-art (SOTA)`
- ❌ `breakthrough`
- ❌ `unprecedented`
- ❌ `superior`
- ❌ `quantum advantage`
- ❌ `scaling advantage`
- ❌ `revolutionary`

## Manuscript Checklist Before Submission
1. Are all baselines evaluated on identical hardware or explicitly labeled with hardware specs?
2. Are all random seeds reported, and are error bars / confidence intervals displayed on all figures?
3. Are all failed runs and timeouts included in the summary statistics?
4. Are negative results and performance boundaries disclosed alongside positive findings?
5. Does every quantitative claim map directly to a machine-readable artifact in `research/results/`?
