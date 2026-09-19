#!/usr/bin/env python3
"""
Physical D-Wave Quantum Annealer Execution Driver.

Loads the exact BQM instances exported by `dwave_quantum_challenge.rs`
and executes them against D-Wave Advantage / Advantage2 via the Ocean SDK.
Records exact QPU access, programming, sampling times, and energy distributions.

Prerequisites:
  pip install dwave-ocean-sdk
  dwave config create (or set DWAVE_API_TOKEN environment variable)
"""

import os
import sys
import json
import time

def solve_with_dwave(json_path: str, num_reads: int = 1000, annealing_time: int = 20):
    if not os.path.exists(json_path):
        print(f"Error: file not found: {json_path}")
        print("Run `cargo run --release --bin dwave_quantum_challenge` first to export the instances.")
        return

    try:
        import dimod
        from dwave.system import DWaveSampler, EmbeddingComposite
    except ImportError:
        print("Error: dwave-ocean-sdk is not installed.")
        print("Install via: pip install dwave-ocean-sdk")
        return

    print(f"\nLoading BQM from {json_path}...")
    with open(json_path, "r") as f:
        data = json.load(f)

    linear = {u: w for u, w in data["linear"]}
    quadratic = {(u, v): w for u, v, w in data["quadratic"]}
    offset = data["energy_offset"]

    bqm = dimod.BinaryQuadraticModel(linear, quadratic, offset, dimod.BINARY)
    print(f"BQM constructed: {bqm.num_variables} variables, {len(bqm.quadratic)} quadratic couplers.")

    print(f"Connecting to D-Wave Leap QPU (reads={num_reads}, anneal_time={annealing_time}µs)...")
    try:
        sampler = EmbeddingComposite(DWaveSampler())
        t0 = time.time()
        sampleset = sampler.sample(
            bqm,
            num_reads=num_reads,
            annealing_time=annealing_time,
            label="IsingEngine-vs-DWave-Benchmark"
        )
        total_wall_time = time.time() - t0
    except Exception as e:
        print(f"Failed to execute on D-Wave QPU: {e}")
        print("Make sure you have an active D-Wave Leap token configured (`dwave config create`).")
        return

    print("\n--- D-Wave QPU Timing Breakdown ---")
    timing = sampleset.info.get("timing", {})
    for key, val in timing.items():
        print(f"  {key:<30}: {val} µs")
    print(f"  Total Client Wall-Clock Time  : {total_wall_time:.4f} s")

    best_sample = sampleset.first
    print("\n--- D-Wave Energy Result ---")
    print(f"  Best Energy Found: {best_sample.energy:.4f}")
    print(f"  Number of Occurrences: {best_sample.num_occurrences} / {num_reads}")

if __name__ == "__main__":
    base_dir = os.path.join(os.path.dirname(__file__), "..", "target", "dwave_benchmarks")
    chimera_path = os.path.join(base_dir, "chimera_2048.json")
    portfolio_path = os.path.join(base_dir, "portfolio_100.json")

    print("=" * 70)
    print("D-WAVE QUANTUM ANNEALER EXECUTION HARNESS")
    print("=" * 70)

    if len(sys.argv) > 1:
        solve_with_dwave(sys.argv[1])
    else:
        print("\n[1/2] Benchmarking Chimera 2048...")
        solve_with_dwave(chimera_path)
        print("\n[2/2] Benchmarking Portfolio 100...")
        solve_with_dwave(portfolio_path)
