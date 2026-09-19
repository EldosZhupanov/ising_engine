#!/usr/bin/env python3
"""
CD002 Witness: Discrete Gauge Synchronization vs Spectral Synchronization.
Pure Python standard library implementation (no external dependencies).
"""

import itertools
import math
import sys
import random


def all_permutations(d):
    """Generate all d! permutations as tuples."""
    return list(itertools.permutations(range(d)))


def compose_perm(p, q):
    """Compute composition p o q: (p o q)(i) = p[q[i]]."""
    return tuple(p[q[i]] for i in range(len(p)))


def invert_perm(p):
    """Compute inverse permutation p^-1."""
    d = len(p)
    inv = [0] * d
    for i, val in enumerate(p):
        inv[val] = i
    return tuple(inv)


def perm_overlap(p, q):
    """Count number of matching points between two permutations (Tr(P Q^T))."""
    return sum(1 for i in range(len(p)) if p[i] == q[i])


def mat_mul_vec(M, v):
    """Multiply square matrix M by vector v."""
    n = len(v)
    return [sum(M[i][j] * v[j] for j in range(n)) for i in range(n)]


def dot(v1, v2):
    """Dot product of two vectors."""
    return sum(x * y for x, y in zip(v1, v2))


def norm(v):
    """Euclidean norm of vector."""
    return math.sqrt(sum(x * x for x in v))


def normalize(v):
    """Normalize vector to unit length."""
    nv = norm(v)
    if nv < 1e-12:
        return v
    return [x / nv for x in v]


def power_iteration_top_k(R, k, max_iter=200, tol=1e-8):
    """
    Compute top k eigenvectors of symmetric matrix R using power iteration with Gram-Schmidt deflation.
    """
    n = len(R)
    eigenvectors = []

    for _ in range(k):
        # Random initial vector
        v = [random.gauss(0, 1) for _ in range(n)]
        # Orthogonalize against previously found eigenvectors
        for prev in eigenvectors:
            proj = dot(v, prev)
            v = [v[i] - proj * prev[i] for i in range(n)]
        v = normalize(v)

        for _ in range(max_iter):
            # Multiply
            v_next = mat_mul_vec(R, v)
            # Orthogonalize
            for prev in eigenvectors:
                proj = dot(v_next, prev)
                v_next = [v_next[i] - proj * prev[i] for i in range(n)]
            v_next = normalize(v_next)

            # Check convergence
            diff = norm([v_next[i] - v[i] for i in range(n)])
            v = v_next
            if diff < tol:
                break

        eigenvectors.append(v)

    return eigenvectors  # list of k vectors of length n


def solve_spectral_sync(edge_measurements, K, d):
    """
    Spectral Permutation Synchronization (Pachauri et al. 2013).
    Constructs block matrix R (Kd x Kd), finds top d eigenvectors,
    and projects each block to S_d by maximizing Tr(P^T Block).
    """
    n = K * d
    # Initialize R as identity
    R = [[0.0] * n for _ in range(n)]
    for i in range(n):
        R[i][i] = 1.0

    # Fill off-diagonal blocks
    for (u, v), pi_uv in edge_measurements.items():
        # Block (u, v) gets permutation matrix of pi_uv
        # M_uv[i, pi_uv[i]] = 1.0
        for i in range(d):
            j = pi_uv[i]
            R[u * d + i][v * d + j] = 1.0
            R[v * d + j][u * d + i] = 1.0  # transpose

    # Compute top-d eigenvectors
    top_vecs = power_iteration_top_k(R, d)

    # For each node, form d x d block: Block[i][j] = sum_{m=0}^{d-1} vec_m[v*d + i] * (coordinate j?)
    # In standard spectral sync, U is Kd x d. The v-th block is U_v (d x d), where row i is U[v*d + i, :].
    # Projection onto S_d: find P in S_d maximizing sum_{i} U_v[i, P(i)].
    all_p = all_permutations(d)
    estimated_perms = []

    for v in range(K):
        best_perm = None
        best_score = -1e9
        for p in all_p:
            # Score for permutation p: sum_{i=0}^{d-1} U_v[i, p(i)]
            score = sum(top_vecs[p[i]][v * d + i] for i in range(d))
            if score > best_score:
                best_score = score
                best_perm = p
        estimated_perms.append(best_perm)

    return estimated_perms


def solve_discrete_map(edge_measurements, unary_observations, K, d, lambda_obs=1.0):
    """
    Exact Discrete MAP optimizer over S_d^K.
    Energy = sum_{(u,v)} perm_overlap(P_u, Pi_uv o P_v) + lambda_obs * sum_v perm_overlap(P_v, Obs_v)
    """
    perms = all_permutations(d)
    n_perms = len(perms)

    # Precompute edge overlap table: table[(u,v)][i, j] = overlap(p_i, pi_uv o p_j)
    edge_tables = {}
    for (u, v), pi_uv in edge_measurements.items():
        table = []
        for i, p_u in enumerate(perms):
            row = []
            for j, p_v in enumerate(perms):
                expected_u = compose_perm(pi_uv, p_v)
                row.append(perm_overlap(p_u, expected_u))
            table.append(row)
        edge_tables[(u, v)] = table

    # Precompute unary overlap table
    unary_table = []
    if unary_observations is not None and lambda_obs > 0:
        for v in range(K):
            obs_v = unary_observations[v]
            unary_table.append([lambda_obs * perm_overlap(p, obs_v) for p in perms])

    best_energy = -1e9
    best_assignment = None
    tie_count = 0

    # Exhaustive search over S_d^K
    for config in itertools.product(range(n_perms), repeat=K):
        energy = 0.0
        for (u, v), table in edge_tables.items():
            energy += table[config[u]][config[v]]
        if unary_table:
            for v in range(K):
                energy += unary_table[v][config[v]]

        if energy > best_energy + 1e-9:
            best_energy = energy
            best_assignment = [perms[idx] for idx in config]
            tie_count = 1
        elif abs(energy - best_energy) <= 1e-9:
            tie_count += 1

    return best_assignment, best_energy, tie_count


