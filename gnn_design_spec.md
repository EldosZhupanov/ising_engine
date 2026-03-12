🦾 [Superpowers] Socratic Design Spec: Hybrid GNN-QUBO Solver
## Objective
Integrate a Graph Neural Network (GNN) heuristic to 'warm-start' the Ising Engine.

## Why?
Simulated annealing starts from a random uniform distribution of spins (-1, 1). For NP-hard problems, this means the solver wastes thousands of sweeps just finding the 'general valley' of the energy landscape. A GNN can predict the likely ground-state distribution of nodes based on graph topology instantly.

## Architecture (Rust + PyTorch/ONNX)
1. **GNN Model (Python/PyTorch):** Trains on historical solved QUBO matrices. Learns that certain structural motifs (e.g., highly connected cliques with negative weights) tend to align in specific spin directions.
2. **Inference Bridge (Rust  or ):** The Rust engine takes a new QUBO, passes its edge weights into the pre-trained ONNX model.
3. **Warm Start:** The model outputs a probability vector (e.g., Node 5 has 90% chance to be '1').
4. **Hybrid Initialization:** Instead of , the replicas initialize their starting states biased by the GNN's probabilities.

## Phase 1 Implementation Plan
- We will build the **GNN Probability Injector** trait in Rust first.
- We will mock the ML model output with a heuristic function to prove the pipeline works.
- We will update  to accept .
