use rand::Rng;

pub struct AnlsPreconditioner {
    pub target_rank: usize,
    pub max_iterations: usize,
    pub tolerance: f64,
}

impl AnlsPreconditioner {
    pub fn new(target_rank: usize, max_iterations: usize, tolerance: f64) -> Self {
        Self {
            target_rank,
            max_iterations,
            tolerance,
        }
    }

    /// Factorize a dense N x N matrix Q into W (N x K) and H (K x N)
    /// Using Multiplicative Update Rules for NMF
    pub fn factorize(&self, q: &[Vec<f64>]) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
        let n = q.len();
        let k = self.target_rank;
        let mut rng = rand::thread_rng();

        // Initialize W (N x K) and H (K x N) with positive random values
        let mut w = vec![vec![0.0; k]; n];
        let mut h = vec![vec![0.0; n]; k];

        for row in w.iter_mut().take(n) {
            for val in row.iter_mut().take(k) { *val = rng.gen_range(0.1..1.0); }
        }
        for row in h.iter_mut().take(k) {
            for val in row.iter_mut().take(n) { *val = rng.gen_range(0.1..1.0); }
        }

        // Shift Q to be non-negative if necessary (NMF requirement)
        let mut min_val = 0.0;
        for r in q.iter() {
            for &val in r.iter() {
                if val < min_val { min_val = val; }
            }
        }
        let shift = if min_val < 0.0 { min_val.abs() + 1e-5 } else { 0.0 };

        let mut v = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                v[i][j] = q[i][j] + shift;
            }
        }

        // Multiplicative Update Loop
        for _ in 0..self.max_iterations {
            // Update H
            let wt_v = Self::multiply_transpose_w(&w, &v, n, k);
            let wt_w_h = Self::multiply(&Self::multiply_transpose_w_w(&w, n, k), &h, k, n);
            for i in 0..k {
                for j in 0..n {
                    if wt_w_h[i][j] > 1e-9 {
                        h[i][j] *= wt_v[i][j] / wt_w_h[i][j];
                    }
                }
            }

            // Update W
            let v_ht = Self::multiply_h_transpose(&v, &h, n, k);
            let w_h_ht = Self::multiply(&w, &Self::multiply_h_ht(&h, k, n), n, k);
            for i in 0..n {
                for j in 0..k {
                    if w_h_ht[i][j] > 1e-9 {
                        w[i][j] *= v_ht[i][j] / w_h_ht[i][j];
                    }
                }
            }
        }

        // We return W and H (approximating V = Q + shift).
        // In the solver integration, we will compute Q_approx = W*H - shift.
        (w, h)
    }

    fn multiply(a: &[Vec<f64>], b: &[Vec<f64>], rows_a: usize, cols_b: usize) -> Vec<Vec<f64>> {
        let cols_a = a[0].len();
        let mut res = vec![vec![0.0; cols_b]; rows_a];
        for i in 0..rows_a {
            for j in 0..cols_b {
                let mut sum = 0.0;
                for l in 0..cols_a { sum += a[i][l] * b[l][j]; }
                res[i][j] = sum;
            }
        }
        res
    }

    fn multiply_transpose_w(w: &[Vec<f64>], v: &[Vec<f64>], n: usize, k: usize) -> Vec<Vec<f64>> {
        let mut res = vec![vec![0.0; n]; k];
        for i in 0..k {
            for j in 0..n {
                let mut sum = 0.0;
                for l in 0..n { sum += w[l][i] * v[l][j]; }
                res[i][j] = sum;
            }
        }
        res
    }

    fn multiply_transpose_w_w(w: &[Vec<f64>], n: usize, k: usize) -> Vec<Vec<f64>> {
        let mut res = vec![vec![0.0; k]; k];
        for i in 0..k {
            for j in 0..k {
                let mut sum = 0.0;
                for l in 0..n { sum += w[l][i] * w[l][j]; }
                res[i][j] = sum;
            }
        }
        res
    }

    fn multiply_h_transpose(v: &[Vec<f64>], h: &[Vec<f64>], n: usize, k: usize) -> Vec<Vec<f64>> {
        let mut res = vec![vec![0.0; k]; n];
        for i in 0..n {
            for j in 0..k {
                let mut sum = 0.0;
                for l in 0..n { sum += v[i][l] * h[j][l]; }
                res[i][j] = sum;
            }
        }
        res
    }

    fn multiply_h_ht(h: &[Vec<f64>], k: usize, n: usize) -> Vec<Vec<f64>> {
        let mut res = vec![vec![0.0; k]; k];
        for i in 0..k {
            for j in 0..k {
                let mut sum = 0.0;
                for l in 0..n { sum += h[i][l] * h[j][l]; }
                res[i][j] = sum;
            }
        }
        res
    }
}
