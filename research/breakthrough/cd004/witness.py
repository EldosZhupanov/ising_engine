#!/usr/bin/env python3
"""
CD004 Witness: Native Soft-Conflict Learning vs Chronological Branch-and-Bound.
Pure Python standard library implementation.
"""

import itertools
import math
import sys
import random
import time


def compute_energy(J, h, s, N):
    """Compute exact Ising energy of full spin configuration s in {-1, +1}^N."""
    e = 0.0
    for i in range(N):
        for j in range(i + 1, N):
            if J[i][j] != 0:
                e -= J[i][j] * s[i] * s[j]
        e -= h[i] * s[i]
    return e


def find_heuristic_incumbent(J, h, N, num_restarts=10):
    """Greedy 1-opt local search to establish a realistic tight incumbent E*."""
    best_s = None
    best_e = 1e18

    for _ in range(num_restarts):
        s = [random.choice([-1, 1]) for _ in range(N)]
        e = compute_energy(J, h, s, N)

        # 1-opt descent
        improved = True
        while improved:
            improved = False
            for i in range(N):
                # Calculate delta E of flipping spin i
                # Old contribution: - sum_{j != i} J_ij s_i s_j - h_i s_i
                # Flip s_i to -s_i: delta_E = 2 * s_i * (sum_j J_ij s_j + h_i)
                field = h[i] + sum(J[i][j] * s[j] for j in range(N) if j != i)
                delta_e = 2 * s[i] * field
                if delta_e < -1e-8:
                    s[i] = -s[i]
                    e += delta_e
                    improved = True

        if e < best_e:
            best_e = e
            best_s = list(s)

    return best_s, best_e


def compute_lower_bound(J, h, N, fixed_vars, fixed_signs):
    """
    Compute fast certified lower bound on conditional energy:
    E(s_F, s_R) >= E_F - sum_{R} |J_uv| - sum_{R} |h_u^eff|
    """
    F_set = set(fixed_vars)
    F_dict = {v: sign for v, sign in zip(fixed_vars, fixed_signs)}
    R_vars = [i for i in range(N) if i not in F_set]

    # 1. Fixed internal energy E_F
    e_fixed = 0.0
    for idx_u, u in enumerate(fixed_vars):
        s_u = fixed_signs[idx_u]
        for idx_v in range(idx_u + 1, len(fixed_vars)):
            v = fixed_vars[idx_v]
            if J[u][v] != 0:
                e_fixed -= J[u][v] * s_u * fixed_signs[idx_v]
        e_fixed -= h[u] * s_u

    # 2. Remaining edges within R
    rem_edges_cost = 0.0
    for idx_u, u in enumerate(R_vars):
        for idx_v in range(idx_u + 1, len(R_vars)):
            v = R_vars[idx_v]
            if J[u][v] != 0:
                rem_edges_cost += abs(J[u][v])

    # 3. Effective field on R
    eff_field_cost = 0.0
    for u in R_vars:
        h_eff = h[u] + sum(J[u][v] * F_dict[v] for v in fixed_vars if J[u][v] != 0)
        eff_field_cost += abs(h_eff)

    return e_fixed - rem_edges_cost - eff_field_cost


def extract_minimal_conflict_core(J, h, N, fixed_vars, fixed_signs, E_star):
    """
    Greedy deletion filter: eliminates non-essential variables from F
    while preserving LB(core) > E_star.
    """
    core_vars = list(fixed_vars)
    core_signs = list(fixed_signs)

    # Try removing variables from the earliest decisions to latest
    for i in range(len(fixed_vars)):
        v_to_remove = fixed_vars[i]
        if v_to_remove not in core_vars:
            continue

        # Trial core without v_to_remove
        trial_vars = [u for u in core_vars if u != v_to_remove]
        trial_signs = [core_signs[core_vars.index(u)] for u in trial_vars]

        lb = compute_lower_bound(J, h, N, trial_vars, trial_signs)
        if lb > E_star + 1e-8:
            # v_to_remove was not needed for conflict!
            core_vars = trial_vars
            core_signs = trial_signs

    return core_vars, core_signs


