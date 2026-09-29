# Odd covering systems: read-only intake (2026-09-29)

**Question / hypothesis.** Can the certificate method of [Mian–Siddique,
arXiv:2607.25628](https://arxiv.org/abs/2607.25628) be extended from
`lcm > 10,000` to a new, independently checkable exclusion on this laptop?
**Status: INCONCLUSIVE for stronger mathematics; FALSIFIED for extending the
published certificate beyond its first obstruction at 10,395.** No search, build, or foreign
code was run in this intake.

| Item | Finding |
|---|---|
| Published method | This is an exhaustive **exclusion**, not a search for congruence residues. A covering with lcm `N` needs `2N <= sigma(N)`; only 23 odd non-deficient `N <= 10,000` remain. For each, a pairwise-coprime divisor set `T` gives a CRT forced-overlap/capacity inequality excluding every residue assignment. [Lean repository](https://github.com/ibrahimmian36/centurion) checks the arithmetic, the completeness of the 23-candidate list, and the passage from a finite period to all integers. |
| First barrier | Since there is no odd non-deficient candidate between `10,000` and `10,395`, the same argument can trivially restate the threshold as `lcm > 10,394`; this is not new mathematics. The paper explicitly reports that its capacity test fails for `N = 10,395`, `12,285`, and `17,325`. A larger enumeration bound alone cannot pass `10,395`; a sharper overlap/peeling bound or independently checkable UNSAT certificate is needed. |
| Our machine / cost | 4 logical CPUs, 8.7 GiB RAM; Lean is not installed. Exact-integer screening of candidates modestly above 10,000 appears feasible, but no runtime or memory claim is measured here. The authors report about 10 minutes for their pinned Lean 4.30.0/mathlib build at 10,000 after fetching caches; a stronger proof has unknown cost. No large download or computation is justified before a new lemma/certificate is specified. |
| Duplicate risk | **High.** The 2025 [McNew–Setty work](https://arxiv.org/abs/2507.23041) already studies covering numbers to `10^6` with MIP, although its table leaves one primitive-count ambiguity (`94 or 95`), so it should not be described as a fully independently certified odd exclusion. A later [open-problems-lab repository](https://github.com/BrettKnox/open-problems-lab) *claims* an exact-rational screen excluding odd lcm `<= 10^6` and a Lean proof of the underlying inequality, but the finite scan was not reproduced here. An [August 2026 author preprint](https://www.researchgate.net/publication/412318119_Certified_coverage_bounds_and_a_nonexistence_range_for_odd_covering_systems_the_Erdos-Selfridge_problem_Erdos_Problem_7) *claims* independently checked certificates to `2,000,000` using symmetry, recursive peeling, and branch-and-bound; no independent replay or peer-reviewed confirmation was established here. The Erdős–Selfridge question remains open. |
| Verification | For a **positive** covering, an independent exact script checking distinct odd moduli and all residues in one lcm period suffices as a finite witness check. For a **negative** bound, an exact script can replay a finite list of capacity inequalities and candidate enumeration only when the sufficiency theorem and completeness argument are proved or independently reviewed. Lean is not required to explore, but Lean or another proof assistant is needed to match the paper's kernel-checked assurance; an UNSAT exit code alone is not a certificate. |

**Decision — НЕ ДЕЛАТЬ** a bound-raising or Ising-solver campaign now. The
current certificate meets its first documented obstruction just above 10,000,
and larger numerical claims are already public. Reopen only for a specific new
inequality/proof certificate that excludes `10,395` and is tested against the
later preprints, with a separate exact checker and a realistic proof cost.

**Evidence limits:** primary paper and author repositories/preprints were read
on 2026-09-29; external numeric claims are attributed, not reproduced. The
machine description is a local inventory, not a benchmark. No novelty, bound,
or runtime improvement is claimed for Ising Engine.
