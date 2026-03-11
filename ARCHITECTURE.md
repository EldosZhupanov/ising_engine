# Architecture

## Module Hierarchy

```
ising_engine (lib)
├── core          — Data structures for the QUBO Hamiltonian
│   ├── CsrMatrix   — Compressed Sparse Row storage for J_ij weights
│   └── QuboModel   — Linear biases (h_i) + quadratic interactions + energy computation
│
├── compiler      — Logic → QUBO translation
│   └── LogicBuilder — AND, XOR, half/full adders, 2x2 multiplier → penalty functions
│
└── solver        — Optimization algorithms
    ├── Replica                  — State vector + temperature + energy
    ├── ParallelTemperingSolver  — Base solver: Metropolis-Hastings + replica exchange
    ├── AdaptiveTemperingSolver  — Adds runtime temperature adjustment
    └── ClusterSolver            — Adds Wolff-style cluster flips
```

## QUBO Pipeline

```
Digital Circuit          LogicBuilder            CsrMatrix + Linear
  ┌──────┐               ┌──────────┐            ┌──────────────┐
  │A AND B│──── gates ───►│ Penalty  │── build ──►│  QuboModel   │
  │X XOR Y│              │ Functions │            │  (Sparse H)  │
  └──────┘               └──────────┘            └──────┬───────┘
                                                        │
                  ┌─────────────────────────────────────┘
                  ▼
            Parallel Tempering
            ┌──────────────────┐
            │ N replicas at    │
            │ T_max → T_min    │
            │ Metropolis sweep │
            │ + Replica swap   │
            └────────┬─────────┘
                     │
                     ▼
              Minimum Energy State
              → Binary Assignment
```

## Solver Variants

| Solver | Parallelism | Temperature | Special Moves |
|--------|------------|-------------|---------------|
| `ParallelTemperingSolver` | Sequential | Fixed log-space | — |
| `AdaptiveTemperingSolver` | Rayon parallel | Adaptive (target 23% swap rate) | — |
| `ClusterSolver` | Rayon parallel | Adaptive | Wolff cluster flips every 10 steps |

## Binaries

Core binaries are thin wrappers (~30–45 lines) that configure a circuit and solver:

- `ising_engine` (main) — Toffoli gate reverse computation
- `toffoli` — Same as main
- `adder` — 2-bit reversible adder
- `factor` — 2×2 integer factorization
- `hash_breaker` — Hash preimage search (4-bit toy hash)
- `adaptive` — Factorization with adaptive tempering
- `cluster` — Factorization with cluster flips

Research binaries (self-contained, not refactored):
- `maxcut_pro`, `sat_qubo`, `bayesian_scaling_lab*` — Optimization experiments
- `scanner`, `qubo`, `web_demo` — Crypto/web applications
