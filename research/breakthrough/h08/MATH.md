# H08: mechanism

On z∈[0,1]^n use F_α(z)=E(z)+sum α_i(z_i²-z_i). Every binary vertex has F_α=E for all α. Dynamics: v_(t+1)=μv_t-η[∇E(z_t)+α_t⊙(2z_t-1)]; z_(t+1)=clip(z_t+v_(t+1),0,1). Adapt α using the smallest Hessian eigenmode and a trust region; periodically round and accept only canonically scored incumbent improvements. Vertex equivalence does NOT imply relaxed local minima, trajectories or ground-state rounding are preserved. Compute Lanczos/matvec costs explicitly.

Architecture: an isolated behavior-changing pass/operator/schedule; original objective remains the final arbiter. No correctness or equilibrium-sampling guarantee is inferred from a heuristic proposal score.
