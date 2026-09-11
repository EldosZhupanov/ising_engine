# H07: mechanism

With d_i=1-2x_i and Δ_i=d_i(h_i+sum q_ij x_j), Δ_(ij)=Δ_i+Δ_j+q_ij d_i d_j. At a 1-opt minimum Δ_i≥0. A pair can improve only if q_ij d_i d_j<0. For q_ij=0 no improving pair exists there, so scanning graph edges is COMPLETE for two-flip improvement. The minimum sequential two-flip barrier relative to the start is B_ij=min(max(0,Δ_i,Δ_ij),max(0,Δ_j,Δ_ij)). Execute the most negative Δ_ij, then restore 1-opt. This barrier pertains to these two length-2 paths only, not the basin escape barrier.

Architecture: an isolated behavior-changing pass/operator/schedule; original objective remains the final arbiter. No correctness or equilibrium-sampling guarantee is inferred from a heuristic proposal score.
