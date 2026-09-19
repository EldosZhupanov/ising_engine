---
id: cd003-math
kind: research-protocol
status: active
authority_scope: mathematical-derivation
created: 2026-09-19
immutable: false
---

# CD003 Mathematical Derivation: Precision-Rank Bounded Response

## 1. Problem Formulation

Consider an Ising/QUBO objective defined over two interacting subsystems $x \in \{0, 1\}^n$ (internal) and $z \in \{0, 1\}^b$ (boundary):
$$E(x, z) = x^T Q_x x + x^T J_{xz} z + z^T Q_z z + h_x^T x + h_z^T z$$

When $z$ is fixed as a boundary context, the optimal conditional internal configuration is:
$$x^*(z) = \arg\min_{x \in \{0, 1\}^n} \left[ E_{\text{int}}(x) + x^T J_{xz} z \right]$$
where $E_{\text{int}}(x) = x^T Q_x x + h_x^T x$, and terms depending solely on $z$ do not alter the minimizer over $x$.

---

## 2. Low-Rank Factorization & Parameter Projection

Suppose the cross-coupling matrix $J_{xz} \in \mathbb{R}^{n \times b}$ has rank $r$.
It can be factored as:
$$J_{xz} = U V^T, \quad U \in \mathbb{R}^{n \times r}, \quad V \in \mathbb{Z}^{b \times r}$$
where the columns of $V = (\mathbf{v}_1, \dots, \mathbf{v}_r)$ have bounded integer entries:
$$\|V\|_\infty = \max_{j, k} |V_{jk}| \le K$$

Then the cross-interaction decomposes into:
$$x^T J_{xz} z = x^T (U V^T) z = (U^T x)^T (V^T z)$$

Define the **boundary projection mapping**:
$$\boldsymbol{\theta}(z) \equiv V^T z \in \mathbb{R}^r$$
where each component is:
$$\theta_k(z) = \mathbf{v}_k^T z = \sum_{j=1}^b V_{jk} z_j, \quad k \in \{1, \dots, r\}$$

---

## 3. Proof of the Precision-Rank Bound

### Lemma 1 (Image Size of Bounded Linear Integer Map on $\{0, 1\}^b$)
Let $\mathbf{v} \in \mathbb{Z}^b$ with $\|\mathbf{v}\|_\infty \le K$.
For any $z \in \{0, 1\}^b$, the scalar product $S = \sum_{j=1}^b v_j z_j$ is an integer satisfying:
$$S_{\min} = \sum_{j: v_j < 0} v_j \ge -b K, \quad S_{\max} = \sum_{j: v_j > 0} v_j \le b K$$
Because $S$ can only take integer values in the discrete interval $[S_{\min}, S_{\max}]$, the number of distinct values of $\mathbf{v}^T z$ is at most:
$$|\{ \mathbf{v}^T z : z \in \{0, 1\}^b \}| \le S_{\max} - S_{\min} + 1 \le 2 b K + 1$$
*(When all $v_j \ge 0$, the bound tightens to $b K + 1$.)*

### Lemma 2 (Conditional Minimizer Invariance Under Invariant $\boldsymbol{\theta}$)
For any two boundary vectors $z_1, z_2 \in \{0, 1\}^b$, if $V^T z_1 = V^T z_2 = \boldsymbol{\theta}$, then:
$$E(x, z_1) - E_{\text{bound}}(z_1) = E_{\text{int}}(x) + \boldsymbol{\theta}^T (U^T x) = E(x, z_2) - E_{\text{bound}}(z_2)$$
The objective functions over $x$ are strictly identical.
Under any deterministic tie-breaking rule (e.g. lowest integer index), the conditional minimizer satisfies:
$$x^*(z_1) = x^*(z_2)$$

### Theorem 1 (Precision-Rank Bounded Response)
Under rank $r$ and integer precision $\|V\|_\infty \le K$, the number of unique conditional responses $N_{\text{resp}} = |\{ x^*(z) : z \in \{0, 1\}^b \}|$ is bounded by the number of distinct values of $\boldsymbol{\theta}(z)$:
$$N_{\text{resp}} \le \prod_{k=1}^r (2 b K + 1) = (2 b K + 1)^r$$

---

## 4. Phase Transition: Polynomial vs Exponential

The number of boundary configurations is $2^b = 2^{b \ln 2}$.
The precision-rank bound is $(2 b K + 1)^r \approx (2 b K)^r$.

Let the coefficient bit-depth be $p = \log_2 K$.
Then $K = 2^p$, and the bound is:
$$N_{\text{resp}} \le (2 b \cdot 2^p)^r = (2b)^r \cdot 2^{r p}$$

We identify three distinct structural regimes:
1. **Constant Precision ($p = O(1)$, $K = O(1)$):**
   $$N_{\text{resp}} \le O(b^r) \ll 2^b$$
   The response space is **strictly polynomial**. Structural compression is mathematically guaranteed.
2. **Logarithmic Precision ($p = c \log_2 b$):**
   $$N_{\text{resp}} \le (2b)^{r(c+1)} = O(b^{r(c+1)})$$
   Still polynomial!
3. **Linear Precision ($p = b/r$):**
   $$N_{\text{resp}} \le (2b)^r \cdot 2^b$$
   The bound becomes vacuous; the system can realize all $2^b$ unique responses (as in the H11 counterexample where $p = b-1$, $r=1$).

**Crucial Corollary:**
Low interface rank ($r \ll b$) compresses conditional response complexity **if and only if** coefficient precision is sublinear in boundary size ($p \ll b/r$).
