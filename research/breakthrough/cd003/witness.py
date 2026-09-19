#!/usr/bin/env python3
"""
CD003 Witness: Precision-Rank Bounded Response Witness.
Evaluates the number of unique conditional responses N_resp vs projection size N_theta
and theoretical bound (2bK + 1)^r across boundary sizes b, precision bounds K, and ranks r.
Pure Python standard library implementation.
"""

import itertools
import math
import sys
import random


def generate_all_binary(length):
    """Generate all 2^length binary vectors as tuples."""
    return list(itertools.product((0, 1), repeat=length))


def compute_projection_image(V, b, r):
    """
    Compute all distinct values of theta(z) = V^T z for z in {0, 1}^b.
    V is a list of r column vectors, each of length b.
    Returns a set of tuple coordinates.
    """
    all_z = generate_all_binary(b)
    image = set()
    for z in all_z:
        theta = tuple(sum(V[k][j] * z[j] for j in range(b)) for k in range(r))
        image.add(theta)
    return image


def find_conditional_ground_state(Q_x, h_x, U, theta, n, r):
    """
    Find internal assignment x in {0, 1}^n minimizing:
      E(x) = x^T Q_x x + h_x^T x + (U^T x)^T theta
    Under deterministic tie-breaking (lowest integer index).
    """
    best_energy = 1e18
    best_x = None

    # Exhaustive search over all 2^n configurations
    for x in generate_all_binary(n):
        # x^T Q_x x
        energy = sum(x[i] * Q_x[i][j] * x[j] for i in range(n) for j in range(n))
        # h_x^T x
        energy += sum(h_x[i] * x[i] for i in range(n))
        # (U^T x)^T theta = sum_{k=0}^{r-1} theta[k] * sum_{i=0}^{n-1} U[i][k] * x[i]
        for k in range(r):
            energy += theta[k] * sum(U[i][k] * x[i] for i in range(n))

        if energy < best_energy - 1e-9:
            best_energy = energy
            best_x = x

    return best_x


def evaluate_instance(b, n, r, K, family_name, seed=42):
    """
    Evaluate a single instance of cross-coupled Ising system.
    Returns dict of metrics.
    """
    random.seed(seed)

    # 1. Construct V (b x r)
    V = []  # list of r columns, each length b
    if family_name == "unit_positive":
        for k in range(r):
            V.append([1] * b)
        K_actual = 1
    elif family_name == "ternary":
        for k in range(r):
            col = [random.choice([-1, 0, 1]) for _ in range(b)]
            # ensure not all zero
            if all(c == 0 for c in col):
                col[0] = 1
            V.append(col)
        K_actual = 1
    elif family_name == "bounded_K":
        for k in range(r):
            col = [random.randint(-K, K) for _ in range(b)]
            if all(c == 0 for c in col):
                col[0] = K
            V.append(col)
        K_actual = K
    elif family_name == "binary_powers":
        # r must be 1
        col = [2**i for i in range(b)]
        V.append(col)
        K_actual = 2**(b - 1)
    else:
        raise ValueError(f"Unknown family {family_name}")

    # 2. Construct internal Hamiltonian Q_x (n x n), h_x (n), and U (n x r)
    # To maximize distinct responses, design U to distinguish different theta values
    Q_x = [[0.0] * n for _ in range(n)]
    for i in range(n):
        for j in range(i + 1, n):
            val = random.choice([-1.0, 0.0, 1.0])
            Q_x[i][j] = val
            Q_x[j][i] = val
    h_x = [random.uniform(-1.0, 1.0) for _ in range(n)]

    U = []
    for i in range(n):
        # row i has r entries
        row = [random.uniform(-2.0, 2.0) for _ in range(r)]
        U.append(row)

    # 3. Compute distinct projection image theta(z)
    all_z = generate_all_binary(b)
    theta_map = {}
    for z in all_z:
        theta = tuple(sum(V[k][j] * z[j] for j in range(b)) for k in range(r))
        theta_map[z] = theta

    unique_thetas = set(theta_map.values())
    N_theta = len(unique_thetas)

    # 4. Compute conditional ground state x*(z) for each z
    unique_responses = set()
    # Cache x*(theta) since it depends only on theta
    theta_to_x = {}
    for theta in unique_thetas:
        x_opt = find_conditional_ground_state(Q_x, h_x, U, theta, n, r)
        theta_to_x[theta] = x_opt

    for z in all_z:
        theta = theta_map[z]
        x_opt = theta_to_x[theta]
        unique_responses.add(x_opt)

    N_resp = len(unique_responses)
    N_raw = len(all_z)  # 2^b

    # Theoretical upper bounds
    # Tight 1D interval bound for each column
    max_thetas_per_col = []
    for k in range(r):
        col = V[k]
        s_min = sum(val for val in col if val < 0)
        s_max = sum(val for val in col if val > 0)
        max_thetas_per_col.append(s_max - s_min + 1)

    tight_bound = 1
    for m in max_thetas_per_col:
        tight_bound *= m

    general_bound = (2 * b * K_actual + 1)**r

    # Invariant verification
    assert N_resp <= N_theta, f"VIOLATION: N_resp ({N_resp}) > N_theta ({N_theta})"
    assert N_theta <= tight_bound, f"VIOLATION: N_theta ({N_theta}) > tight_bound ({tight_bound})"
    assert tight_bound <= general_bound, f"VIOLATION: tight_bound ({tight_bound}) > general_bound ({general_bound})"

    return {
        "b": b,
        "n": n,
        "r": r,
        "K": K_actual,
        "family": family_name,
        "N_raw": N_raw,
        "N_theta": N_theta,
        "N_resp": N_resp,
        "tight_bound": tight_bound,
        "general_bound": general_bound,
        "compression_ratio": N_resp / N_raw,
    }


