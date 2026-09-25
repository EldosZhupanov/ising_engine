//! Inference dynamics for tensor energy networks: D0 through D5.

use crate::models::EnergyModel;
use crate::types::{ContinuousState, SpinState};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Outcome and performance telemetry of an inference relaxation trajectory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicsResult {
    pub final_state: SpinState,
    pub initial_energy: f64,
    pub final_energy: f64,
    pub energy_trajectory: Vec<f64>,
    pub sweeps: usize,
    pub flips: usize,
    pub converged: bool,
    pub oscillation_detected: bool,
    pub wall_time_us: u128,
}

/// D0: Sequential greedy coordinate descent.
/// Evaluates each spin sequentially; flips if Delta E = 2 * s_i * h_i^eff < 0.
/// Guarantees monotonic energy decrease until an exact local attractor is reached.
pub fn greedy_descent(
    model: &dyn EnergyModel,
    initial_state: &SpinState,
    max_sweeps: usize,
) -> DynamicsResult {
    let start_time = Instant::now();
    let n = model.num_spins();
    let mut state = initial_state.clone();
    let mut flips = 0;
    let mut sweeps = 0;
    let mut converged = false;

    let initial_energy = model.energy(&state);
    let mut energy_trajectory = Vec::with_capacity(max_sweeps + 1);
    energy_trajectory.push(initial_energy);

    for _sweep in 0..max_sweeps {
        sweeps += 1;
        let mut sweep_flips = 0;

        for i in 0..n {
            let field = model.effective_field(&state, i);
            let si = state.get(i) as f64;
            let delta_e = 2.0 * si * field;
            // Coordinate descent step: flip if delta_e < 0 (i.e. spin misaligned with local field)
            if delta_e < -1e-12 {
                state.flip(i);
                sweep_flips += 1;
                flips += 1;
            }
        }

        let cur_e = model.energy(&state);
        energy_trajectory.push(cur_e);

        if sweep_flips == 0 {
            converged = true;
            break;
        }
    }

    let final_energy = *energy_trajectory.last().unwrap();
    let wall_time_us = start_time.elapsed().as_micros();

    DynamicsResult {
        final_state: state,
        initial_energy,
        final_energy,
        energy_trajectory,
        sweeps,
        flips,
        converged,
        oscillation_detected: false,
        wall_time_us,
    }
}

/// D1: Asynchronous stochastic Glauber / Gibbs sampling.
/// Transitions occur with probability: P(s_i -> +1) = 1 / (1 + exp(-2 * h_i^eff / T)).
pub fn glauber_sampling(
    model: &dyn EnergyModel,
    initial_state: &SpinState,
    temperature: f64,
    sweeps: usize,
    seed: u64,
) -> DynamicsResult {
    let start_time = Instant::now();
    let n = model.num_spins();
    let mut state = initial_state.clone();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut flips = 0;

    let initial_energy = model.energy(&state);
    let mut energy_trajectory = Vec::with_capacity(sweeps + 1);
    energy_trajectory.push(initial_energy);

    let beta = if temperature > 1e-9 {
        1.0 / temperature
    } else {
        1e9
    };

    for _ in 0..sweeps {
        for _ in 0..n {
            let i = rng.gen_range(0..n);
            let field = model.effective_field(&state, i);
            // P(s_i = +1) = sigma(2 * beta * field)
            let prob_up = 1.0 / (1.0 + (-2.0 * beta * field).exp());
            let new_val = if rng.gen::<f64>() < prob_up { 1 } else { -1 };
            if new_val != state.get(i) {
                state.set(i, new_val);
                flips += 1;
            }
        }
        energy_trajectory.push(model.energy(&state));
    }

    let final_energy = *energy_trajectory.last().unwrap();
    let wall_time_us = start_time.elapsed().as_micros();

    DynamicsResult {
        final_state: state,
        initial_energy,
        final_energy,
        energy_trajectory,
        sweeps,
        flips,
        converged: false,
        oscillation_detected: false,
        wall_time_us,
    }
}

/// D2: Parallel damped updates.
/// s_i(t+1) = sign((1 - alpha) * s_i(t) + alpha * h_i^eff(s(t))).
/// Tracks and detects 2-cycles and limit oscillations.
pub fn parallel_damped_update(
    model: &dyn EnergyModel,
    initial_state: &SpinState,
    alpha: f64,
    max_sweeps: usize,
) -> DynamicsResult {
    let start_time = Instant::now();
    let n = model.num_spins();
    let mut state = initial_state.clone();
    let mut prev_state = initial_state.clone();
    let mut flips = 0;
    let mut sweeps = 0;
    let mut converged = false;
    let mut oscillation_detected = false;

    let initial_energy = model.energy(&state);
    let mut energy_trajectory = Vec::with_capacity(max_sweeps + 1);
    energy_trajectory.push(initial_energy);

    let mut fields = vec![0.0; n];

    for _sweep in 0..max_sweeps {
        sweeps += 1;
        model.compute_all_fields(&state, &mut fields);

        let mut next_state = state.clone();
        for i in 0..n {
            let si = state.get(i) as f64;
            let effective = (1.0 - alpha) * si + alpha * fields[i];
            let new_s = if effective >= 0.0 { 1 } else { -1 };
            if new_s != state.get(i) {
                flips += 1;
            }
            next_state.set(i, new_s);
        }

        // Check convergence
        if next_state == state {
            converged = true;
            state = next_state;
            energy_trajectory.push(model.energy(&state));
            break;
        }

        // Check 2-cycle oscillation (next == prev_state)
        if sweeps > 1 && next_state == prev_state {
            oscillation_detected = true;
            state = next_state;
            energy_trajectory.push(model.energy(&state));
            break;
        }

        prev_state = state;
        state = next_state;
        energy_trajectory.push(model.energy(&state));
    }

    let final_energy = *energy_trajectory.last().unwrap();
    let wall_time_us = start_time.elapsed().as_micros();

    DynamicsResult {
        final_state: state,
        initial_energy,
        final_energy,
        energy_trajectory,
        sweeps,
        flips,
        converged,
        oscillation_detected,
        wall_time_us,
    }
}

