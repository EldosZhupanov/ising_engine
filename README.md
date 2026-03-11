# 🧊 Ising Engine

A research-grade **QUBO (Quadratic Unconstrained Binary Optimization)** engine written in Rust. Uses simulated annealing with parallel tempering to solve combinatorial optimization problems by mapping them onto Ising model ground-state searches.

> **⚠️ Disclaimer:** This is a **research/educational project**. Crypto-related binaries are experimental demonstrations, not production trading systems.

## Features

- 🔧 **Logic-to-QUBO compiler** — Translates digital circuits (AND, XOR, adders, multipliers) into QUBO penalty functions
- 🌡️ **Parallel Tempering** — Multiple replicas at different temperatures with Metropolis-Hastings sweeps and replica exchange
- 🔄 **Adaptive Tempering** — Runtime temperature optimization targeting optimal swap acceptance rates
- 🧲 **Cluster Flips** — Wolff-style cluster moves for escaping deep local minima
- ⚡ **CSR Matrix** — Compressed Sparse Row representation for efficient large-scale problems
- 🧬 **Rayon parallelism** — Multi-threaded replica sweeps

## Quick Start

```bash
# Build
cargo build --release

# Run the Toffoli gate demo (reverse logic)
cargo run --release

# Run 2x2 factorization (factor 6 = ? × ?)
cargo run --release --bin factor

# Run hash preimage attack
cargo run --release --bin hash_breaker

# Run tests
cargo test
```

## Architecture

```
src/
├── lib.rs                   # Library root
├── core/
│   ├── csr_matrix.rs        # Compressed Sparse Row matrix
│   └── qubo_model.rs        # QUBO Hamiltonian + energy calculation
├── compiler/
│   └── logic_builder.rs     # Logic gates → QUBO penalty compiler
├── solver/
│   ├── replica.rs           # Replica state + clamped-variable utilities
│   ├── parallel_tempering.rs # Base PT solver
│   ├── adaptive.rs          # Adaptive temperature PT
│   └── cluster.rs           # Cluster-flip + adaptive PT
└── bin/
    ├── toffoli.rs            # Reverse Toffoli gate
    ├── adder.rs              # Reverse 2-bit adder
    ├── factor.rs             # 2×2 factorization
    ├── hash_breaker.rs       # Hash preimage search
    ├── adaptive.rs           # Adaptive tempering demo
    ├── cluster.rs            # Cluster flip demo
    └── ...                   # Research/experimental binaries
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full module design.

## How It Works

1. **Compile** logic gates into QUBO penalty functions (energy = 0 iff gate is satisfied)
2. **Build** a sparse Hamiltonian matrix (CSR format)
3. **Solve** via parallel tempering: Metropolis sweeps + replica exchange
4. **Extract** the binary state with minimum energy

### Example: Factoring 6

```rust
use ising_engine::compiler::{LogicBuilder, Multiplier2x2};
use ising_engine::solver::ParallelTemperingSolver;

let mut compiler = LogicBuilder::new();
let (a0, a1) = (compiler.add_var(), compiler.add_var());
let (b0, b1) = (compiler.add_var(), compiler.add_var());
let (p0, p1, p2, p3) = (compiler.add_var(), compiler.add_var(),
                          compiler.add_var(), compiler.add_var());

compiler.add_multiplier_2x2(Multiplier2x2 {
    a: [a0, a1],
    b: [b0, b1],
    p: [p0, p1, p2, p3],
});
let model = compiler.build();

let solver = ParallelTemperingSolver {
    num_replicas: 64, temp_max: 300.0, temp_min: 0.01,
    sweeps_per_exchange: 2000, total_exchanges: 400,
};

// Clamp output to 6 (binary: 0110)
let clamped = vec![(p3, 0), (p2, 1), (p1, 1), (p0, 0)];
let result = solver.solve(&model, &clamped);
// result → A=2, B=3 or A=3, B=2
```

## Security

See [SECURITY.md](SECURITY.md). All secrets must be provided via environment variables. Never commit `.env` files.

## License

Research use. See repository for details.
