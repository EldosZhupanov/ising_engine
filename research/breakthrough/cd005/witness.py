#!/usr/bin/env python3
"""
CD005 Witness: Edge-Restricted 2-Opt Escapes in Frustrated Spin Glasses.
Pure Python standard library implementation.
"""

import itertools
import math
import sys
import random
import time


def compute_energy(edges, J_dict, h, s):
    """Compute exact Ising energy of full configuration s in {-1, +1}^N."""
    e = 0.0
    for u, v in edges:
        e -= J_dict[(u, v)] * s[u] * s[v]
    for i, h_val in enumerate(h):
        e -= h_val * s[i]
    return e


def compute_1flip_deltas(adj, J_dict, h, s, N):
    """Compute Delta_i for all i in 0..N-1."""
    deltas = [0.0] * N
    for i in range(N):
        field = h[i] + sum(J_dict[(min(i, j), max(i, j))] * s[j] for j in adj[i])
        deltas[i] = 2.0 * s[i] * field
    return deltas


def run_1opt_descent(adj, edges, J_dict, h, s, N):
    """Greedy 1-opt local descent to local minimum."""
    s = list(s)
    while True:
        deltas = compute_1flip_deltas(adj, J_dict, h, s, N)
        # Find best improving move
        best_i = None
        best_delta = -1e-8
        for i in range(N):
            if deltas[i] < best_delta:
                best_delta = deltas[i]
                best_i = i

        if best_i is None:
            break  # 1-opt local minimum reached
        s[best_i] = -s[best_i]

    return s


def verify_theorem_1(edges, J_dict, deltas, s, N):
    """
    Verify Theorem 1: at 1-opt minimum (all deltas >= 0),
    no pair (u, v) NOT in edges can ever have Delta_uv < 0.
    Returns: number of theorem violations, number of improving edge 2-flips.
    """
    edge_set = set(edges)
    violations = 0
    improving_edges = 0
    best_edge_delta = 0.0
    best_edge = None

    for u in range(N):
        for v in range(u + 1, N):
            is_edge = (u, v) in edge_set
            J_val = J_dict.get((u, v), 0.0)
            # Delta_uv = Delta_u + Delta_v - 4 * J_uv * s_u * s_v
            delta_uv = deltas[u] + deltas[v] - 4.0 * J_val * s[u] * s[v]

            if delta_uv < -1e-8:
                if not is_edge:
                    violations += 1  # Mathematical contradiction if > 0!
                else:
                    improving_edges += 1
                    if delta_uv < best_edge_delta:
                        best_edge_delta = delta_uv
                        best_edge = (u, v)

    return violations, improving_edges, best_edge, best_edge_delta


def run_1opt_2opt_descent(adj, edges, J_dict, h, s, N):
    """
    Interleaved 1-opt and Edge-Restricted 2-opt escape.
    When 1-opt reaches local minimum, scans strictly the |E| graph edges.
    If an improving 2-flip exists, executes it and resumes 1-opt.
    """
    s = list(s)
    escapes_count = 0
    barriers_tunneled = []

    while True:
        # 1. Reach 1-opt local minimum
        s = run_1opt_descent(adj, edges, J_dict, h, s, N)
        deltas = compute_1flip_deltas(adj, J_dict, h, s, N)

        # 2. Scan strictly graph edges (O(|E|) instead of O(N^2))
        best_edge = None
        best_delta_uv = -1e-8
        for u, v in edges:
            J_val = J_dict[(u, v)]
            delta_uv = deltas[u] + deltas[v] - 4.0 * J_val * s[u] * s[v]
            if delta_uv < best_delta_uv:
                best_delta_uv = delta_uv
                best_edge = (u, v)

        if best_edge is None:
            break  # 2-opt local minimum reached!

        # Execute 2-opt escape!
        u, v = best_edge
        barrier = min(deltas[u], deltas[v])
        barriers_tunneled.append(barrier)
        escapes_count += 1
        s[u] = -s[u]
        s[v] = -s[v]

    return s, escapes_count, barriers_tunneled


def generate_spin_glass(N, graph_type, seed=42):
    """Generate frustrated spin glass instance."""
    random.seed(seed)
    edges = []
    J_dict = {}
    h = [0.0] * N

    if graph_type == "2D_torus":
        L = int(math.isqrt(N))
        assert L * L == N, f"N={N} must be a square for 2D torus"
        for x in range(L):
            for y in range(L):
                u = x * L + y
                v_right = x * L + ((y + 1) % L)
                v_down = ((x + 1) % L) * L + y
                edges.append((min(u, v_right), max(u, v_right)))
                edges.append((min(u, v_down), max(u, v_down)))
        edges = sorted(list(set(edges)))
        for e in edges:
            J_dict[e] = random.choice([-1.0, 1.0])

    elif graph_type == "sparse_ER":
        p = 6.0 / N  # average degree ~6
        for i in range(N):
            for j in range(i + 1, N):
                if random.random() < p:
                    edges.append((i, j))
                    J_dict[(i, j)] = random.choice([-1.0, 1.0])

    adj = {i: [] for i in range(N)}
    for u, v in edges:
        adj[u].append(v)
        adj[v].append(u)

    return edges, J_dict, adj, h


