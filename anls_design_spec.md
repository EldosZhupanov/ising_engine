🦾 [Superpowers] Socratic Design Spec: ANLS (Alternating Non-negative Least Squares) Preconditioner
## Objective
Build a fully functional ANLS module to perform Low-Rank Matrix Factorization (NMF) on giant QUBO graphs before feeding them into the Ising Engine.

## Mathematical Architecture
1. **Input:** A large N x N QUBO weight matrix Q (dense or sparse).
2. **Target:** Factorize Q ≈ W * H, where W is N x K, and H is K x N. K is the rank (K << N).
3. **Algorithm:** We alternate solving W and H using Multiplicative Update Rules (a fast variant of ANLS for non-negative matrices). Since QUBO matrices can have negative weights, we will split Q into Q_pos and Q_neg, or shift the matrix to be strictly non-negative, factorize, and shift back.

## Engineering Pipeline
1. **Module Creation:** Create `src/core/anls.rs`.
2. **Struct `AnlsPreconditioner`:** Will hold hyperparameters: target_rank (K), max_iterations, tolerance.
3. **TDD:** Write red tests in `tests/test_anls.rs` to prove Q ≈ W*H on a 100x100 matrix.
4. **Integration:** Allow `QuboModel` to be compressed via `.compress_with_anls(k)`.
