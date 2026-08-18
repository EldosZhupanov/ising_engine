# memory/DECISIONS.md — pointer

**Authoritative source:** `../research/architecture/ADR/` — read the ADR itself
before any architectural work. Query the graph with
`python ../research/architecture/query_graph.py`.

Record a new ADR when you change structure, a boundary, or a cross-cutting
guarantee. An ADR states: context, the decision, alternatives considered,
consequences, and links to superseded/related ADRs.

## The nine, in one line each

| ADR | Decision |
|---|---|
| [0000](../research/architecture/ADR/ADR-0000-adr-system-and-knowledge-graph.md) | Architecture Decision Records with typed knowledge-graph edges |
| [0001](../research/architecture/ADR/ADR-0001-operators-not-algorithms.md) | The engine is built from **Operators, not Algorithms** |
| [0002](../research/architecture/ADR/ADR-0002-two-state-backends.md) | Two state backends (DenseByte, SparseBitSlice) behind one interface |
| [0003](../research/architecture/ADR/ADR-0003-cache-locality-outranks-flops.md) | **Cache locality outranks FLOPS** in all layout and kernel decisions |
| [0004](../research/architecture/ADR/ADR-0004-reproducibility-mandatory.md) | **Bit-identical reproducibility is mandatory, not optional** |
| [0005](../research/architecture/ADR/ADR-0005-deterministic-certified-lowering.md) | Lowering passes are deterministic and emit energy-preservation certificates |
| [0006](../research/architecture/ADR/ADR-0006-amendment-presumed-fixed.md) | Amendment I: the architectural vector is presumed fixed, not dogmatically fixed |
| [0007](../research/architecture/ADR/ADR-0007-four-track-program.md) | Four parallel tracks: Engine, Research, Benchmark, Verification |
| [0008](../research/architecture/ADR/ADR-0008-experiment-infrastructure.md) | Experiment infrastructure — one command, full report, black-box engines |

## The two that bite most often

**ADR-0004 (reproducibility).** Same seed ⇒ same trajectory ⇒ same energy, and
every adaptive feature is opt-in and bit-identical on replay. This is why
FP contraction (`mul_add`), reduced-precision RNG comparisons, and reordered
float sums are **rejected by default** — they change trajectories, so they are
behaviour-changing and need explicit approval, no matter how much faster they are.

**ADR-0001 (operators, not algorithms).** Operators are selected through their
capability passport, never by name, so a new operator is usable the moment it is
registered. Its corollary is easy to miss: a capability with **no implementing
operator can never be selected**, and will stay empty silently. Three of six
capabilities were in that state for the entire history of the project —
`ExactInference`, `Approximate` and `Warmstart` still have zero implementations.
