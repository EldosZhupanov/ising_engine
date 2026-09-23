# Submission for ms_03_050_009

This directory contains the official QOBLIB submission for problem **ms_03_050_009**.

| Field | Value |
| --- | --- |
| Problem | ms_03_050_009 |
| Submitter | Eldos Zhupanov |
| Affiliation | Ising Engine Project |
| Date | 2026-09-24 |
| ====== | |
| Reference | https://github.com/EldosZhupanov/ising_engine |
| Best Objective Value | 0.0 |
| Optimality Reference | 0.0 (Exact Feasible Solution $Ax = b$) |
| ====== | |
| Modeling Approach | Exact Penalty QUBO $\sum_i (A_i \cdot x - b_i)^2$ |
| # Decision Variables | 20 |
| # Binary Variables | 20 |
| # Integer Variables | 0 |
| # Continuous Variables | 0 |
| # Constraints | 3 |
| Coefficients Type | Integer |
| ====== | |
| Workflow | Exact QUBO formulation solved with UltimateSolver + CD005 2-Opt and Tabu search. |
| Algorithm Type | Stochastic Parallel Tempering + 2-Opt Local Search + Tabu |
| Paradigm | Classical Commodity CPU |
| # Runs | 10 |
| # Feasible Runs | 10 |
| # Successful Runs | 10 |
| ====== | |
| Hardware Specifications | x86_64 Linux, commodity CPU |
| ====== | |
| Total Runtime (s) | 0.70 |
| CPU Runtime (s) | 0.70 |
| QPU Runtime | N/A |
| ====== | |
| Remarks | Validated by official QOBLIB check_marketsplit verifier (0 violations, exit code 0). Newly evaluated instance. |
