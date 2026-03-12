🦾 [Superpowers] Socratic Design Spec: Enterprise Architecture

## Pillar 1: REST API Server (SaaS)
- Framework: axum + tokio (already in Cargo.toml)
- Endpoint: POST /solve/qubo
- Purpose: Allows external clients to send JSON matrices and receive Ground State arrays instantly.

## Pillar 2: Web3 Mempool Connector
- Framework: ethers-rs (already in Cargo.toml)
- Mechanism: WebSocket Provider listening to new pending transactions.
- Purpose: Feeds live DeFi pool data directly into the API or Engine for MEV.

## Pillar 3: SIMD Acceleration (CPU limit push)
- Note: Native GPU (CUDA) requires environment setup (nvcc), so we start with rust native parallel/SIMD via core::arch or Rayon optimization.
- Action: We will create a new binary or service module housing these.