def evaluate_accuracy(estimated_perms, true_perms, d):
    """
    Accuracy up to global gauge transformation g0 in S_d:
    Acc = max_{g0} (1/K) sum_v I(g0 o est_v == true_v)
    """
    perms = all_permutations(d)
    K = len(estimated_perms)
    best_matches = 0

    for g0 in perms:
        matches = 0
        for est, true in zip(estimated_perms, true_perms):
            if compose_perm(g0, est) == true:
                matches += 1
        if matches > best_matches:
            best_matches = matches

    return best_matches / K


def run_experiment_grid():
    print("=== CD002 EXPERIMENTAL GRID: ADVERSARIAL GAUGE CYCLES ===")
    print(f"{'K':<3} {'d':<3} {'Graph':<9} {'Noise':<14} | {'Spectral':<9} | {'MAP(pure)':<9} {'Ties':<5} | {'MAP(unary)':<10} | {'Raw':<6}")
    print("-" * 75)

    random.seed(42)
    configs = [
        (3, 3, "cycle", "transposition"),
        (3, 3, "cycle", "derangement"),
        (4, 3, "cycle", "transposition"),
        (4, 3, "cycle", "derangement"),
        (4, 3, "complete", "transposition"),
        (5, 3, "cycle", "transposition"),
        (3, 4, "cycle", "transposition"),
    ]

    results = []
    for K, d, graph_type, noise_type in configs:
        perms = all_permutations(d)
        true_perms = [random.choice(perms) for _ in range(K)]

        # Edges
        edges = []
        if graph_type == "cycle":
            for i in range(K):
                edges.append((i, (i + 1) % K))
        elif graph_type == "complete":
            for i in range(K):
                for j in range(i + 1, K):
                    edges.append((i, j))

        # Ground truth measurements
        measurements = {}
        for u, v in edges:
            measurements[(u, v)] = compose_perm(true_perms[u], invert_perm(true_perms[v]))

        # Adversarial corruption on edge (0, 1)
        if noise_type == "transposition":
            tau = list(range(d))
            tau[0], tau[1] = tau[1], tau[0]
            tau = tuple(tau)
        elif noise_type == "derangement":
            tau = tuple((i + 1) % d for i in range(d))

        corrupted_edge = (0, 1)
        measurements[corrupted_edge] = compose_perm(tau, measurements[corrupted_edge])

        # Noisy unary observations
        unary_obs = []
        for p in true_perms:
            p_noisy = list(p)
            p_noisy[0], p_noisy[1] = p_noisy[1], p_noisy[0]
            unary_obs.append(tuple(p_noisy))

        # 1. Spectral Sync
        spec_perms = solve_spectral_sync(measurements, K, d)
        acc_spec = evaluate_accuracy(spec_perms, true_perms, d)

        # 2. Discrete MAP (pure pairwise, no unary)
        map_pure, energy_pure, ties_pure = solve_discrete_map(measurements, None, K, d, lambda_obs=0.0)
        acc_map_pure = evaluate_accuracy(map_pure, true_perms, d)

        # 3. Discrete MAP with unary
        map_unary, energy_unary, ties_unary = solve_discrete_map(measurements, unary_obs, K, d, lambda_obs=1.0)
        acc_map_unary = evaluate_accuracy(map_unary, true_perms, d)

        acc_raw = evaluate_accuracy(unary_obs, true_perms, d)

        print(f"{K:<3} {d:<3} {graph_type:<9} {noise_type:<14} | {acc_spec:<9.3f} | {acc_map_pure:<9.3f} {ties_pure:<5} | {acc_map_unary:<10.3f} | {acc_raw:<6.3f}")
        results.append({
            "K": K, "d": d, "graph": graph_type, "noise": noise_type,
            "acc_spec": acc_spec, "acc_map_pure": acc_map_pure, "ties_pure": ties_pure,
            "acc_map_unary": acc_map_unary, "acc_raw": acc_raw
        })

    return results


def test_self_check():
    d = 3
    p1 = (1, 0, 2)
    inv = invert_perm(p1)
    assert compose_perm(p1, inv) == (0, 1, 2)
    assert perm_overlap(p1, p1) == 3
    assert perm_overlap(p1, (0, 1, 2)) == 1

    # Clean 3-cycle
    K = 3
    true_perms = [(0, 1, 2), (1, 2, 0), (2, 0, 1)]
    meas = {
        (0, 1): compose_perm(true_perms[0], invert_perm(true_perms[1])),
        (1, 2): compose_perm(true_perms[1], invert_perm(true_perms[2])),
        (2, 0): compose_perm(true_perms[2], invert_perm(true_perms[0])),
    }
    # Discrete MAP on clean cycle should have 100% accuracy
    m, e, ties = solve_discrete_map(meas, None, K, d, lambda_obs=0.0)
    acc = evaluate_accuracy(m, true_perms, d)
    assert acc == 1.0, f"Clean MAP accuracy failed: {acc}"
    print("CD002 witness self-check: PASS (3/3)")


if __name__ == "__main__":
    test_self_check()
    if len(sys.argv) > 1 and sys.argv[1] == "--check":
        sys.exit(0)
    run_experiment_grid()
