"""Standardized instance objects shared by every parser and consumer.

Two canonical forms:

- ``Graph``  — an undirected weighted graph (MaxCut instances).
- ``Qubo``   — minimize E(x) = offset + Σ l_i x_i + Σ_{i<j} q_ij x_i x_j
               over x ∈ {0,1}^n. ``maximize=True`` marks instances whose
               *native* objective is a maximization equal to ``-E`` (MaxCut
               value, ORLIB BQP objective), matching the Rust engine's
               convention exactly so cross-language results agree.

The MaxCut → QUBO mapping mirrors ``src/benchmark/instances.rs``:
cut(S) = Σ w_ij [x_i ≠ x_j] and [x_i ≠ x_j] = x_i + x_j − 2 x_i x_j, so
minimizing E = −cut gives q_ij = 2 w_ij and l_i = −Σ_j w_ij.
"""

from __future__ import annotations

import math
from dataclasses import dataclass, field


@dataclass
class Graph:
    """Undirected weighted graph; vertices are 0-indexed."""

    name: str
    n: int
    edges: list  # list[(u, v, w)] with u < v
    meta: dict = field(default_factory=dict)

    @property
    def m(self) -> int:
        return len(self.edges)

    def cut_value(self, x) -> float:
        """MaxCut objective of a 0/1 side assignment."""
        return sum(w for (u, v, w) in self.edges if x[u] != x[v])

    def to_qubo(self) -> "Qubo":
        """Minimization QUBO with E = -cut (native objective maximized)."""
        linear = [0.0] * self.n
        quad = {}
        for (u, v, w) in self.edges:
            quad[(u, v)] = quad.get((u, v), 0.0) + 2.0 * w
            linear[u] -= w
            linear[v] -= w
        return Qubo(
            name=self.name,
            n=self.n,
            linear=linear,
            quad=quad,
            offset=0.0,
            maximize=True,
            meta=dict(self.meta, kind="maxcut"),
        )

    def validate(self):
        """Raise ValueError on any structural inconsistency."""
        if self.n <= 0:
            raise ValueError(f"{self.name}: n must be positive, got {self.n}")
        for (u, v, w) in self.edges:
            if not (0 <= u < v < self.n):
                raise ValueError(f"{self.name}: bad edge ({u},{v}) for n={self.n}")
            if not math.isfinite(w):
                raise ValueError(f"{self.name}: non-finite weight on ({u},{v})")


@dataclass
class Qubo:
    """Binary quadratic minimization instance (see module docstring)."""

    name: str
    n: int
    linear: list  # list[float], length n
    quad: dict  # {(i, j): w} with i < j
    offset: float = 0.0
    maximize: bool = False
    best_known: float = None  # native units, if published
    meta: dict = field(default_factory=dict)

    def energy(self, x) -> float:
        e = self.offset
        for i, l in enumerate(self.linear):
            if x[i]:
                e += l
        for (i, j), w in self.quad.items():
            if x[i] and x[j]:
                e += w
        return e

    def native_objective(self, energy: float) -> float:
        """Natural-units score for a given minimization energy."""
        return -energy if self.maximize else energy

    def to_qubo(self) -> "Qubo":
        return self

    @property
    def nnz(self) -> int:
        return len(self.quad)

    def validate(self):
        if self.n <= 0:
            raise ValueError(f"{self.name}: n must be positive, got {self.n}")
        if len(self.linear) != self.n:
            raise ValueError(f"{self.name}: linear length {len(self.linear)} != n {self.n}")
        for l in self.linear:
            if not math.isfinite(l):
                raise ValueError(f"{self.name}: non-finite linear term")
        for (i, j), w in self.quad.items():
            if not (0 <= i < j < self.n):
                raise ValueError(f"{self.name}: bad quad index ({i},{j}) for n={self.n}")
            if not math.isfinite(w):
                raise ValueError(f"{self.name}: non-finite quad term ({i},{j})")
