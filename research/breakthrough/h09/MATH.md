# H09: mechanism

Given target first-passage survival S(t), fixed restart period τ and restart overhead c have mean cost R(τ)=[integral_0^τ S(t)dt+c]/[1-S(τ)]. Estimate survival with censoring, stratified by quotient-basin return, and choose τ on a frozen grid using held-out trajectories. Online continue when h(t|basin,Δ statistics) exceeds fresh-run success rate per cost; use shrinkage to global hazard and forced exploration. Unknown optimum requires a fixed intermediate target, not a target retrospectively set from candidate wins.

Architecture: an isolated behavior-changing pass/operator/schedule; original objective remains the final arbiter. No correctness or equilibrium-sampling guarantee is inferred from a heuristic proposal score.
