use ising_engine::core::lattice::{aardal_diophantine_reduction, LatticeBasis};

#[test]
fn test_lll_orthogonality_improvement() {
    // Highly skewed 4D lattice
    let basis = vec![
        vec![10, 11, 12, 13],
        vec![14, 15, 16, 18],
        vec![20, 21, 23, 24],
        vec![25, 27, 28, 30],
    ];
    let mut lattice = LatticeBasis::new(basis).unwrap();

    let initial_norm_prod: f64 = (0..4)
        .map(|i| (lattice.vector_norm_sq(i) as f64).sqrt())
        .product();

    lattice.lll(0.75).unwrap();

    let reduced_norm_prod: f64 = (0..4)
        .map(|i| (lattice.vector_norm_sq(i) as f64).sqrt())
        .product();

    // LLL must strictly reduce the Hadamard orthogonality product
    assert!(reduced_norm_prod < initial_norm_prod);
}

#[test]
fn test_aardal_extracted_vectors_satisfy_equations() {
    // Ground truth hidden boolean vector: x = [1, 0, 1, 1, 0, 1]
    let hidden_x = [1i64, 0, 1, 1, 0, 1];
    let n = hidden_x.len();

    // Construct 2 random constraints
    let matrix = vec![vec![14, 25, 31, 42, 19, 53], vec![71, 18, 29, 34, 85, 12]];

    let mut rhs = vec![0i64; 2];
    for (i, row) in matrix.iter().enumerate() {
        for j in 0..n {
            rhs[i] += row[j] * hidden_x[j];
        }
    }

    // Run Aardal-Hurkens-Lenstra reduction
    let res = aardal_diophantine_reduction(&matrix, &rhs, 10_000, 10).unwrap();

    assert!(res.particular_solution.is_some() || !res.kernel_basis.is_empty());

    // Check the extracted integer solution when one was found.
    if let Some(ref particular) = res.particular_solution {
        for (i, row) in matrix.iter().enumerate() {
            let mut sum = 0i64;
            for j in 0..n {
                sum += row[j] * particular[j];
            }
            assert_eq!(
                sum, rhs[i],
                "Particular solution must satisfy linear system"
            );
        }
    }

    // Any kernel vector must satisfy A * v = 0
    for v in &res.kernel_basis {
        for row in &matrix {
            let mut sum = 0i64;
            for j in 0..n {
                sum += row[j] * v[j];
            }
            assert_eq!(sum, 0, "Kernel vector must satisfy A * v = 0");
        }
    }
}

#[test]
fn test_kernel_qubo_energy_equivalence() {
    use ising_engine::core::lattice::build_kernel_qubo;

    // Linear system with 3 variables and 1 constraint:
    // 2 x_0 + 3 x_1 + 5 x_2 = 10
    // Particular integer solution: x_0 = [5, 0, 0] -> 2(5) + 3(0) + 5(0) = 10.
    let x0 = vec![5i64, 0, 0];
    // Kernel vectors:
    // v_0 = [3, -2, 0] -> 2(3) + 3(-2) = 0
    // v_1 = [5, 0, -2] -> 2(5) + 5(-2) = 0
    let kernel = vec![vec![3i64, -2, 0], vec![5i64, 0, -2]];

    let r = kernel.len();
    let lambda_centers = vec![0i64; r];
    let bits_per_coord = 3;

    let mapping = build_kernel_qubo(&x0, &kernel, &lambda_centers, bits_per_coord);

    // Test across several random and corner binary states
    let test_states = vec![
        vec![0i8; mapping.num_spins],
        vec![1i8; mapping.num_spins],
        vec![1, 0, 1, 0, 1, 0],
        vec![0, 1, 0, 1, 0, 1],
    ];

    for state in test_states {
        let qubo_e = mapping.model.calculate_total_energy(&state);

        let (_lambda, x) = mapping.decode(&x0, &kernel, &state);
        let mut direct_e = 0.0;
        for &xj in &x {
            direct_e += (xj * (xj - 1)) as f64;
        }

        assert!(
            (qubo_e - direct_e).abs() < 1e-9,
            "QUBO energy {} must match direct integer energy {}",
            qubo_e,
            direct_e
        );
    }
}

#[test]
fn test_babai_nearest_plane_cvp() {
    let basis = vec![vec![10, 0], vec![0, 10]];
    let mut lattice = LatticeBasis::new(basis).unwrap();
    lattice.lll(0.75).unwrap();

    let target = vec![19.2, 31.8];
    let c = lattice.babai_nearest_plane(&target);
    assert_eq!(c, vec![2, 3]); // Closest lattice point is (20, 30) -> c = [2, 3]
}
