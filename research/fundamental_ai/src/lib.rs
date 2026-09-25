//! Fundamental AI Research Module: Higher-Order Tensor Energy Dynamics.
//!
//! Investigates whether higher-order interactions (p >= 3) and energy relaxation
//! provide a viable computational primitive for neural memory and reasoning.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]

pub mod analytic_ebm;
pub mod baselines;
pub mod datasets;
pub mod dynamics;
pub mod experiment;
pub mod learned_energy;
pub mod learning;
pub mod models;
pub mod synchronization_audit;
pub mod types;

pub use analytic_ebm::*;
pub use baselines::*;
pub use datasets::*;
pub use dynamics::*;
pub use experiment::*;
pub use learned_energy::*;
pub use learning::*;
pub use models::*;
pub use synchronization_audit::*;
pub use types::*;
