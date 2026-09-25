---
name: claim-audit
description: Formal audit matrix and epistemic status tracking for all scientific claims.
---

# Claim Audit Protocol

This skill enforces continuous auditing of every scientific assertion made in the project repository, papers, or documentation.

## Epistemic Status Taxonomy

Every claim in the project must be assigned one of four formal statuses:

1. `SUPPORTED`: Direct, reproducible experimental or mathematical proof exists within this repository, verified by independent checks and surviving falsification attempts.
2. `PARTIALLY_SUPPORTED`: Evidence exists for a subset of conditions/instances, but boundaries or counterexamples have been observed.
3. `UNVERIFIED`: The claim is plausible, hypothesized, or reported in external literature, but has not been independently verified under our frozen protocols.
4. `FALSIFIED`: Experimental data or theoretical counterexamples have disproven the claim.

*Rule: It is strictly forbidden to upgrade a claim from UNVERIFIED to SUPPORTED without publishing reproducible evidence.*

## Claim Audit Table Format

Maintain in `research/CLAIMS.md`:

| Claim ID | Statement | Direct Evidence | Independent Verification | Relevant Literature | Counter-Evidence / Boundary | Confidence | Status |
|---|---|---|---|---|---|---|---|
| `CLM-001` | *Exact statement* | *Path to RESULT.md* | *Verifier name & checksum* | *DOI / arXiv link* | *Observed failures / limits* | *High / Med / Low* | `SUPPORTED` |

## Review Criteria
- An audit pass must be performed before any manuscript submission or major release.
- Any claim found lacking empirical logs or mathematical proof must be demoted to `UNVERIFIED` or `FALSIFIED`.
