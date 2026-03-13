use super::csr_matrix::CsrMatrix;

/// QUBO (Quadratic Unconstrained Binary Optimization) model.
///
/// Represents the Hamiltonian: H(x) = Σ_i h_i·x_i + Σ_{i<j} J_ij·x_i·x_j
/// where x_i ∈ {0, 1} are binary variables.
pub struct QuboModel {
    pub num_vars: usize,
    pub linear: Vec<f64>,
    pub quadratic: CsrMatrix,
}

impl QuboModel {
    /// Computes the total energy of the system for the given binary state.
    ///
    /// The 0.5 factor accounts for the symmetric CSR matrix storing both (i,j) and (j,i).
    pub fn calculate_total_energy(&self, state: &[i8]) -> f64 {
        let mut energy = 0.0;
        for i in 0..self.num_vars {
            if state[i] == 1 {
                energy += self.linear[i];
                for (j, weight) in self.quadratic.get_row(i) {
                    if state[j] == 1 {
                        energy += weight * 0.5;
                    }
                }
            }
        }
        energy
    }
}
