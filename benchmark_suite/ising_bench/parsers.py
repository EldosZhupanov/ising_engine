"""Parsers for every supported benchmark file format.

Each parser takes ``(text, name)`` and returns a standardized ``Graph`` /
``Qubo`` (or a list of them for multi-problem files). Direction conventions
(minimize vs. maximize) are encoded per format and mirror the Rust engine's
``src/benchmark/instances.rs`` so results agree across languages.

Formats:

- ``rudy``           — weighted MaxCut edge list, header ``n m`` (G-Set,
                       Biq Mac mac/ising sets, Optsicom sets).
- ``dimacs``         — DIMACS graph format (``c`` comments, ``p`` header,
                       ``e u v [w]`` edges).
- ``maxcut_auto``    — sniffs rudy vs. dimacs on the first data line.
- ``snap_edgelist``  — SNAP whitespace edge lists with ``#`` comments and
                       arbitrary node ids (remapped densely, deduped,
                       unweighted → w = 1).
- ``orlib_bqp``      — Beasley OR-Library multi-problem BQP files
                       (maximize x'Qx; first token = problem count).
- ``biqmac_sparse``  — single-instance Beasley-style sparse BQP as
                       distributed in Biq Mac (bqp*.sparse / gka*.sparse;
                       maximize convention, diagonal = linear).
- ``qplib``          — QPLIB unconstrained binary quadratic instances only
                       (constrained / non-binary instances are refused, not
                       silently mis-modelled).
"""

from __future__ import annotations

from .models import Graph, Qubo


def _tokens(line: str):
    """Strip ``#``/``%`` comments and split on whitespace."""
    for marker in ("#", "%"):
        idx = line.find(marker)
        if idx >= 0:
            line = line[:idx]
    return line.split()


# ---------------------------------------------------------------------------
# Graph formats
# ---------------------------------------------------------------------------


def parse_rudy(text: str, name: str) -> Graph:
    """rudy weighted-MaxCut: header ``n m``, then ``m`` lines ``i j w``
    (1-indexed; missing weight defaults to 1)."""
    lines = (t for t in (map(_tokens, text.splitlines())) if t)
    header = next(lines, None)
    if header is None or len(header) < 2:
        raise ValueError(f"{name}: missing rudy header")
    n = int(header[0])
    edges = {}
    for t in lines:
        if len(t) < 2:
            continue
        u, v = int(t[0]), int(t[1])
        w = float(t[2]) if len(t) > 2 else 1.0
        if not (1 <= u <= n and 1 <= v <= n):
            raise ValueError(f"{name}: vertex out of range ({u},{v}) for n={n}")
        if u == v:
            continue
        key = (min(u, v) - 1, max(u, v) - 1)
        edges[key] = edges.get(key, 0.0) + w
    g = Graph(name=name, n=n, edges=[(u, v, w) for (u, v), w in sorted(edges.items())])
    g.validate()
    return g


def parse_dimacs(text: str, name: str) -> Graph:
    """DIMACS: ``c`` comments, ``p <fmt> n m`` header, ``e u v [w]`` edges."""
    n = None
    edges = {}
    for raw in text.splitlines():
        t = raw.split()
        if not t or t[0] == "c":
            continue
        if t[0] == "p":
            n = int(t[2])
        elif t[0] == "e":
            if n is None:
                raise ValueError(f"{name}: DIMACS edge before 'p' header")
            u, v = int(t[1]), int(t[2])
            w = float(t[3]) if len(t) > 3 else 1.0
            if not (1 <= u <= n and 1 <= v <= n):
                raise ValueError(f"{name}: vertex out of range ({u},{v})")
            if u == v:
                continue
            key = (min(u, v) - 1, max(u, v) - 1)
            edges[key] = edges.get(key, 0.0) + w
    if n is None:
        raise ValueError(f"{name}: no DIMACS 'p' header")
    g = Graph(name=name, n=n, edges=[(u, v, w) for (u, v), w in sorted(edges.items())])
    g.validate()
    return g


def parse_maxcut_auto(text: str, name: str) -> Graph:
    """Sniff rudy vs. DIMACS on the first non-comment line."""
    for raw in text.splitlines():
        t = raw.split()
        if not t or t[0].startswith(("#", "%")):
            continue
        if t[0] in ("c", "p", "e"):
            return parse_dimacs(text, name)
        return parse_rudy(text, name)
    raise ValueError(f"{name}: empty graph file")


def parse_snap_edgelist(text: str, name: str) -> Graph:
    """SNAP edge list: ``#`` comments, ``u v`` pairs with arbitrary ids.
    Ids are remapped densely in first-appearance order (deterministic);
    duplicate/reverse edges are deduped; self-loops dropped; w = 1."""
    remap = {}
    edges = set()

    def vid(x: str) -> int:
        if x not in remap:
            remap[x] = len(remap)
        return remap[x]

    for raw in text.splitlines():
        t = _tokens(raw)
        if len(t) < 2:
            continue
        u, v = vid(t[0]), vid(t[1])
        if u == v:
            continue
        edges.add((min(u, v), max(u, v)))
    if not remap:
        raise ValueError(f"{name}: empty SNAP edge list")
    g = Graph(
        name=name,
        n=len(remap),
        edges=[(u, v, 1.0) for (u, v) in sorted(edges)],
    )
    g.validate()
    return g


# ---------------------------------------------------------------------------
# QUBO formats
# ---------------------------------------------------------------------------


