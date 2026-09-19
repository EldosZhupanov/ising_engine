# Cross-domain mechanism map and prior-art matrix — CD001

Search date: 2026-09-17. Scoped first pass over seven families, not an exhaustive
map of science. Ising is the common test representation. Status labels apply to
specified mechanisms, never entire fields. Source sections below were inspected;
search results alone do not establish equivalence.

## Mechanisms and boundaries

| Field | Mathematical mechanism | Ising transfer | Boundary and type |
|---|---|---|---|
| Learning | p_next(i) proportional to p(i) exp(-eta loss_i) | reweight candidate configurations by energy | KNOWN exponential weights; regret versus available experts is not discovery of an unrepresented optimum; full support over configurations is exponential |
| Memory | compile a function/relation for later queries | F_B(z)=min_x E_B(x,z), retaining reconstruction | KNOWN compilation/context caching; representation size and supported queries trade off; universal compact exact compilation has conditional complexity barriers |
| Search | eliminate variables with min-plus messages | m_parent(z)=min_x[local energy + child messages] | KNOWN bucket elimination; dense-table cost exponential in induced width, not proof that every large-width instance is hard |
| Proof | conflicts justify reusable consequences | a certified bound L(partial)>incumbent excludes a subcube | KNOWN bound pruning / nogood principle; a bad local minimum is not a proof that its partial assignment is impossible |
| Control | replace repeated optimization with parameter-dependent solution regions | regions where a discrete optimizer stays optimal | KNOWN parametric-optimization principle; binary piecewise-constant maps differ from convex-QP piecewise-affine controllers; region enumeration may be large |
| Physical computation | perturb an equilibrium and compare responses | training through an energy-based dynamical system | KNOWN equilibrium propagation; smooth stable-state assumptions do not automatically transfer to a discrete argmin at level crossings; no hardware-energy savings inferred from CPU simulation |
| Biological adaptation | selection changes frequencies by relative fitness | population weighting/mutation for Ising states | KNOWN population-genetic / multiplicative-update connection under stated weak-selection model; no theorem of globally optimal spin assignment or preserved diversity follows |

## Source matrix and reading depth

| Proposed contribution | Closest primary source, inspected location | Same problem / mechanism / guarantee / decision space | Difference remaining |
|---|---|---|---|
| Energy exponential reweighting as a new learning rule | [Arora–Hazan–Kale](https://theoryofcomputing.org/articles/v008a006/v008a006.pdf), §2.1 Eq2.6 and Theorem2.3 | different loss application; same finite-expert update after substituting loss=scaled energy; regret requires bounded losses; same probability simplex | NONE for the update; NOT NOVEL |
| Evolution as a fundamentally new population optimizer | [Chastain et al.](https://arxiv.org/pdf/1208.3160), population dynamics and weak-selection theorem/corollary | biology differs; a specific multiplicative-update connection is established; assumptions are not arbitrary Ising selection | packaging alone adds no mechanism; broad equivalence of every evolutionary algorithm NOT asserted |
| Conditional optimum memory | [Darwiche–Marquis](https://arxiv.org/pdf/1106.1819), §3 Proposition3.1 and §4 query map; [context search](https://ics.uci.edu/~dechter/publications/r153a.html), abstract only | compilation is analogous; caching conditional subproblems overlaps; Boolean entailment lower bounds are not automatically Ising query lower bounds | OVERLAPPING; a new certified representation with better scoped tradeoff would need a separate claim |
| Exact block messages | [Dechter](https://ics.uci.edu/~csp/r76A.pdf), §2 and §4.2 Theorem4 | same finite-domain elimination algebra under min/sum; standard width-dependent guarantee | NONE for the general elimination idea; NOT NOVEL |
| Learning forbidden regions from failure | [GRASP](https://www.cs.cmu.edu/~emc/flac09/lectures/CSE-TR-292-96.pdf), §2–3 search/conflict analysis | SAT differs from soft optimization; logical consequence is the transferable principle; energy failure alone does not justify a clause | KNOWN principle, no proposed new proof system |
| Precompute parameter regions | [Bemporad et al.](https://cse.lab.imtlucca.it/~bemporad/publications/papers/automatica-mpqp.pdf), §3–4 multiparametric QP; [corrigendum](https://cse.lab.imtlucca.it/~bemporad/publications/papers/automatica-mpqp-corrige.pdf), corrected example7.1 | convex constrained QP differs from discrete Ising; offline parameter partition analogous, guarantees/decision space differ | OVERLAPPING, not exact equivalence of algorithms; finite binary lower-envelope construction is elementary |
| Energy relaxation as a novel learning primitive | [Scellier–Bengio](https://arxiv.org/pdf/1602.05179), §2–3 and gradient theorem | continuous states differ; equilibrium perturbation already used for training | NO novelty in the broad slogan; discrete/hardware extensions require separate mathematics |
| Rank-one boundary guarantees few responses (H11) | [Lucas](https://arxiv.org/pdf/1302.5843), §2.1 Eq6 square-of-linear Ising penalty; our elementary witness | same square-penalty construction, different conditional-response question | universal H11 has an explicit counterexample; exact historical priority of this witness NOT VERIFIED; not a new solver |

## Candidate transfers and decisions before implementation

T1: population genetics / online learning -> normalized energy reweighting.
Rule p_next(s)=p(s)exp(-eta E(s))/Z is exactly Hedge on a fixed configuration set.
Reject novelty of that rule before building it. Mutation/search of new states is
a separate mechanism; giving it a biological name supplies no new guarantee.

T2: explicit control -> parameter-cell response memory. If E_theta(x)=c_x+a_x^T theta,
then x is optimal on intersection_y {(a_x-a_y)^T theta <= c_y-c_x}.
This is an elementary lower envelope of affine functions. Efficiently constructing
or representing it is the unresolved computational work, not a new principle.
Do not conflate it with the convex MPC theorem. Reject the broad novelty claim.

T3/H11: low-rank model reduction -> a compact table of conditional minimizers.
A new local working hypothesis, not attributed to a paper. Rank alone omits
coefficient precision and interior interactions. Test the explicit Ising witness
in [h11/MATH](breakthrough/h11/MATH.md). No general speed benchmark is needed to
refute a universal response-count bound.

## Search coverage and unresolved questions

Queries included bucket elimination/unified reasoning; knowledge compilation map;
explicit constrained LQR/multiparametric QP (including the discovered corrigendum);
multiplicative weights/game/evolution; GRASP conflict learning; equilibrium
propagation; low-rank Ising; squared-penalty number partitioning.
The square penalty and reweighting equations were checked in full primary PDFs.
We did not exhaust patents, unpublished work, all solvers, or every language.
No result here establishes an unoccupied scientific direction. A possible future
question is a *specified*, precision-aware class of response programs that can be
learned cheaply and certified; its novelty and usefulness remain UNKNOWN.
