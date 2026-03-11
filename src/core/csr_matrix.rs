/// Compressed Sparse Row (CSR) matrix for scalable QUBO representation.
///
/// Stores the quadratic interaction weights J_ij of the QUBO Hamiltonian
/// in a memory-efficient sparse format.
#[derive(Debug, Clone)]
pub struct CsrMatrix {
    pub values: Vec<f64>,
    pub col_indices: Vec<usize>,
    pub row_offsets: Vec<usize>,
}

impl CsrMatrix {
    /// Returns an iterator over (column_index, weight) pairs for the given row.
    pub fn get_row(&self, row: usize) -> impl Iterator<Item = (usize, f64)> + '_ {
        let start = self.row_offsets[row];
        let end = self.row_offsets[row + 1];
        self.col_indices[start..end]
            .iter()
            .copied()
            .zip(self.values[start..end].iter().copied())
    }

    /// Creates an empty CSR matrix (no quadratic interactions).
    pub fn empty(num_vars: usize) -> Self {
        Self {
            values: vec![],
            col_indices: vec![],
            row_offsets: vec![0; num_vars + 1],
        }
    }
}
