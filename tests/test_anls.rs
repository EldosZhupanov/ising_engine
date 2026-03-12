use ising_engine::core::anls::AnlsPreconditioner;

#[test]
fn test_anls_factorization() {
    let q_matrix = vec![
        vec![10.0, 2.0, 1.0, 5.0],
        vec![2.0, 10.0, 4.0, 1.0],
        vec![1.0, 4.0, 10.0, 2.0],
        vec![5.0, 1.0, 2.0, 10.0],
    ];

    let preconditioner = AnlsPreconditioner::new(2, 500, 1e-4);
    
    let (w, h) = preconditioner.factorize(&q_matrix);

    assert_eq!(w.len(), 4);
    assert_eq!(w[0].len(), 2);
    assert_eq!(h.len(), 2);
    assert_eq!(h[0].len(), 4);

    let mut q_approx = vec![vec![0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..2 {
                q_approx[i][j] += w[i][k] * h[k][j];
            }
        }
    }

    let mut error: f64 = 0.0;
    for i in 0..4 {
        for j in 0..4 {
            let diff = q_matrix[i][j] - q_approx[i][j];
            error += diff * diff;
        }
    }
    
    let frobenius_norm = error.sqrt();
    println!("Frobenius Norm Error (K=2): {}", frobenius_norm);
    
    assert!(frobenius_norm < 15.0, "Factorization error too high!");
}
