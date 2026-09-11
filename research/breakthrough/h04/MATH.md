# H04: mechanism

For exact field-free Ising, align u_i^r=s_i^r s_a^r. Group elites by quotient Hamming distance. In basin k fit p_k(u)=p_k(u_root) product_(i≠root)p_k(u_i|u_parent) on a maximum-mutual-information spanning tree; use Dirichlet pseudocounts. Sample from mixture sum π_k p_k with π_k∝exp(-β E_k)/(1+n_k), mix ε independent random restarts, then score on ORIGINAL E. Anneal β via d log β/dt=η(H_observed-H_target), capped; this is deliberately nonequilibrium feedback, not Gibbs sampling. No hard freezing from consensus alone.

Architecture: an isolated behavior-changing pass/operator/schedule; original objective remains the final arbiter. No correctness or equilibrium-sampling guarantee is inferred from a heuristic proposal score.