/// D3: Simulated Annealing.
/// Geometric cooling schedule T_{k+1} = gamma * T_k.
pub fn simulated_annealing(
    model: &dyn EnergyModel,
    initial_state: &SpinState,
    t_start: f64,
    t_end: f64,
    sweeps: usize,
    seed: u64,
) -> DynamicsResult {
    let start_time = Instant::now();
    let n = model.num_spins();
    let mut state = initial_state.clone();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut flips = 0;

    let initial_energy = model.energy(&state);
    let mut energy_trajectory = Vec::with_capacity(sweeps + 1);
    energy_trajectory.push(initial_energy);

    let cooling_rate = (t_end / t_start).powf(1.0 / (sweeps as f64));
    let mut t = t_start;

    for _ in 0..sweeps {
        let beta = if t > 1e-9 { 1.0 / t } else { 1e9 };
        for _ in 0..n {
            let i = rng.gen_range(0..n);
            let field = model.effective_field(&state, i);
            let si = state.get(i) as f64;
            let delta_e = 2.0 * si * field;
            if delta_e <= 0.0 || rng.gen::<f64>() < (-delta_e * beta).exp() {
                state.flip(i);
                flips += 1;
            }
        }
        t *= cooling_rate;
        energy_trajectory.push(model.energy(&state));
    }

    let final_energy = *energy_trajectory.last().unwrap();
    let wall_time_us = start_time.elapsed().as_micros();

    DynamicsResult {
        final_state: state,
        initial_energy,
        final_energy,
        energy_trajectory,
        sweeps,
        flips,
        converged: false,
        oscillation_detected: false,
        wall_time_us,
    }
}

/// D4: Continuous relaxation: dx_i/dt = -dE/dx_i - gamma * x_i + noise, followed by sign binarization.
pub fn continuous_relaxation(
    model: &dyn EnergyModel,
    initial_state: &SpinState,
    dt: f64,
    gamma: f64,
    noise_amplitude: f64,
    steps: usize,
    seed: u64,
) -> DynamicsResult {
    let start_time = Instant::now();
    let n = model.num_spins();
    let mut c_state = ContinuousState::from_spins(initial_state);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let initial_energy = model.energy(initial_state);
    let mut energy_trajectory = Vec::with_capacity(steps + 1);
    energy_trajectory.push(initial_energy);

    for _ in 0..steps {
        let cur_spins = c_state.to_spins();
        for i in 0..n {
            let field = model.effective_field(&cur_spins, i);
            let xi = c_state.values[i];
            let noise: f64 = rng.gen_range(-1.0..1.0) * noise_amplitude;
            let d_xi = (field - gamma * xi) * dt + noise * dt.sqrt();
            c_state.values[i] = (xi + d_xi).clamp(-1.0, 1.0);
        }
        energy_trajectory.push(model.energy(&c_state.to_spins()));
    }

    let final_state = c_state.to_spins();
    let final_energy = model.energy(&final_state);
    let wall_time_us = start_time.elapsed().as_micros();

    DynamicsResult {
        final_state,
        initial_energy,
        final_energy,
        energy_trajectory,
        sweeps: steps,
        flips: 0,
        converged: true,
        oscillation_detected: false,
        wall_time_us,
    }
}

/// D5: Inertial / momentum dynamics.
/// v_i(t+1) = beta * v_i(t) + (1 - beta) * h_i^eff(x(t))
/// x_i(t+1) = clamp(x_i(t) + v_i(t+1) * dt, -1.0, 1.0)
pub fn momentum_dynamics(
    model: &dyn EnergyModel,
    initial_state: &SpinState,
    beta: f64,
    dt: f64,
    steps: usize,
) -> DynamicsResult {
    let start_time = Instant::now();
    let n = model.num_spins();
    let mut c_state = ContinuousState::from_spins(initial_state);
    let mut v = vec![0.0; n];

    let initial_energy = model.energy(initial_state);
    let mut energy_trajectory = Vec::with_capacity(steps + 1);
    energy_trajectory.push(initial_energy);

    for _ in 0..steps {
        let cur_spins = c_state.to_spins();
        for i in 0..n {
            let field = model.effective_field(&cur_spins, i);
            v[i] = beta * v[i] + (1.0 - beta) * field;
            c_state.values[i] = (c_state.values[i] + v[i] * dt).clamp(-1.0, 1.0);
        }
        energy_trajectory.push(model.energy(&c_state.to_spins()));
    }

    let final_state = c_state.to_spins();
    let final_energy = model.energy(&final_state);
    let wall_time_us = start_time.elapsed().as_micros();

    DynamicsResult {
        final_state,
        initial_energy,
        final_energy,
        energy_trajectory,
        sweeps: steps,
        flips: 0,
        converged: true,
        oscillation_detected: false,
        wall_time_us,
    }
}
