//! # Ising Engine
//!
//! `ising_engine` is a high-performance combinatorial optimization core
//! designed specifically for modeling and solving complex AI-assisted decision problems.
//! It frames problems in the form of Quadratic Unconstrained Binary Optimization (QUBO)
//! models and strictly minimizes energy via advanced Replica Exchange (Parallel Tempering).
//!
//! ## Features
//!
//! - **Parallel Tempering:** Native multi-threaded replica exchange algorithm avoiding local minima.
//! - **Adaptive & Cluster Solvers:** Variations on standard simulated annealing.
//! - **Digital Logic Compiler:** Convert digital logic (AND, XOR, Adders) directly into QUBO graph topologies.
//!
//! ## Example
//!
//! ```rust
//! use ising_engine::core::{CsrMatrix, QuboModel};
//! use ising_engine::solver::parallel_tempering::ParallelTemperingSolver;
//!
//! let num_vars = 10;
//! let model = QuboModel {
//!     num_vars,
//!     linear: vec![1.0; num_vars],
//!     quadratic: CsrMatrix::empty(num_vars),
//! };
//!
//! let solver = ParallelTemperingSolver {
//!     num_replicas: 4,
//!     temp_max: 10.0,
//!     temp_min: 0.1,
//!     sweeps_per_exchange: 100,
//!     total_exchanges: 50,
//!     seed: None,
//! };
//!
//! let solution = solver.solve(&model, &[]);
//! ```

#![deny(warnings)]
#![deny(clippy::all)]
pub mod compiler;
pub mod core;
pub mod solver;
