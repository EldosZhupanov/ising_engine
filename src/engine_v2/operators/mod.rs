//! Operator library (Constitution §8, Blueprint Step 5). Each operator is one
//! physical law implemented against `SpinState` only — the LEGO bricks the
//! Runtime schedules by CAPABILITY. This is where the library grows; the
//! framework above never changes as bricks are added.
//!
//! The library now spans the major families of physical/optimization principles,
//! each a distinct passport the Decision and Evolution engines select by
//! CAPABILITY:
//!
//!  - Thermal local:   `GibbsColorSweep`, `MetropolisSweep`, `HistoryFieldSweep`
//!  - Greedy local:    `GreedyDescent`, `SteepestDescent`
//!  - Exploration:     `RandomFlipSweep`, `RandomRestartWorst`
//!  - Extremal:        `ExtremalOptimization`
//!  - Replica/cluster: `ReplicaExchange`, `HoudayerClusterMove`, ICM
//!    (`IsoenergeticClusterMove`)
//!  - Population:      `PopulationResample`, `EliteBroadcast`
//!
//! Every operator is written against `SpinState` only and cross-validated to
//! zero ledger drift against `ReferenceState` on both backends. Further physics
//! (belief propagation, simulated bifurcation, renormalization) drops in behind
//! the same contract with zero framework changes.

pub mod cluster;
pub mod ensemble_thermostat;
pub mod extremal_metropolis;
pub mod extremal_optimization;
pub mod gibbs_color_sweep;
pub mod greedy_descent;
pub mod history_field;
pub mod metropolis_sweep;
pub mod move_synthesis;
pub mod path_relink_sweep;
pub mod population_annealing;
pub mod random_flip_sweep;
pub mod replica_exchange;
pub mod steepest_descent;
pub mod tabu_sweep;

pub use cluster::{HoudayerClusterMove, IsoenergeticClusterMove};
pub use ensemble_thermostat::EnsembleThermostat;
pub use extremal_metropolis::ExtremalMetropolis;
pub use extremal_optimization::ExtremalOptimization;
pub use gibbs_color_sweep::GibbsColorSweep;
pub use greedy_descent::GreedyDescent;
pub use history_field::HistoryFieldSweep;
pub use metropolis_sweep::MetropolisSweep;
pub use move_synthesis::{MoveSynthesizer, Source as SynthSource};
pub use path_relink_sweep::PathRelinkSweep;
pub use population_annealing::{EliteBroadcast, PopulationResample, RandomRestartWorst};
pub use random_flip_sweep::RandomFlipSweep;
pub use replica_exchange::ReplicaExchange;
pub use steepest_descent::SteepestDescent;
pub use tabu_sweep::TabuSweep;
