---
id: cd005-math
kind: research-protocol
status: active
authority_scope: mathematical-derivation
created: 2026-09-19
immutable: false
---

# CD005 Mathematical Derivation: Edge-Completeness of 2-Opt Escapes

## 1. Hamiltonian & Spin Flip Algebra

Let the Ising Hamiltonian on graph $G = (V, E)$ be:
$$E(s) = - \sum_{(u, v) \in E} J_{uv} s_u s_v - \sum_{v \in V} h_v s_v, \quad s_v \in \{-1, +1\}$$

Let $e_i$ denote the elementary spin flip operation on variable $i$:
$$(s \oplus e_i)_k = \begin{cases} -s_i & \text{if } k = i \\ s_k & \text{if } k \ne i \end{cases}$$

### Local 1-Flip Delta $\Delta_i$
The change in energy under flipping spin $i$ is:
$$\Delta_i \equiv E(s \oplus e_i) - E(s) = 2 s_i \left( h_i + \sum_{j \in N(i)} J_{ij} s_j \right)$$

### 1-Opt Local Minimum Condition
A configuration $s$ is a **1-opt local minimum** if and only if no single spin flip can decrease the energy:
$$\Delta_i \ge 0 \quad \forall i \in V$$

---

## 2. Derivation of the Pairwise 2-Flip Delta $\Delta_{ij}$

Consider simultaneously flipping two distinct spins $i$ and $j$:
$$\Delta_{ij} \equiv E(s \oplus e_i \oplus e_j) - E(s)$$

By expanding the Hamiltonian:
$$\Delta_{ij} = \left[ E(s \oplus e_i) - E(s) \right] + \left[ E(s \oplus e_i \oplus e_j) - E(s \oplus e_i) \right]$$
The first term is $\Delta_i$.
In the second term, spin $j$ is flipped while spin $i$ has already been inverted to $-s_i$:
$$E(s \oplus e_i \oplus e_j) - E(s \oplus e_i) = 2 s_j \left( h_j + \sum_{k \ne i, j} J_{jk} s_k + J_{ji} (-s_i) \right)$$
$$= 2 s_j \left( h_j + \sum_{k \ne j} J_{jk} s_k - 2 J_{ij} s_i \right) = \Delta_j - 4 J_{ij} s_i s_j$$
Wait, let us verify the sign:
Old interaction energy between $i$ and $j$: $-J_{ij} s_i s_j$.
New interaction energy under $(s_i \to -s_i, s_j \to -s_j)$: $-J_{ij} (-s_i)(-s_j) = -J_{ij} s_i s_j$.
Notice: **the interaction energy between $i$ and $j$ does NOT change between the original state and the double-flipped state!**
However, the sum $\Delta_i + \Delta_j$ counted the change in the edge $(i, j)$ TWICE:
- In $\Delta_i$, the edge $(i, j)$ changed by $+2 J_{ij} s_i s_j$.
- In $\Delta_j$, the edge $(i, j)$ changed by $+2 J_{ij} s_i s_j$.
Therefore, $\Delta_i + \Delta_j$ overcounts the change on edge $(i, j)$ by $4 J_{ij} s_i s_j$:
$$\Delta_{ij} = \Delta_i + \Delta_j - 4 J_{ij} s_i s_j$$
Equivalently:
- If the edge $(i, j)$ is currently **frustrated** (i.e. $J_{ij} s_i s_j < 0$), then $-4 J_{ij} s_i s_j > 0$?
Wait! Let's check with an exact numerical example:
Suppose $s_1 = +1, s_2 = +1$, and $J_{12} = -1$ (anti-ferromagnetic edge).
Original edge energy: $-(-1)(+1)(+1) = +1$ (frustrated, high energy).
Flipping both spins: $s_1 \to -1, s_2 \to -1$.
New edge energy: $-(-1)(-1)(-1) = +1$ (still $+1$).
The edge energy itself did not change.
What was $\Delta_1$? $2 s_1 (J_{12} s_2) = 2(1)(-1)(1) = -2$.
What was $\Delta_2$? $2 s_2 (J_{12} s_1) = 2(1)(-1)(1) = -2$.
Sum $\Delta_1 + \Delta_2 = -4$.
Actual $\Delta_{12} = 0$.
So $\Delta_{12} = \Delta_1 + \Delta_2 - 4 J_{12} s_1 s_2 = -4 - 4(-1)(1)(1) = -4 + 4 = 0$. Perfect!

