# H10: mechanism

For fixed B and boundary assignment z, cache F_B(z)=min_(x_B)E_B(x_B,z) with argmin and a full coefficient fingerprint. Reuse only on identical model, block and boundary key. Build a response only when predicted future visit count times solve cost exceeds build+lookup cost. On coefficient perturbation δ, a stored minimizer is certified unchanged if its gap g to runner-up exceeds 2 sup_x |δE_B(x,z)|; otherwise invalidate and recompute. Select blocks by estimated amortization divided by 2^|boundary|, not raw covariance magnitude. Each cache entry stores a conditional solution, never an unqualified global backbone.

Architecture: an isolated behavior-changing pass/operator/schedule; original objective remains the final arbiter. No correctness or equilibrium-sampling guarantee is inferred from a heuristic proposal score.
