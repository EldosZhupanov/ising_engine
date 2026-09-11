# H03: mechanism

Let U=E(x_best), L_(i,b)≤min_(x:x_i=b) E(x). If L_(i,b)>U then no global optimum has x_i=b. With numerical bounds use certified outward error intervals and strict separation. Under temporary assumptions A, certificates apply ONLY within A and expire on unfreezing A. Allocate next probing work by (U-L_(i,b))^-1 divided by estimated bound cost; this allocation is heuristic, the bound implication is exact.

Architecture: an isolated behavior-changing pass/operator/schedule; original objective remains the final arbiter. No correctness or equilibrium-sampling guarantee is inferred from a heuristic proposal score.