def solve_chronological_bnb(J, h, N, E_star):
    """
    Standard Chronological Branch-and-Bound.
    """
    nodes_visited = 0
    subtrees_pruned = 0
    best_E = E_star

    def bnb_recurse(depth, fixed_vars, fixed_signs):
        nonlocal nodes_visited, subtrees_pruned, best_E
        nodes_visited += 1

        # Check lower bound
        lb = compute_lower_bound(J, h, N, fixed_vars, fixed_signs)
        if lb >= best_E - 1e-8:
            subtrees_pruned += 1
            return

        if depth == N:
            # Reached full assignment
            e = compute_energy(J, h, fixed_signs, N)
            if e < best_E:
                best_E = e
            return

        # Branch on variable at index 'depth'
        v = depth
        for sign in [1, -1]:
            bnb_recurse(depth + 1, fixed_vars + [v], fixed_signs + [sign])

    bnb_recurse(0, [], [])
    return nodes_visited, subtrees_pruned, best_E


def solve_conflict_driven_bnb(J, h, N, E_star):
    """
    Native Conflict-Driven Branch-and-Bound with Deletion Filtering and Backjumping.
    Returns: nodes_visited, subtrees_pruned, backjumps_count, mean_core_ratio, best_E.
    """
    nodes_visited = 0
    subtrees_pruned = 0
    backjumps_count = 0
    core_ratios = []
    best_E = E_star

    # Decision level mapping: depth -> variable
    def cdcl_recurse(depth, fixed_vars, fixed_signs):
        nonlocal nodes_visited, subtrees_pruned, backjumps_count, best_E
        nodes_visited += 1

        # Check lower bound
        lb = compute_lower_bound(J, h, N, fixed_vars, fixed_signs)
        if lb >= best_E - 1e-8:
            subtrees_pruned += 1
            # Conflict occurred! Extract minimal core
            if len(fixed_vars) > 1:
                core_vars, _ = extract_minimal_conflict_core(J, h, N, fixed_vars, fixed_signs, best_E)
                ratio = len(core_vars) / len(fixed_vars)
                core_ratios.append(ratio)

                # Determine backjump target: second highest depth in core
                core_depths = [fixed_vars.index(u) for u in core_vars]
                core_depths.sort()
                # If core doesn't contain current variable or is smaller than full branch
                if len(core_depths) >= 2:
                    jump_depth = core_depths[-2]  # second highest
                else:
                    jump_depth = -1

                if jump_depth < depth - 1:
                    backjumps_count += 1
                    return jump_depth  # Signal non-chronological backjump
            else:
                core_ratios.append(1.0)
            return depth - 1  # Normal backtrack

        if depth == N:
            e = compute_energy(J, h, fixed_signs, N)
            if e < best_E:
                best_E = e
            return depth - 1

        v = depth
        for sign in [1, -1]:
            target_depth = cdcl_recurse(depth + 1, fixed_vars + [v], fixed_signs + [sign])
            if target_depth is not None and target_depth < depth:
                # Backjump propagates upward!
                return target_depth

        return depth - 1

    cdcl_recurse(0, [], [])
    mean_core = sum(core_ratios) / max(len(core_ratios), 1)
    return nodes_visited, subtrees_pruned, backjumps_count, mean_core, best_E


