# H11 derivation and explicit counterexample

Let x,z in {0,1}^b and a_i=2^i, i=0,...,b-1. Fix z as the boundary and minimize

E(x,z)=(a^T x-a^T z)^2.

The rectangular coefficient W in x^T Wz is -2aa^T, exactly rank one. Expanding
with binary idempotence gives unary a_i^2 for each x_i and z_i, within-block
coefficient 2a_i a_j, and cross coefficient -2a_i a_j. Full symmetric square
matrix vv^T, v=(a,-a), is rank one too, but deleting its diagonal or using an
upper-triangular QUBO convention does NOT preserve that rank assertion.

Binary positional representation is injective. Thus for EVERY z the unique
minimizer is x=z with E=0. All 2^b boundary values have different minimizers.
An adjacent integer exists at every endpoint/interior value, so the next-best
gap is exactly 1 for every z, including all zeros/all ones. This disproves H11
without claiming NP-hardness: the optimizer is the identity map, O(b) output
operations. A large response count is not a lower bound on program size/runtime.

For native Ising s=2x-1 and t=2z-1,
E=(a^T(s-t))^2/4. Let v=(a,-a), y=(s,t). Then
4E=sum v_i^2 + sum_(i<j)2v_i v_j y_i y_j.
No fields are needed. Integer 4E allows exact independent scoring with no float.

Maximum absolute coefficient of the CANONICAL BINARY POLYNOMIAL is
C_b=2^(2b-1), attained at x_(b-1) z_(b-1). Normalize that polynomial by C_b:
all coefficients <=1 but its unique-minimum gap becomes 2^(1-2b).
This witness therefore does not refute a theorem that additionally fixes a
positive normalized gap or coefficient precision. The binary coefficient
bit-length grows O(b), not exponentially; value magnitude grows exponentially.

A boundary on the boundary: when there are no interior interactions and
E(x,t)=sum_i(c_i+a_i t)x_i with a_i>0, each bit changes at most once as t grows,
so a consistent zero-at-tie choice yields at most b+1 configurations. Interior
coupling invalidates this separable threshold argument. For integer z-weights
bounded by M, their nonnegative scalar sum has at most bM+1 values; that is a
pseudo-polynomial encoding bound, not a polynomial exact solver for general
binary-encoded coefficients. Do not replace the rejected universal claim with
these narrower assumptions after seeing results and call it confirmation.

Interpretation: rank, precision, response count, and response-program complexity
are four different quantities. Generic low-rank compression alone is not a route
to a new universal Ising optimizer. The square construction is an elementary
use of known penalty encoding, not claimed as a new theorem for science.

## Stronger assumptions: exact bounds, not a rescued H11

For a fixed interior energy E_0(x), the boundary enters only via f=Wz. If W has
integer entries bounded by C and rank r, choose r independent rows. Their outputs
are integers in [-bC,bC], so at most (2bC+1)^r field vectors are possible. With a
fixed deterministic tie-break depending only on the objective, the same bound
holds for selected answers. This can still be exponential in coefficient bit size.

For unique minimizers x at f and x' at f', each with gap at least gamma,

E_0(x')-E_0(x)+f^T(x'-x) >= gamma,
E_0(x)-E_0(x')+f'^T(x-x') >= gamma.

Adding gives (f-f')^T(x'-x) >= 2 gamma. Because binary states differ in at most
n coordinates, ||f-f'||_2 >= 2 gamma/sqrt(n). If all fields lie in a radius-R
ball of an r-dimensional subspace, disjoint radius gamma/sqrt(n) balls around
one representative per distinct answer give at most
(1+R sqrt(n)/gamma)^r answers by volume comparison. Radius is measured in that
subspace; n is the number of interior variables. This elementary packing bound
includes interior couplings but requires a uniform positive margin. It bounds
answer count, not the cost of finding the answers or certifying their gap.

These bounds were independently derived during review. Their scientific priority
has NOT been established. They delimit the counterexample rather than supplying
a new claimed algorithm. Numerical tests below address the explicit rank-only
witness; they do not empirically confirm these universal bounds.
