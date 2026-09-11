# Scope of the global-flip symmetry argument

Prospective mathematical note. Does not edit or reinterpret frozen experiment outcomes in [RELATIONAL_PRIMITIVE.md](../RELATIONAL_PRIMITIVE.md) or RC-001. It narrows the *algorithmic extrapolation* made from those outcomes.

For E(s)=sum_(i<j) J_ij s_i s_j, E(s)=E(-s). If P(s)=P(-s), pairing each s with -s proves E_P[s_i]=0. The premise concerns a distribution, not the set of every possible optimization dynamics. Symmetry-broken product distributions, conditionals and finite-basin measures need not satisfy it.

Choose an anchor a and u_i=s_i s_a, u_a=1. This maps each global-flip orbit onto one representative and leaves pair products unchanged: s_i s_j=u_i u_j. Therefore min_s E(s)=min_(u:u_a=1) E(u). Under the symmetric original measure, E[u_i]=E[s_i s_a], a two-point correlation which need not vanish. This is relational information expressed in quotient coordinates. It removes only one binary degree of freedom per independent field-free component; it does not make a spin glass easy.

Counterexample: E=-s_1 s_2 has optima (++),(--). Their symmetric mixture has zero marginals. After gauge alignment both become (++); the product distribution concentrated there has nonzero marginals and finds an optimum with probability one. This directly falsifies the universal marginal-state corollary, not the Gibbs theorem. Multimodality after alignment remains a failure mode; products can generate configurations absent from every good basin. Field-bearing models cannot be globally flipped without changing objective, so this gauge is gated on exact zero Ising field.

Signed relation consistency also requires product_(edges in C) r_ij=+1 on every cycle C, not absence of every odd-length cycle. A triangle with all r=+1 is consistent. General inconsistent weighted relations retain an optimization problem: union-find only checks hard relations after they have been selected.

Experiment status: proof first; an executable counterexample will be included in the isolated prototype test suite. No quality, speed, novelty or production claim follows from this note.
