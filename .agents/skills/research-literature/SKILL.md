---
name: research-literature
description: Scientific literature search and rigorous bibliographic verification protocol for Ising and combinatorial optimization.
---

# Research Literature Protocol

This skill governs all scientific literature search, citation verification, and prior art analysis in Ising Engine.

## Core Rules

1. **Source Hierarchy**:
   1. Primary published peer-reviewed paper (journal/proceedings).
   2. Official author code repository and benchmark submission artifact.
   3. Publisher DOI / CrossRef metadata.
   4. arXiv preprint (explicitly labeled as preprint).
   5. Technical documentation from original authors.
   *Blog posts, marketing whitepapers, and promotional benchmarks are strictly prohibited as primary evidence.*

2. **Verification Requirements**:
   - Check peer-reviewed status: distinguish preprints from published papers.
   - Record exact DOI, publication year, journal/conference venue, and volume/pages.
   - Check for newer versions, retractions, errata, or follow-up amendments.
   - Verify citation claims: never cite a paper based on an LLM hallucination or an abstract without checking that the text directly supports the claim.
   - Actively search for competing methods and negative/falsifying results.

3. **No Fabricated Citations**:
   - Every cited paper must have a confirmed DOI, arXiv identifier, or verified publisher URL.
   - If a citation cannot be independently confirmed via CrossRef, arXiv, or publisher lookup, mark it as `[UNCONFIRMED CITATION]` and do not cite it as established authority.

4. **Literature Search Workflow**:
   1. Search peer-reviewed and arXiv databases for exact keywords and problem formulations.
   2. Map the lineage of the algorithmic idea (who originated it, who proved complexity bounds, who established empirical baselines).
   3. Extract numerical baseline numbers directly from the paper's tables, noting exact hardware, timeouts, and metric definitions.
   4. Archive verified bibliographic entries in `research/literature/`.
