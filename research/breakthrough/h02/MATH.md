# H02: mechanism

For an induced tree B with outside assignment fixed, h_i^B=h_i+sum_(j outside B)q_ij x_j. Messages m_(i→p)(x_p)=min_(x_i∈{0,1})[h_i^B x_i+q_ip x_i x_p+sum_(k child i)m_(k→i)(x_i)]. Backtrack the minimum; accept unconditionally since current block assignment was feasible. Candidate block utility U(B)=[sum_(i in B)max(0,-Δ_i)+sum_(ij internal)max(0,-q_ij d_i d_j)]/[|B|+|∂E B|]. The numerator is a heuristic proposal score, NOT a lower bound or guaranteed improvement. Only exact induced-tree edges may be used: ignoring chords is incorrect.

Architecture: an isolated behavior-changing pass/operator/schedule; original objective remains the final arbiter. No correctness or equilibrium-sampling guarantee is inferred from a heuristic proposal score.
