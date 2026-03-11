use crate::core::{CsrMatrix, QuboModel};

/// QUBO Logic Compiler — translates digital logic gates into QUBO penalty functions.
///
/// Each gate adds penalty terms to the Hamiltonian such that the ground state
/// (energy = 0) corresponds to the correct logical operation.
pub struct Multiplier2x2 {
    pub a: [usize; 2],
    pub b: [usize; 2],
    pub p: [usize; 4],
}

pub struct LogicBuilder {
    linear: Vec<f64>,
    quadratic_edges: Vec<(usize, usize, f64)>,
}

impl LogicBuilder {
    pub fn new() -> Self {
        Self {
            linear: vec![],
            quadratic_edges: vec![],
        }
    }

    /// Registers a new binary variable (spin) in the system.
    pub fn add_var(&mut self) -> usize {
        let id = self.linear.len();
        self.linear.push(0.0);
        id
    }

    /// Adds a bidirectional edge to the quadratic matrix.
    fn add_edge(&mut self, u: usize, v: usize, weight: f64) {
        self.quadratic_edges.push((u, v, weight));
        self.quadratic_edges.push((v, u, weight));
    }

    /// Penalty function for Z = A AND B
    /// P = AB - 2AZ - 2BZ + 3Z
    pub fn add_and_gate(&mut self, a: usize, b: usize, z: usize) {
        self.linear[z] += 3.0;
        self.add_edge(a, b, 1.0);
        self.add_edge(a, z, -2.0);
        self.add_edge(b, z, -2.0);
    }

    /// Penalty function for Z = X XOR Y (uses ancilla bit W)
    /// P = X + Y + Z + 4W + 2XY - 2XZ - 2YZ - 4XW - 4YW + 4ZW
    pub fn add_xor_gate(&mut self, x: usize, y: usize, z: usize, w: usize) {
        self.linear[x] += 1.0;
        self.linear[y] += 1.0;
        self.linear[z] += 1.0;
        self.linear[w] += 4.0;
        self.add_edge(x, y, 2.0);
        self.add_edge(x, z, -2.0);
        self.add_edge(y, z, -2.0);
        self.add_edge(x, w, -4.0);
        self.add_edge(y, w, -4.0);
        self.add_edge(z, w, 4.0);
    }

    /// Half adder: sum = A XOR B, carry = A AND B.
    /// Automatically allocates an ancilla variable for the XOR gate.
    pub fn add_half_adder(&mut self, a: usize, b: usize, sum: usize, carry: usize) {
        let ancilla = self.add_var();
        self.add_xor_gate(a, b, sum, ancilla);
        self.add_and_gate(a, b, carry);
    }

    /// Full adder: sum = A XOR B XOR Cin, carry_out = majority(A, B, Cin).
    /// Implemented as two half adders + XOR for carry propagation.
    pub fn add_full_adder(&mut self, a: usize, b: usize, c_in: usize, sum: usize, c_out: usize) {
        let sum1 = self.add_var();
        let carry1 = self.add_var();
        let carry2 = self.add_var();

        self.add_half_adder(a, b, sum1, carry1);
        self.add_half_adder(sum1, c_in, sum, carry2);

        let w = self.add_var();
        self.add_xor_gate(carry1, carry2, c_out, w);
    }

    /// 2x2 binary multiplier: `P[3:0] = A[1:0] * B[1:0]`.
    /// Builds the multiplication circuit from AND gates and half adders.
    pub fn add_multiplier_2x2(&mut self, m: Multiplier2x2) {
        self.add_and_gate(m.a[0], m.b[0], m.p[0]);
        let m10 = self.add_var();
        self.add_and_gate(m.a[1], m.b[0], m10);
        let m01 = self.add_var();
        self.add_and_gate(m.a[0], m.b[1], m01);
        let c1 = self.add_var();
        self.add_half_adder(m10, m01, m.p[1], c1);
        let m11 = self.add_var();
        self.add_and_gate(m.a[1], m.b[1], m11);
        self.add_half_adder(m11, c1, m.p[2], m.p[3]);
    }

    /// Compiles all accumulated gates into an optimized CSR-backed QuboModel.
    /// Merges duplicate edges and eliminates zero-weight entries.
    pub fn build(self) -> QuboModel {
        let n = self.linear.len();
        let mut row_edges: Vec<Vec<(usize, f64)>> = vec![vec![]; n];

        for (u, v, w) in self.quadratic_edges {
            row_edges[u].push((v, w));
        }

        let mut values = vec![];
        let mut col_indices = vec![];
        let mut row_offsets = vec![0];

        for mut edges in row_edges {
            // Group and merge duplicate edges (when gates share a wire)
            edges.sort_by_key(|&(v, _)| v);
            let mut merged = vec![];
            for (v, w) in edges {
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
                if w != 0.0 {
                    col_indices.push(v);
                    values.push(w);
                }
            }
            row_offsets.push(col_indices.len());
        }

        QuboModel {
            num_vars: n,
            linear: self.linear,
            quadratic: CsrMatrix {
                values,
                col_indices,
                row_offsets,
            },
        }
    }
}

impl Default for LogicBuilder {
    fn default() -> Self {
        Self::new()
    }
}
