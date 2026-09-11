# H01: mechanism

Write E=E_rest+x_v(a+b x_i+c x_j). Then f(x_i,x_j)=min(0,a+b x_i+c x_j). Set offset+=f00; h_i+=f10-f00; h_j+=f01-f00; q_ij+=f11-f10-f01+f00. Record x_v*(x_i,x_j)=1[a+b x_i+c x_j<0]. Reverse elimination gives E_original(lift(y))=E_reduced(y) for EVERY residual y. Degree 0/1 use the corresponding smaller table. Repeated elimination stays QUBO only while boundary degree≤2. Arbitrary degree 3 with fields can generate cubic terms; never discard them. Search dimension becomes k residual vertices; table/graph construction still costs time and memory.

Architecture: an isolated behavior-changing pass/operator/schedule; original objective remains the final arbiter. No correctness or equilibrium-sampling guarantee is inferred from a heuristic proposal score.
