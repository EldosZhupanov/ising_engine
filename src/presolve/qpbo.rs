//! Roof duality / QPBO via max-flow (weak persistencies).
//!
//! References:
//! - Hammer, Hansen & Simeone, "Roof duality, complementation and
//!   persistency in quadratic 0-1 optimization", Math. Programming 28 (1984).
//! - Boros & Hammer, "Pseudo-Boolean optimization", Discrete Appl. Math. 123
//!   (2002) — the implication-network (network N_f) formulation used here.
//! - Kolmogorov & Rother, "Minimizing non-submodular functions with graph
//!   cuts — a review", IEEE PAMI 29 (2007).
//!
//! ## Construction (derived from first principles)
//!
//! Minimize E(x) = Σ_i a_i x_i + Σ_{i<j} b_ij x_i x_j over x ∈ {0,1}^n.
//! Rewrite E as a POSIFORM: a constant plus a sum of terms c·ℓ (linear) and
//! c·ℓ_p·ℓ_q (quadratic) with every coefficient c ≥ 0, over literals
//! ℓ ∈ {x_i, x̄_i}. Every QUBO admits one:
//! - a_i x_i: a_i≥0 → a_i·x_i ; a_i<0 → const += a_i, |a_i|·x̄_i.
//! - b x_i x_j: b≥0 → b·x_i·x_j ; b<0 → (b·x_i as linear) + |b|·x_i·x̄_j.
//!
//! Each positive term penalizes "all its literals = 1". Build the implication
//! network on 2n literal nodes plus source T (the constant-true literal) and
//! sink F (=T̄): a term c·(u·v) adds arcs u → v̄ and v → ū of capacity c
//! (implications "u ⇒ v̄" and "v ⇒ ū" that discourage u=v=1); a linear term
//! c·u is c·(u·T) → arcs u → F and T → ū. Source→sink max-flow computes the
//! roof-dual bound; in the residual graph, a variable whose literal x_i and
//! complement x̄_i lie in DIFFERENT strongly connected components is
//! persistently fixed (weak persistency), to the value determined by the
//! source-side of the residual min cut.
//!
//! ## Safety
//!
//! The free/fixed decision is convention-independent (SCC equality). The
//! fixed VALUE is read from residual source-reachability; its polarity was
//! calibrated against brute force and is regression-locked. All capacities
//! are exact f64; Dinic's phase count is capacity-agnostic (O(V²E)), so real
//! weights terminate. A residual epsilon guards float noise. Every fixing is
//! exhaustively verified optimum-preserving in the test suite.

use crate::core::QuboModel;

const EPS: f64 = 1e-9;

/// Dinic max-flow on f64 capacities.
struct Dinic {
    n: usize,
    head: Vec<Vec<usize>>, // adjacency: node -> indices into `edges`
    to: Vec<usize>,
    cap: Vec<f64>,
    level: Vec<i32>,
    it: Vec<usize>,
}

impl Dinic {
    fn new(n: usize) -> Self {
        Self {
            n,
            head: vec![Vec::new(); n],
            to: Vec::new(),
            cap: Vec::new(),
            level: vec![0; n],
            it: vec![0; n],
        }
    }

    /// Adds a directed edge u→v with capacity `c` (and its residual reverse 0).
    fn add_edge(&mut self, u: usize, v: usize, c: f64) {
        let e = self.to.len();
        self.to.push(v);
        self.cap.push(c);
        self.head[u].push(e);
        self.to.push(u);
        self.cap.push(0.0);
        self.head[v].push(e + 1);
    }

    fn bfs(&mut self, s: usize, t: usize) -> bool {
        self.level.iter_mut().for_each(|x| *x = -1);
        let mut q = std::collections::VecDeque::new();
        self.level[s] = 0;
        q.push_back(s);
        while let Some(u) = q.pop_front() {
            for &e in &self.head[u] {
                if self.cap[e] > EPS && self.level[self.to[e]] < 0 {
                    self.level[self.to[e]] = self.level[u] + 1;
                    q.push_back(self.to[e]);
                }
            }
        }
        self.level[t] >= 0
    }

    fn dfs(&mut self, u: usize, t: usize, f: f64) -> f64 {
        if u == t {
            return f;
        }
        while self.it[u] < self.head[u].len() {
            let e = self.head[u][self.it[u]];
            let v = self.to[e];
            if self.cap[e] > EPS && self.level[v] == self.level[u] + 1 {
                let d = self.dfs(v, t, f.min(self.cap[e]));
                if d > EPS {
                    self.cap[e] -= d;
                    self.cap[e ^ 1] += d;
                    return d;
                }
            }
            self.it[u] += 1;
        }
        0.0
    }

    fn max_flow(&mut self, s: usize, t: usize) -> f64 {
        let mut flow = 0.0;
        while self.bfs(s, t) {
            self.it.iter_mut().for_each(|x| *x = 0);
            loop {
                let f = self.dfs(s, t, f64::INFINITY);
                if f <= EPS {
                    break;
                }
                flow += f;
            }
        }
        flow
    }

    /// Residual adjacency: v reachable from u iff there is a path of edges
    /// with residual capacity > EPS.
    fn residual_adj(&self) -> Vec<Vec<usize>> {
        let mut adj = vec![Vec::new(); self.n];
        for (u, slot) in adj.iter_mut().enumerate() {
            for &e in &self.head[u] {
                if self.cap[e] > EPS {
                    slot.push(self.to[e]);
                }
            }
        }
        adj
    }
}

