<div align="center">

<img src="assets/logo.png" alt="Ising Engine Logo" width="420"/>

# Ising Engine

**High-performance combinatorial optimization engine written in Rust**

*Research-focused framework for solving Ising, QUBO and HUBO optimization problems.*

</div>

---

## Overview

Ising Engine is a high-performance optimization framework designed for combinatorial optimization research and AI-assisted decision systems.

The project provides a modular Rust implementation of modern optimization algorithms with support for Ising, QUBO and HUBO models, parallel execution, SIMD-oriented computation and reproducible benchmarking.

---

## Features

- High-performance Rust implementation
- Ising optimization
- QUBO optimization
- HUBO optimization (higher-order models)
- Parallel Tempering
- Population Annealing
- Multi-Spin Coding
- SIMD-ready architecture
- Sparse CSR matrix representation
- Parallel execution
- Modular solver architecture
- Axum HTTP API
- Criterion benchmarks
- Research workspace for experimental algorithms

---

## Repository Structure

```text
src/
├── core/          Core mathematical structures
├── solver/        Optimization algorithms
├── compiler/      Problem compiler
├── bin/           Demonstrations and benchmark binaries

research/
├── Experimental algorithms
├── Scaling experiments
├── Research prototypes

tests/
├── Unit tests
├── Integration tests

benches/
├── Performance benchmarks
```

---

## Current Capabilities

Currently implemented:

- Binary optimization using Ising models
- QUBO optimization
- HUBO optimization
- Parallel optimization algorithms
- SIMD-oriented implementation
- Modular optimization framework
- HTTP API
- Benchmark infrastructure

---

## Build

```bash
cargo build --release
```

---

## Run Tests

```bash
cargo test --release
```

---

## Run Benchmarks

```bash
cargo bench
```

---

## Documentation

| File | Purpose |
|------|---------|
| ARCHITECTURE.md | System architecture |
| ROADMAP.md | Development roadmap |
| PERF.md | Performance notes |
| VERIFY.md | Verification procedures |
| SECURITY.md | Security considerations |
| AGENTS.md | Development workflow |
| CLAUDE.md | AI development guidelines |
| CONTEXT.md | Project context |

---

## Design Principles

The project follows several engineering principles:

- Performance-first implementation
- Modular architecture
- Clean Rust codebase
- Research-friendly design
- Reproducible experiments
- Extensible optimization framework

---

## Applications

Potential application areas include:

- Combinatorial Optimization
- Graph Optimization
- Max-Cut
- SAT
- Scheduling
- Logistics
- Portfolio Optimization
- Resource Allocation
- Operations Research
- AI-assisted Decision Systems

---

## Technology Stack

- Rust
- Rayon
- Tokio
- Axum
- Criterion
- SIMD
- CSR Sparse Matrices

---

## Project Status

The project is under active development.

Current priorities include:

- Solver improvements
- Performance optimization
- Additional benchmark coverage
- Experimental optimization techniques
- Research infrastructure

---

## License

Licensed under either:

- MIT License
- Apache License 2.0

at your option.

---

<div align="center">

**Built with Rust for high-performance optimization research.**

</div>
