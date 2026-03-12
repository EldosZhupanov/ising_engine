🦾 [Superpowers] Socratic Design Spec: Bayesian Optimization Autopilot
## Objective
Replace hardcoded solver parameters with an autonomous Bayesian Optimization layer that dynamically finds the optimal (temp_max, temp_min, sweeps) for any given QUBO matrix to maximize solution quality (minimize energy) before full execution.

## Architecture (Rust native heuristic BO)
1. **Parameter Bounds:**
   - `temp_max` \in [10.0, 5000.0]
   - `temp_min` \in [0.001, 1.0]
   - `sweeps_multiplier` \in [10, 500]
2. **Exploration Phase (Grid/Random Seed):** Run 10 ultra-fast micro-anneals (e.g., 5 exchanges instead of 1000) across the parameter space.
3. **Exploitation Phase (Expected Improvement):** Since a full Gaussian Process library in pure Rust (without C++ dependencies like GPyOpt) is heavy, we will implement a lightweight **Tree-structured Parzen Estimator (TPE)** or a **Surrogate-based Hill Climbing** heuristic tailored for our specific 3D parameter space.
4. **The Autopilot Struct:** `BayesianOrchestrator::solve_auto(&model)` will handle the micro-runs, update the surrogate model, pick the best config, and run the final Deep Anneal.

## Engineering Pipeline
1. Create `src/solver/autopilot.rs`.
2. Implement the parameter sampling and feedback loop.
3. Create `src/bin/bayesian_demo.rs` to prove it finds better configurations than hardcoded ones.