def _bqp_problem(tok, name: str) -> Qubo:
    """One Beasley-style BQP problem from a token iterator: ``n nnz`` then
    ``nnz`` lines ``i j q`` (1-indexed). Objective is MAXIMIZE x'Qx with Q
    SYMMETRIC and only one triangle listed, so each off-diagonal entry
    contributes TWICE (q_ij x_i x_j + q_ji x_j x_i); diagonal = linear.
    Verified against the published optimum of bqp50 #1 (2098). The stored
    minimization energy is the objective's negation."""
    n = int(next(tok))
    nnz = int(next(tok))
    linear = [0.0] * n
    quad = {}
    for _ in range(nnz):
        i = int(next(tok))
        j = int(next(tok))
        q = float(next(tok))
        if not (1 <= i <= n and 1 <= j <= n):
            raise ValueError(f"{name}: index out of range ({i},{j}) for n={n}")
        if i == j:
            linear[i - 1] -= q
        else:
            key = (min(i, j) - 1, max(i, j) - 1)
            quad[key] = quad.get(key, 0.0) - 2.0 * q
    qubo = Qubo(name=name, n=n, linear=linear, quad=quad, maximize=True)
    qubo.validate()
    return qubo


def parse_orlib_bqp(text: str, name: str) -> list:
    """OR-Library multi-problem BQP file: first token = problem count."""
    tok = iter(text.split())
    count = int(next(tok))
    out = []
    for k in range(count):
        out.append(_bqp_problem(tok, f"{name}.{k + 1}"))
    return out


def parse_biqmac_sparse(text: str, name: str) -> Qubo:
    """Biq Mac single-instance sparse BQP: ``n nnz`` then ``i j q`` with Q
    symmetric, one triangle listed (off-diagonals count twice). Unlike the
    OR-Library originals, Biq Mac's converted be/gka files are for
    MINIMIZATION of x'Qx (their docs: "in the OR-library the problems are
    given for maximization!"). Verified: gka1a minimum = -3414 exactly
    matches the published optimum."""
    tok = iter(text.split())
    n = int(next(tok))
    nnz = int(next(tok))
    linear = [0.0] * n
    quad = {}
    for _ in range(nnz):
        i = int(next(tok))
        j = int(next(tok))
        q = float(next(tok))
        if not (1 <= i <= n and 1 <= j <= n):
            raise ValueError(f"{name}: index out of range ({i},{j}) for n={n}")
        if i == j:
            linear[i - 1] += q
        else:
            key = (min(i, j) - 1, max(i, j) - 1)
            quad[key] = quad.get(key, 0.0) + 2.0 * q
    qubo = Qubo(name=name, n=n, linear=linear, quad=quad, maximize=False)
    qubo.validate()
    return qubo


def parse_qplib(text: str, name: str) -> Qubo:
    """QPLIB unconstrained binary quadratic: objective ½x'Qx + b'x + c.
    Refuses constrained or non-binary instances (their optimum would change
    if constraints were dropped)."""
    lines = iter([t for t in (raw.split() for raw in text.splitlines()) if t])

    def next_line(what):
        t = next(lines, None)
        if t is None:
            raise ValueError(f"{name}: missing {what}")
        return t

    next_line("name")
    # 3-char problem type: objective / VARIABLES / CONSTRAINTS (QPLIB spec
    # §4.1) — e.g. "QBN" = quadratic objective, binary vars, no constraints.
    # The file layout is adaptive: a constraint-count line exists ONLY when
    # the constraints character is not 'N', so for the unconstrained
    # instances we accept there is no 'm' line at all.
    ptype = next_line("type")[0]
    if len(ptype) < 3 or ptype[1].upper() != "B":
        raise ValueError(f"{name}: QPLIB type {ptype} is not all-binary (unsupported)")
    if ptype[2].upper() != "N":
        raise ValueError(f"{name}: QPLIB type {ptype} is constrained; unsupported")
    sense = next_line("sense")[0].lower()
    maximize = sense == "maximize"
    obj_sign = -1.0 if maximize else 1.0
    n = int(next_line("n")[0])
    nq = int(next_line("nq")[0])
    linear = [0.0] * n
    quad = {}
    for _ in range(nq):
        t = next_line("quad term")
        i, j, q = int(t[0]), int(t[1]), float(t[2])
        if not (1 <= i <= n and 1 <= j <= n):
            raise ValueError(f"{name}: quad index out of range")
        # Objective = ½·Σ_listed q_ij x_i x_j + b'x + c: every LISTED entry
        # (diagonal and off-diagonal alike) contributes with factor ½ and is
        # NOT symmetric-doubled. Verified exactly against the official
        # QPLIB_3565 solution file (objective 282.0000).
        if i == j:
            # ½ q x_i² = ½ q x_i for binary x.
            linear[i - 1] += obj_sign * 0.5 * q
        else:
            key = (min(i, j) - 1, max(i, j) - 1)
            quad[key] = quad.get(key, 0.0) + obj_sign * 0.5 * q
    b_default = float(next_line("b_default")[0])
    for i in range(n):
        linear[i] += obj_sign * b_default
    nb = int(next_line("nb")[0])
    for _ in range(nb):
        t = next_line("linear term")
        i, v = int(t[0]), float(t[1])
        if not (1 <= i <= n):
            raise ValueError(f"{name}: linear index out of range")
        linear[i - 1] += obj_sign * (v - b_default)
    t = next(lines, None)
    c = float(t[0]) if t else 0.0
    qubo = Qubo(
        name=name,
        n=n,
        linear=linear,
        quad=quad,
        offset=obj_sign * c,
        maximize=maximize,
    )
    qubo.validate()
    return qubo


#: parser-name → callable registry used by the loader and validators.
PARSERS = {
    "rudy": parse_rudy,
    "dimacs": parse_dimacs,
    "maxcut_auto": parse_maxcut_auto,
    "snap_edgelist": parse_snap_edgelist,
    "orlib_bqp": parse_orlib_bqp,
    "biqmac_sparse": parse_biqmac_sparse,
    "qplib": parse_qplib,
}