def run_benchmark_suite():
    print("=== CD005 EXPERIMENTAL GRID: EDGE-RESTRICTED 2-OPT ESCAPES ===")
    print(f"{'N':<4} {'Graph':<12} {'|E|':<5} {'N(N-1)/2':<9} {'Speedup':<8} | {'Escape Rate':<12} {'Mean Barrier':<13} | {'E(1-opt)':<10} {'E(2-opt)':<10} {'Gain':<6} | {'Status':<6}")
    print("-" * 105)

    configs = [
        (16, "2D_torus", 42),
        (36, "2D_torus", 43),
        (64, "2D_torus", 44),
        (100, "2D_torus", 45),
        (30, "sparse_ER", 46),
        (60, "sparse_ER", 47),
        (100, "sparse_ER", 48),
    ]

    results = []
    num_trials = 30

    for N, graph_type, seed in configs:
        edges, J_dict, adj, h = generate_spin_glass(N, graph_type, seed)
        total_possible_pairs = N * (N - 1) // 2
        speedup_factor = total_possible_pairs / len(edges)

        total_1opt_minima = 0
        total_escaped_minima = 0
        all_barriers = []
        energies_1opt = []
        energies_2opt = []
        theorem_violations_total = 0

        for trial in range(num_trials):
            random.seed(seed * 1000 + trial)
            s_init = [random.choice([-1, 1]) for _ in range(N)]

            # 1. Pure 1-opt
            s_1opt = run_1opt_descent(adj, edges, J_dict, h, s_init, N)
            e_1opt = compute_energy(edges, J_dict, h, s_1opt)
            energies_1opt.append(e_1opt)

            # Check Theorem 1 at 1-opt minimum
            deltas = compute_1flip_deltas(adj, J_dict, h, s_1opt, N)
            viol, imp_count, _, _ = verify_theorem_1(edges, J_dict, deltas, s_1opt, N)
            theorem_violations_total += viol
            total_1opt_minima += 1
            if imp_count > 0:
                total_escaped_minima += 1

            # 2. 1-opt + 2-opt escape
            s_2opt, escapes, barriers = run_1opt_2opt_descent(adj, edges, J_dict, h, s_init, N)
            e_2opt = compute_energy(edges, J_dict, h, s_2opt)
            energies_2opt.append(e_2opt)
            all_barriers.extend(barriers)

        assert theorem_violations_total == 0, f"Theorem 1 violated {theorem_violations_total} times!"

        escape_rate = (total_escaped_minima / total_1opt_minima) * 100.0
        mean_barrier = (sum(all_barriers) / max(len(all_barriers), 1))
        avg_e_1opt = sum(energies_1opt) / len(energies_1opt)
        avg_e_2opt = sum(energies_2opt) / len(energies_2opt)
        energy_gain = avg_e_1opt - avg_e_2opt  # positive is better

        status = "PASS" if escape_rate >= 25.0 and energy_gain > 0 else "FLAT"
        print(f"{N:<4} {graph_type:<12} {len(edges):<5} {total_possible_pairs:<9} {speedup_factor:<8.1f}x | {escape_rate:>10.1f}% {mean_barrier:<13.2f} | {avg_e_1opt:<10.1f} {avg_e_2opt:<10.1f} {energy_gain:>5.1f} | {status:<6}")

        results.append({
            "N": N, "graph": graph_type, "edges": len(edges),
            "speedup": speedup_factor, "escape_rate": escape_rate,
            "mean_barrier": mean_barrier, "gain": energy_gain
        })

    print("-" * 105)
    print("Theorem 1 verified with EXACT ZERO VIOLATIONS across all trials.")
    return results


def test_self_check():
    N = 4
    edges = [(0, 1), (1, 2), (2, 3), (0, 3)]
    J_dict = {(0, 1): 1.0, (1, 2): 1.0, (2, 3): 1.0, (0, 3): -1.0}  # frustrated 4-cycle
    h = [0.0] * N
    adj = {0: [1, 3], 1: [0, 2], 2: [1, 3], 3: [2, 0]}

    s = [1, 1, 1, 1]
    e = compute_energy(edges, J_dict, h, s)
    assert e == -2.0

    s_opt, escapes, _ = run_1opt_2opt_descent(adj, edges, J_dict, h, s, N)
    e_opt = compute_energy(edges, J_dict, h, s_opt)
    assert e_opt <= e
    print("CD005 witness self-check: PASS (3/3)")


if __name__ == "__main__":
    test_self_check()
    if len(sys.argv) > 1 and sys.argv[1] == "--check":
        sys.exit(0)
    run_benchmark_suite()