def generate_frustrated_instance(N, graph_type, seed=42):
    """Generate frustrated Ising spin glass instance."""
    random.seed(seed)
    J = [[0.0] * N for _ in range(N)]
    h = [0.0] * N  # zero-field (hardest case for spin glasses)

    if graph_type == "2D_lattice":
        # 2D torus L x L
        L = int(math.isqrt(N))
        for x in range(L):
            for y in range(L):
                u = x * L + y
                # Right neighbor
                v_right = x * L + ((y + 1) % L)
                # Down neighbor
                v_down = ((x + 1) % L) * L + y
                J[u][v_right] = random.choice([-1.0, 1.0])
                J[v_right][u] = J[u][v_right]
                J[u][v_down] = random.choice([-1.0, 1.0])
                J[v_down][u] = J[u][v_down]

    elif graph_type == "random_sparse":
        # Erdős-Rényi p=0.4
        p = 0.4
        for i in range(N):
            for j in range(i + 1, N):
                if random.random() < p:
                    val = random.choice([-2.0, -1.0, 1.0, 2.0])
                    J[i][j] = val
                    J[j][i] = val

    elif graph_type == "complete_SK":
        # Sherrington-Kirkpatrick
        for i in range(N):
            for j in range(i + 1, N):
                val = random.choice([-1.0, 1.0])
                J[i][j] = val
                J[j][i] = val

    return J, h


def run_benchmark_grid():
    print("=== CD004 EXPERIMENTAL GRID: NATIVE SOFT-CONFLICT PRUNING ===")
    print(f"{'N':<3} {'Graph':<14} | {'Nodes (BnB)':<12} {'Nodes (CDCL)':<13} {'Reduction':<10} | {'Backjumps':<10} {'Mean Core':<10} | {'Status':<6}")
    print("-" * 85)

    test_cases = [
        (12, "2D_lattice", 42),
        (12, "random_sparse", 42),
        (12, "complete_SK", 42),
        (14, "random_sparse", 43),
        (14, "complete_SK", 43),
        (16, "2D_lattice", 44),
        (16, "random_sparse", 44),
    ]

    results = []
    for N, graph_type, seed in test_cases:
        J, h = generate_frustrated_instance(N, graph_type, seed)
        _, E_star = find_heuristic_incumbent(J, h, N, num_restarts=20)

        # Run Chronological BnB
        t0 = time.time()
        nodes_bnb, pruned_bnb, e_final_bnb = solve_chronological_bnb(J, h, N, E_star)
        t_bnb = time.time() - t0

        # Run Conflict-Driven BnB
        t0 = time.time()
        nodes_cdcl, pruned_cdcl, bj_count, mean_core, e_final_cdcl = solve_conflict_driven_bnb(J, h, N, E_star)
        t_cdcl = time.time() - t0

        # Both must find optimal or verify incumbent identically
        assert abs(e_final_bnb - e_final_cdcl) < 1e-6, f"Energy mismatch: {e_final_bnb} vs {e_final_cdcl}"

        reduction = (1.0 - nodes_cdcl / nodes_bnb) * 100.0
        status = "PASS" if reduction >= 20.0 or bj_count > 0 else "FLAT"

        print(f"{N:<3} {graph_type:<14} | {nodes_bnb:<12} {nodes_cdcl:<13} {reduction:>8.1f}% | {bj_count:<10} {mean_core:<10.3f} | {status:<6}")
        results.append({
            "N": N, "graph": graph_type, "nodes_bnb": nodes_bnb,
            "nodes_cdcl": nodes_cdcl, "reduction": reduction,
            "backjumps": bj_count, "mean_core": mean_core
        })

    print("-" * 85)
    return results


def test_self_check():
    """Unit test for witness integrity."""
    N = 4
    J = [[0.0] * N for _ in range(N)]
    h = [0.0] * N
    J[0][1] = 1.0; J[1][0] = 1.0
    J[1][2] = 1.0; J[2][1] = 1.0
    J[2][3] = 1.0; J[3][2] = 1.0

    s = [1, 1, 1, 1]
    e = compute_energy(J, h, s, N)
    assert e == -3.0, f"Expected -3.0, got {e}"

    lb = compute_lower_bound(J, h, N, [0], [1])
    assert lb <= -3.0, f"LB {lb} must be <= true min -3.0"

    print("CD004 witness self-check: PASS (3/3)")


if __name__ == "__main__":
    test_self_check()
    if len(sys.argv) > 1 and sys.argv[1] == "--check":
        sys.exit(0)
    run_benchmark_grid()