// Literal node ids: x_i -> 2*i, x̄_i -> 2*i+1. Source (true) and sink (false)
// follow the literal block.
#[inline]
fn pos(i: usize) -> usize {
    2 * i
}
#[inline]
fn neg(i: usize) -> usize {
    2 * i + 1
}
#[inline]
fn comp(lit: usize) -> usize {
    lit ^ 1
}

/// Returns per-variable weak-persistency fixings via roof duality:
/// `Some(v)` = provably fixable to v (a global optimum has x_i = v),
/// `None` = left free. Deterministic. Supports arbitrary QUBO instances.
pub fn roof_duality_persistencies(model: &QuboModel) -> Vec<Option<i8>> {
    let n = model.num_vars;
    if n == 0 {
        return Vec::new();
    }
    let src = 2 * n; // literal "T" (true / constant 1)
    let sink = 2 * n + 1; // literal "F" (false / 0) = complement of src
    let node_comp = |lit: usize| -> usize {
        if lit == src {
            sink
        } else if lit == sink {
            src
        } else {
            comp(lit)
        }
    };

    let mut g = Dinic::new(2 * n + 2);

    // Scale-normalize: persistency is invariant under E → E/scale (positive
    // constant preserves the optima set), and it keeps all capacities O(1)
    // so the absolute residual epsilon is meaningful at any coefficient
    // magnitude (fixes float noise on extreme-magnitude instances).
    let mut scale = 0.0f64;
    for i in 0..n {
        scale = scale.max(model.linear[i].abs());
        for (_, w) in model.quadratic.get_row(i) {
            scale = scale.max(w.abs());
        }
    }
    let scale = if scale > 0.0 { scale } else { 1.0 };
    let lin = |i: usize| model.linear[i] / scale;

    // add_quad(u, v, c): posiform term c·u·v (c ≥ 0) → arcs u→v̄, v→ū.
    let add_quad = |g: &mut Dinic, u: usize, v: usize, c: f64| {
        if c > EPS {
            g.add_edge(u, node_comp(v), c);
            g.add_edge(v, node_comp(u), c);
        }
    };
    // add_linear(u, c): penalty c when literal u = 1 (c any sign).
    // c ≥ 0 → term c·(u·T): arcs u→F, T→ū. c < 0 → const += c, |c|·ū.
    let add_linear = |g: &mut Dinic, u: usize, c: f64| {
        if c >= 0.0 {
            if c > EPS {
                g.add_edge(u, sink, c);
                g.add_edge(src, node_comp(u), c);
            }
        } else {
            let cc = -c;
            let ubar = node_comp(u);
            g.add_edge(ubar, sink, cc);
            g.add_edge(src, u, cc);
        }
    };

    // Linear terms.
    for i in 0..n {
        add_linear(&mut g, pos(i), lin(i));
    }
    // Quadratic terms (upper triangle; model stores symmetric halves whose
    // both-1 contribution sums to the stored value — see calculate_total_energy).
    for i in 0..n {
        for (j, w) in model.quadratic.get_row(i) {
            if i < j {
                let w = w / scale;
                if w >= 0.0 {
                    add_quad(&mut g, pos(i), pos(j), w);
                } else {
                    // b<0: b·x_i·x_j = (b·x_i, linear) + |b|·x_i·x̄_j.
                    add_linear(&mut g, pos(i), w);
                    add_quad(&mut g, pos(i), neg(j), -w);
                }
            }
        }
    }

    g.max_flow(src, sink);
    let adj = g.residual_adj();
    let radj = reverse(&adj);

    // Roof-duality persistency by the residual MIN-CUT sides (Boros-Hammer
    // 2002). In the residual graph:
    //   S = {nodes reachable from source} — the definite source side across
    //       every minimum cut,
    //   T = {nodes that can reach sink}   — the definite sink side.
    // A literal in neither set is ambiguous. Variable i is persistent iff its
    // literal x_i and complement x̄_i land on definite OPPOSITE sides; then
    // its value is fixed by which side x_i is on. Literals that are both
    // ambiguous (e.g. an isolated variable, all-zero model) stay free.
    // Value polarity (x_i = 1 when x_i is on the source side) is
    // regression-locked by the exhaustive strong-persistency test.
    let in_s = bfs_reach(&adj, src); // reachable from source
    let in_t = bfs_reach(&radj, sink); // can reach sink
    let mut out = vec![None; n];
    for (i, slot) in out.iter_mut().enumerate() {
        let (p, q) = (pos(i), neg(i));
        *slot = if in_s[p] && in_t[q] {
            Some(1) // x_i on source side
        } else if in_t[p] && in_s[q] {
            Some(0) // x_i on sink side
        } else {
            None
        };
    }
    out
}

/// Reverse adjacency (edge directions flipped).
fn reverse(adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut r = vec![Vec::new(); adj.len()];
    for (u, nbrs) in adj.iter().enumerate() {
        for &v in nbrs {
            r[v].push(u);
        }
    }
    r
}

fn bfs_reach(adj: &[Vec<usize>], s: usize) -> Vec<bool> {
    let mut seen = vec![false; adj.len()];
    let mut q = std::collections::VecDeque::new();
    seen[s] = true;
    q.push_back(s);
    while let Some(u) = q.pop_front() {
        for &v in &adj[u] {
            if !seen[v] {
                seen[v] = true;
                q.push_back(v);
            }
        }
    }
    seen
}
