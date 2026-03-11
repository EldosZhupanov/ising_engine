/// State of a single replica in the Parallel Tempering ensemble.
#[derive(Clone)]
pub struct Replica {
    pub state: Vec<i8>,
    pub temp: f64,
    pub energy: f64,
}

/// Builds a bitmap for O(1) clamped-variable lookup in hot loops.
///
/// Replaces the original `clamped.iter().any(|&(i, _)| i == var_idx)` O(n) scan.
pub fn build_clamped_set(num_vars: usize, clamped: &[(usize, i8)]) -> Vec<bool> {
    let mut set = vec![false; num_vars];
    for &(idx, _) in clamped {
        set[idx] = true;
    }
    set
}