Now suppose $J_{12} = +1$ (ferromagnetic), $s_1 = +1, s_2 = -1$ (frustrated edge).
Original edge energy: $-(+1)(+1)(-1) = +1$.
New edge energy after double flip: $-(+1)(-1)(+1) = +1$.
What if $h_1 = -1.5, h_2 = +1.5$?
$\Delta_1 = 2(1)[-1.5 + 1(-1)] = 2(-2.5) = -5.0$.
At a 1-opt minimum, $\Delta_i \ge 0$ and $\Delta_j \ge 0$.
When can $\Delta_{ij} < 0$?
$$\Delta_{ij} = \Delta_i + \Delta_j - 4 J_{ij} s_i s_j$$
Since $\Delta_i \ge 0$ and $\Delta_j \ge 0$, we have $\Delta_i + \Delta_j \ge 0$.
For $\Delta_{ij} < 0$, we **strictly require**:
$$4 J_{ij} s_i s_j > \Delta_i + \Delta_j \ge 0 \implies J_{ij} s_i s_j > 0$$
Wait! If $J_{ij} s_i s_j > 0$, the edge is SATISFIED, and $-J_{ij} s_i s_j < 0$!
Wait, in ferromagnetic convention:
If $J_{ij} > 0$ and $s_i = s_j = +1$, the edge energy is $-J_{ij} < 0$ (satisfied).
Then $4 J_{ij} s_i s_j = 4 J_{ij} > 0$.
Then $\Delta_{ij} = \Delta_i + \Delta_j - 4 J_{ij}$.
If $4 J_{ij} > \Delta_i + \Delta_j$, then $\Delta_{ij} < 0$!
And if $(i, j) \notin E$, then $J_{ij} = 0$, so $\Delta_{ij} = \Delta_i + \Delta_j \ge 0$!

---

## 3. Theorem (Edge-Completeness of 2-Opt Escapes)

### Theorem 1
Let $s$ be any 1-opt local minimum ($\Delta_i \ge 0$ for all $i \in V$).
A pair of spins $(i, j)$ can yield an improving 2-flip ($\Delta_{ij} < 0$) **if and only if**:
1. $(i, j) \in E$ (the spins are directly connected by an edge in $G$); AND
2. $J_{ij} s_i s_j > \frac{1}{4} (\Delta_i + \Delta_j) \ge 0$.

### Corollary 1 (Computational Completeness)
To find ALL improving 2-flip moves from a 1-opt local minimum, it is **unnecessary to inspect all $\binom{N}{2}$ pairs**.
It suffices to inspect only the $|E|$ edges of the graph.
On sparse graphs with average degree $\langle d \rangle \ll N$, this reduces the neighborhood scan from $O(N^2)$ to $O(|E|) = O(N \langle d \rangle)$, achieving a speedup factor of:
$$\text{Speedup} = \frac{N(N-1)/2}{|E|} \approx \frac{N}{\langle d \rangle}$$
For $N = 1000$ on a degree-4 lattice: $\text{Speedup} \approx 250\times$.

---

## 4. Barrier Tunneling Formula

At a 1-opt minimum with $\Delta_{ij} < 0$:
Flipping $i$ first encounters an energy barrier of $\Delta_i \ge 0$.
Flipping $j$ first encounters an energy barrier of $\Delta_j \ge 0$.
The atomic 2-flip operator $(i, j)$ tunnels through the intermediate barrier:
$$B_{ij} = \min(\Delta_i, \Delta_j) \ge 0$$
escaping the 1-opt basin into an adjacent lower-energy basin in a single move.