def run_test_suite():
    print("=== CD003 EXPERIMENTAL GRID: PRECISION-RANK BOUNDED RESPONSE ===")
    print(f"{'b':<3} {'r':<2} {'K':<5} {'Family':<15} | {'2^b':<6} {'Bound':<8} {'N_theta':<8} {'N_resp':<8} | {'Compression':<11} | {'Status':<6}")
    print("-" * 80)

    test_grid = [
        # (b, n, r, K, family)
        (2, 4, 1, 1, "unit_positive"),
        (4, 4, 1, 1, "unit_positive"),
        (6, 6, 1, 1, "unit_positive"),
        (8, 6, 1, 1, "unit_positive"),
        (4, 4, 1, 1, "ternary"),
        (6, 6, 1, 1, "ternary"),
        (8, 6, 1, 1, "ternary"),
        (4, 4, 1, 2, "bounded_K"),
        (6, 6, 1, 3, "bounded_K"),
        (4, 4, 1, 8, "binary_powers"),
        (6, 6, 1, 32, "binary_powers"),
        (3, 4, 2, 1, "unit_positive"),
        (4, 4, 2, 1, "ternary"),
        (5, 5, 2, 1, "ternary"),
    ]

    results = []
    for b, n, r, K, family in test_grid:
        res = evaluate_instance(b, n, r, K, family)
        results.append(res)
        status = "PASS" if res["N_resp"] <= res["N_theta"] <= res["tight_bound"] else "FAIL"
        comp_str = f"{res['compression_ratio']:.3f}"
        print(f"{res['b']:<3} {res['r']:<2} {res['K']:<5} {res['family']:<15} | {res['N_raw']:<6} {res['tight_bound']:<8} {res['N_theta']:<8} {res['N_resp']:<8} | {comp_str:<11} | {status:<6}")

    print("-" * 80)
    print("All mathematical invariants strictly verified across all grid configurations.")
    return results


def test_witness_self_check():
    """Unit tests for witness logic."""
    # Test binary generation
    bins = generate_all_binary(3)
    assert len(bins) == 8
    # Test projection
    V = [[1, 1, 1]]
    img = compute_projection_image(V, 3, 1)
    assert img == {(0,), (1,), (2,), (3,)}, f"Expected 4 values, got {img}"
    # Test instance
    res = evaluate_instance(3, 3, 1, 1, "unit_positive")
    assert res["N_theta"] == 4, f"Expected N_theta=4, got {res['N_theta']}"
    assert res["N_resp"] <= 4, f"N_resp={res['N_resp']} cannot exceed 4"
    print("CD003 witness self-check: PASS (3/3)")


if __name__ == "__main__":
    test_witness_self_check()
    if len(sys.argv) > 1 and sys.argv[1] == "--check":
        sys.exit(0)
    run_test_suite()
