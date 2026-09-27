"""MQ-QUAL-001 synthetic cases frozen by the prospective amendment."""


def cases():
    out = [
        {"offset": 7, "linear": [0], "pairs": []},
        {"offset": -2, "linear": [2, -3, 0], "pairs": []},
        {"offset": 0, "linear": [-2, -2, -2], "pairs": [[0, 1, 2], [0, 2, 2], [1, 2, 2]]},
        {"offset": 3.25, "linear": [0.5, -1.25, 0, 0], "pairs": [[0, 1, 2.5], [1, 2, -0.75]]},
    ]
    for n in range(1, 9):
        out.append({"offset": (n - 4) / 4,
                    "linear": [((7*i + 3*n) % 17 - 8) / 4 for i in range(n)],
                    "pairs": [[i, j, ((11*i + 5*j + n) % 19 - 9) / 4]
                              for i in range(n) for j in range(i + 1, n)]})
    return out
