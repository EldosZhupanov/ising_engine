🦾 [Superpowers] Socratic Design Spec: Portfolio Solver Architecture
## Objective
Implement a 'Portfolio of Solvers' (Level 10 from the MIT/Stanford spec). Instead of relying on a single algorithm (Simulated Annealing), the engine will orchestrate multiple diverse optimization algorithms in parallel (multi-threading via Rayon), and return the best global result.

## Architecture (The Hive Mind)
1. **Algorithm 1: UltimateSolver (Simulated Annealing + MSC):** Good for highly non-linear, deep energy landscapes.
2. **Algorithm 2: Tabu Search:** Good for escaping shallow local minima by keeping a memory of recently flipped spins. Purely local search.
3. **Algorithm 3: Extremal Optimization:** Good for power-law distributed graphs (Scale-free networks). Flips the worst-performing node regardless of global energy.
4. **Orchestrator (`PortfolioSolver`):** Spins up 3 parallel threads. Each thread runs a different algorithm on the same QUBO model. Whichever thread finds the lowest energy first (or at the end of the time limit) wins.

## Engineering Pipeline
- Create `src/solver/tabu.rs`.
- Create `src/solver/extremal.rs`.
- Create `src/solver/portfolio.rs` to orchestrate them via Rayon.
