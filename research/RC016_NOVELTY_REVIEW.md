# RC-016 novelty review — frozen before held-in

**Date:** 2026-08-19

**Status:** read-only literature review; no RC-016 science seed was run.
**Effect on `PREREG_RC016_AMENDMENT_4.md` §D5:** narrows the novelty label; it
does not change an estimand, threshold, seed, corpus member, or verdict rule.

## Verdict

The broad claims are **KNOWN**:

- selecting an algorithm per instance from instance features;
- dynamically selecting/configuring algorithm components;
- failure of a fixed feature map to explain algorithm-performance variation;
- the abstract possibility that feature-similar instances have different best
  algorithms.

RC-016 must not claim discovery of any item above. If an opposite-action matched
pair survives both frozen seed blocks, the defensible contribution is narrower:

> A preregistered, RNG-addressed and independently replicated causal
> counterexample showing that the repository's frozen `S0` sensor map is not
> sufficient for choosing between `metropolis_sweep` and `gibbs_color_sweep`
> under the stated equal-sweep budget and `I_replace_work` estimand.

This may be described as a **domain-specific methodological/impossibility
certificate**, not as the discovery of algorithm selection, DAC, feature
insufficiency, or a deployment-optimal scheduler. The last prohibition also
follows Amendment 4 §D3: RC-016 contains no identified cost estimand.

## Primary-source evidence

1. Rice's algorithm-selection framework already maps instances through a feature
   space to algorithm choice/performance. The Purdue record identifies the 1976
   chapter and its original publication:
   <https://docs.lib.purdue.edu/cstech/99/>.
2. ASlib defines and benchmarks per-instance algorithm selection over instance
   features and algorithm-performance data. Bischl et al., *Artificial
   Intelligence* 237 (2016): <https://arxiv.org/abs/1506.02465>.
3. DACBench explicitly treats dynamic control of an algorithm's parameters as an
   established research problem and standardises benchmarks for it. Eimer et al.,
   IJCAI 2021: <https://www.ijcai.org/proceedings/2021/230>.
4. Instance Space Analysis explicitly treats inadequate features as a cause of
   contradictory or poorly predicted algorithm performance and recommends adding
   discriminating features. Smith-Miles and Muñoz, *ACM Computing Surveys* 55(12):
   <https://doi.org/10.1145/3572895>.
5. Beel et al. explicitly motivate performance-based similarity because ordinary
   meta-features may not carry enough information for algorithm selection. This is
   a workshop proposal rather than an Ising causal result, but it removes novelty
   from the abstract feature-collision idea:
   <https://arxiv.org/abs/2006.12328>.

## Boundary of the search

The review found no prior result with all of RC-016's narrow ingredients: this
specific Ising operator pair, the frozen five-scalar `S0` map, exact paired
counterfactual replacement under aligned random draws, preregistered matched-pair
tolerance, and held-out replication. Absence from a targeted search is not proof
of priority. Confidence is **high** that the broad framing is known and **medium**
that the narrow combined certificate is not already published.

Accordingly, future writing may only narrow this label further when closer prior
art is found; it may not widen it after observing RC-016 outcomes.
