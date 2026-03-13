🦾 [Superpowers] Socratic Design Spec: HUBO 3-SAT Benchmark
## Objective
Demonstrate the superiority of the new native HUBO (Hypergraph) architecture by solving a hard 3-SAT problem. We will compare the native Edge3 formulation against the classic QUBO reduction (which requires auxiliary variables).

## Why 3-SAT?
3-SAT is the canonical NP-complete problem. Each clause connects 3 variables: (A OR B OR C). In traditional QUBO, linking 3 variables requires adding a 'dummy' variable and extra edges, inflating the matrix size. With our new HUBO, we map it directly as an `Edge3` hyperedge.

## Benchmark Implementation
1. Generate a random hard 3-SAT instance (clause-to-variable ratio ~ 4.26, the phase transition boundary).
2. Compile it using `LogicBuilder::add_or_gate` (which internally uses Edge3/Edge4 logic now, or we explicitly build a HUBO constraint).
3. Time the `QuantumField` evolution (QPA Engine).
4. Print the throughput and the exact speedup gained by not using auxiliary variables.
