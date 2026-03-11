---
name: ising_engine
description: >
  QUBO combinatorial optimization solver using Ising model simulated annealing.
  Supports factorization, logic-gate reversal, hash preimage search, and
  arbitrary QUBO problems via parallel tempering.
version: 0.1.0
tools:
  - name: ising_solve
    description: >
      Solve a QUBO optimization problem. Supports modes: factor (factorize an integer),
      toffoli (reverse Toffoli gate), hash (hash preimage), custom (raw QUBO matrix).
    parameters:
      - name: mode
        type: string
        required: true
        description: "One of: factor, toffoli, hash, custom"
      - name: target
        type: integer
        required: false
        description: "Target number for factorization mode"
      - name: replicas
        type: integer
        required: false
        description: "Number of parallel tempering replicas (default: 64)"
      - name: sweeps
        type: integer
        required: false
        description: "Sweeps per exchange (default: 2000)"
    run: |
      cd ~/.zeroclaw/workspace/skills/ising_engine
      cargo run --release --bin {{mode}} -- {{target}} 2>&1
---

# 🧊 Ising Engine Skill

This skill gives ZeroClaw access to a research-grade QUBO optimization solver.

## Available Modes

| Mode | Binary | Description |
|------|--------|-------------|
| `factor` | `cargo run --release --bin factor` | Factorize integers via QUBO |
| `toffoli` | `cargo run --release` | Reverse Toffoli gate logic |
| `hash` | `cargo run --release --bin hash_breaker` | Hash preimage search |
| `adaptive` | `cargo run --release --bin adaptive` | Adaptive tempering demo |
| `cluster` | `cargo run --release --bin cluster` | Cluster flip demo |

## Usage Examples

**"Factor the number 6"** → runs `factor` binary, returns 2×3 or 3×2
**"Reverse a Toffoli gate with A=1, B=1, Cout=0"** → runs main binary, finds Cin
