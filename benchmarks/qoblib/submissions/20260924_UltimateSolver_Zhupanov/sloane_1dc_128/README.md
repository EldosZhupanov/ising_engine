# Submission for sloane_1dc_128

This directory contains the official QOBLIB submission for problem **sloane_1dc_128**.

| Field | Value |
| --- | --- |
| Problem | sloane_1dc_128 |
| Submitter | Eldos Zhupanov |
| Affiliation | Ising Engine Project |
| Date | 2026-09-24 |
| ====== | |
| Reference | https://github.com/EldosZhupanov/ising_engine |
| Best Objective Value | 16 |
| Optimality Reference | 16 (Exact Gurobi Optimum) |
| ====== | |
| Modeling Approach | QUBO (Penalty P = 2.0) |
| # Decision Variables | 128 |
| # Binary Variables | 128 |
| # Integer Variables | 0 |
| # Continuous Variables | 0 |
| # Non-Zero Couplings | 1471 |
| Coefficients Type | Integer / Floating |
| ====== | |
| Workflow | Build penalized QUBO Hamiltonian, solve using UltimateSolver with CD005 Edge 2-Opt escapes and exact analytical presolve. |
| Algorithm Type | Stochastic Parallel Tempering + Local Search |
| Paradigm | Classical Commodity CPU |
| # Runs | 10 |
| # Feasible Runs | 10 |
| # Successful Runs | 10 |
| ====== | |
| Hardware Specifications | x86_64 Linux, 1 core (single-threaded) |
| ====== | |
| Total Runtime (s) | 2.20 |
| CPU Runtime (s) | 2.20 |
| QPU Runtime | N/A (Exceeds QPU physical topology limits) |
| ====== | |
| Remarks | Validated by official QOBLIB check_stableset verifier (0 edge collisions). |
