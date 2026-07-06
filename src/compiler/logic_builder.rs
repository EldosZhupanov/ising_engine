use crate::core::{CsrMatrix, QuboModel};

#[derive(Clone, Copy)]
pub struct Multiplier2x2 {
    pub a: [usize; 2],
    pub b: [usize; 2],
    pub p: [usize; 4],
}

pub struct LogicBuilder {
    pub num_vars: usize,
    /// Accumulated constant of the penalty polynomial (e.g. +1 per NOT).
    pub constant: f64,
    pub linear: Vec<f64>,
    pub quadratic: Vec<(usize, usize, f64)>,
    pub cubic: Vec<(usize, usize, usize, f64)>,
    pub quartic: Vec<(usize, usize, usize, usize, f64)>,
}

impl LogicBuilder {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            num_vars: 0,
            constant: 0.0,
            linear: Vec::new(),
            quadratic: Vec::new(),
            cubic: Vec::new(),
            quartic: Vec::new(),
        }
    }

    pub fn add_var(&mut self) -> usize {
        let idx = self.num_vars;
        self.num_vars += 1;
        self.linear.push(0.0);
        idx
    }

    fn add_quad(&mut self, u: usize, v: usize, w: f64) {
        if u == v {
            self.linear[u] += w;
        } else {
            let (min, max) = if u < v { (u, v) } else { (v, u) };
            self.quadratic.push((min, max, w));
        }
    }

    pub fn add_edge3(&mut self, u: usize, v: usize, x: usize, w: f64) {
        self.cubic.push((u, v, x, w));
    }

    pub fn add_edge4(&mut self, u: usize, v: usize, x: usize, y: usize, w: f64) {
        self.quartic.push((u, v, x, y, w));
    }

    pub fn add_not_gate(&mut self, x: usize, y: usize) {
        // Penalty 2xy − x − y + 1 ≥ 0, equality iff y = ¬x (Boros-Hammer
        // form). The +1 keeps the satisfied-state energy at exactly zero.
        self.add_quad(x, y, 2.0);
        self.linear[x] -= 1.0;
        self.linear[y] -= 1.0;
        self.constant += 1.0;
    }

    pub fn add_or_gate(&mut self, a: usize, b: usize, z: usize) {
        self.add_quad(a, b, 1.0);
        self.add_quad(a, z, -2.0);
        self.add_quad(b, z, -2.0);
        self.linear[a] += 1.0;
        self.linear[b] += 1.0;
        self.linear[z] += 1.0;
    }

    // Fixed MUX penalty logic: z = (s AND a) OR (NOT(s) AND b)
    // To keep it simple and robust, we build it via existing primitive gates.
    // This uses auxiliary variables, which is perfectly fine for QUBO!
    pub fn add_mux_gate(&mut self, s: usize, a: usize, b: usize, z: usize) {
        let not_s = self.add_var();
        let path_a = self.add_var();
        let path_b = self.add_var();

        self.add_not_gate(s, not_s);
        self.add_and_gate(s, a, path_a);
        self.add_and_gate(not_s, b, path_b);
        self.add_or_gate(path_a, path_b, z);
    }

    pub fn add_equal_gate(&mut self, a: usize, b: usize, eq: usize) {
        let xor_out = self.add_var();
        let aux = self.add_var();
        self.add_xor_gate(a, b, xor_out, aux);
        self.add_not_gate(xor_out, eq);
    }

    pub fn add_and_gate(&mut self, a: usize, b: usize, z: usize) {
        self.add_quad(a, b, 1.0);
        self.add_quad(a, z, -2.0);
        self.add_quad(b, z, -2.0);
        self.linear[z] += 3.0;
    }

    pub fn add_xor_gate(&mut self, x: usize, y: usize, z: usize, w: usize) {
        self.add_quad(x, y, 2.0);
        self.add_quad(x, z, -2.0);
        self.add_quad(y, z, -2.0);
        self.linear[x] += 1.0;
        self.linear[y] += 1.0;
        self.linear[z] += 1.0;
        self.add_quad(w, x, -4.0);
        self.add_quad(w, y, -4.0);
        self.add_quad(w, z, 4.0);
        self.linear[w] += 4.0;
    }

    pub fn add_half_adder(&mut self, a: usize, b: usize, sum: usize, carry: usize) {
        let aux = self.add_var();
        self.add_xor_gate(a, b, sum, aux);
        self.add_and_gate(a, b, carry);
    }

    pub fn add_full_adder(&mut self, a: usize, b: usize, c_in: usize, sum: usize, c_out: usize) {
        let sum_half = self.add_var();
        let carry1 = self.add_var();
        let carry2 = self.add_var();
        self.add_half_adder(a, b, sum_half, carry1);
        self.add_half_adder(sum_half, c_in, sum, carry2);
        self.add_or_gate(carry1, carry2, c_out);
    }

    pub fn add_multiplier_2x2(&mut self, m: Multiplier2x2) {
        let a = m.a;
        let b = m.b;
        let p = m.p;
        self.add_and_gate(a[0], b[0], p[0]);
        let p01 = self.add_var();
        self.add_and_gate(a[0], b[1], p01);
        let p10 = self.add_var();
        self.add_and_gate(a[1], b[0], p10);
        let p11 = self.add_var();
        self.add_and_gate(a[1], b[1], p11);
        let carry1 = self.add_var();
        self.add_half_adder(p01, p10, p[1], carry1);
        self.add_half_adder(p11, carry1, p[2], p[3]);
    }

    pub fn build(self) -> QuboModel {
        let mut row_edges: Vec<Vec<(usize, f64)>> = vec![vec![]; self.num_vars];
        for (u, v, w) in self.quadratic {
            row_edges[u].push((v, w));
            row_edges[v].push((u, w));
        }

        let mut values = Vec::new();
        let mut col_indices = Vec::new();
        let mut row_offsets = vec![0];

        for edges in row_edges.iter_mut() {
            edges.sort_by_key(|&(v, _)| v);
            let mut merged = Vec::new();
            for &(v, w) in edges.iter() {
                if let Some(&mut (last_v, ref mut last_w)) = merged.last_mut() {
                    if last_v == v {
                        *last_w += w;
                    } else {
                        merged.push((v, w));
                    }
                } else {
                    merged.push((v, w));
                }
            }
            for (v, w) in merged {
                if w.abs() > 1e-9 {
                    col_indices.push(v);
                    values.push(w);
                }
            }
            row_offsets.push(col_indices.len());
        }

        QuboModel {
            num_vars: self.num_vars,
            linear: self.linear,
            quadratic: CsrMatrix {
                values,
                col_indices,
                row_offsets,
            },
            energy_offset: self.constant,
        }
    }

    pub fn build_hubo(self) -> crate::core::hubo::HuboModel {
        let mut model = crate::core::hubo::HuboModel::new(self.num_vars);
        model.linear = self.linear;

        for (u, v, w) in self.quadratic {
            model.edges2[u].push(crate::core::hubo::Edge2 { j: v, weight: w });
            model.edges2[v].push(crate::core::hubo::Edge2 { j: u, weight: w });
        }

        for (u, v, x, w) in self.cubic {
            model.edges3[u].push(crate::core::hubo::Edge3 {
                j: v,
                k: x,
                weight: w,
            });
            model.edges3[v].push(crate::core::hubo::Edge3 {
                j: u,
                k: x,
                weight: w,
            });
            model.edges3[x].push(crate::core::hubo::Edge3 {
                j: u,
                k: v,
                weight: w,
            });
        }

        for (u, v, x, y, w) in self.quartic {
            model.edges4[u].push(crate::core::hubo::Edge4 {
                j: v,
                k: x,
                l: y,
                weight: w,
            });
            model.edges4[v].push(crate::core::hubo::Edge4 {
                j: u,
                k: x,
                l: y,
                weight: w,
            });
            model.edges4[x].push(crate::core::hubo::Edge4 {
                j: u,
                k: v,
                l: y,
                weight: w,
            });
            model.edges4[y].push(crate::core::hubo::Edge4 {
                j: u,
                k: v,
                l: x,
                weight: w,
            });
        }

        model
    }
}
