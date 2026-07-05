use ethers::prelude::*;
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

// Import our Ising Engine directly (zero-copy, in-memory)
use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::ultimate::UltimateSolver;

// =============================================================================
// TOKEN REGISTRY — All tracked tokens on Base
// =============================================================================
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Token {
    WETH,
    USDC,
    CbBTC,
    VIRTUAL,
}

impl Token {
    fn decimals(&self) -> u8 {
        match self {
            Token::WETH => 18,
            Token::USDC => 6,
            Token::CbBTC => 8,
            Token::VIRTUAL => 18,
        }
    }

    fn symbol(&self) -> &'static str {
        match self {
            Token::WETH => "WETH",
            Token::USDC => "USDC",
            Token::CbBTC => "cbBTC",
            Token::VIRTUAL => "VIRTUAL",
        }
    }

    fn index(&self) -> usize {
        match self {
            Token::WETH => 0,
            Token::USDC => 1,
            Token::CbBTC => 2,
            Token::VIRTUAL => 3,
        }
    }
}

const ALL_TOKENS: [Token; 4] = [Token::WETH, Token::USDC, Token::CbBTC, Token::VIRTUAL];
const NUM_TOKENS: usize = 4;

// =============================================================================
// POOL REGISTRY — Top liquid V3 pools (from GeckoTerminal live API)
// =============================================================================
struct PoolInfo {
    address: Address,
    token0: Token,
    token1: Token,
    fee_label: &'static str,
    fee_bps: f64, // fee in basis points (e.g. 30.0 = 0.3%)
}

fn build_pool_registry() -> Vec<PoolInfo> {
    vec![
        // WETH/USDC 0.3% — $50M 24h volume
        PoolInfo {
            address: "0x6c561b446416e1a00e8e93e221854d6ea4171372"
                .parse()
                .unwrap(),
            token0: Token::WETH,
            token1: Token::USDC,
            fee_label: "0.3%",
            fee_bps: 30.0,
        },
        // WETH/USDC 0.05% — $18M 24h volume
        PoolInfo {
            address: "0xd0b53d9277642d899df5c87a3966a349a798f224"
                .parse()
                .unwrap(),
            token0: Token::WETH,
            token1: Token::USDC,
            fee_label: "0.05%",
            fee_bps: 5.0,
        },
        // WETH/USDC 0.01% — $4M 24h volume
        PoolInfo {
            address: "0xb4cb800910b228ed3d0834cf79d697127bbb00e5"
                .parse()
                .unwrap(),
            token0: Token::WETH,
            token1: Token::USDC,
            fee_label: "0.01%",
            fee_bps: 1.0,
        },
        // cbBTC/USDC 0.05% — $10M 24h volume
        PoolInfo {
            address: "0xfbb6eed8e7aa03b138556eedaf5d271a5e1e43ef"
                .parse()
                .unwrap(),
            token0: Token::USDC,
            token1: Token::CbBTC,
            fee_label: "0.05%",
            fee_bps: 5.0,
        },
        // cbBTC/WETH 0.05% — $3.3M 24h volume
        PoolInfo {
            address: "0x7aea2e8a3843516afa07293a10ac8e49906dabd1"
                .parse()
                .unwrap(),
            token0: Token::WETH,
            token1: Token::CbBTC,
            fee_label: "0.05%",
            fee_bps: 5.0,
        },
        // cbBTC/WETH 0.3% — $1.1M 24h volume
        PoolInfo {
            address: "0x8c7080564b5a792a33ef2fd473fba6364d5495e5"
                .parse()
                .unwrap(),
            token0: Token::WETH,
            token1: Token::CbBTC,
            fee_label: "0.3%",
            fee_bps: 30.0,
        },
        // cbBTC/USDC 0.3% — $560K 24h volume
        PoolInfo {
            address: "0xec558e484cc9f2210714e345298fdc53b253c27d"
                .parse()
                .unwrap(),
            token0: Token::USDC,
            token1: Token::CbBTC,
            fee_label: "0.3%",
            fee_bps: 30.0,
        },
        // VIRTUAL/WETH 0.05% — $2.7M 24h volume
        PoolInfo {
            address: "0x9c087eb773291e50cf6c6a90ef0f4500e349b903"
                .parse()
                .unwrap(),
            token0: Token::VIRTUAL,
            token1: Token::WETH,
            fee_label: "0.05%",
            fee_bps: 5.0,
        },
        // VIRTUAL/USDC 0.3% — $1.8M 24h volume
        PoolInfo {
            address: "0x529d2863a1521d0b57db028168fde2e97120017c"
                .parse()
                .unwrap(),
            token0: Token::VIRTUAL,
            token1: Token::USDC,
            fee_label: "0.3%",
            fee_bps: 30.0,
        },
    ]
}

// =============================================================================
// PRICE MATRIX — stores live rates between all token pairs
// =============================================================================
// Key: (pool_address, token_from, token_to) → rate (how many token_to per 1 token_from)
type PriceMatrix = HashMap<(Address, Token, Token), f64>;

/// Compute exchange rate from sqrtPriceX96
fn compute_rates_from_sqrt_price(sqrt_price_x96: U256, token0: Token, token1: Token) -> (f64, f64) {
    let sqrt_f: f64 = sqrt_price_x96.to_string().parse::<f64>().unwrap_or(0.0);
    let two_96: f64 = 2.0_f64.powi(96);
    let ratio = sqrt_f / two_96;
    let price_raw = ratio * ratio;
    let dec_diff = token0.decimals() as i32 - token1.decimals() as i32;
    let adjustment = 10.0_f64.powi(dec_diff);
    let rate_0_to_1 = price_raw * adjustment;
    let rate_1_to_0 = if rate_0_to_1 > 0.0 {
        1.0 / rate_0_to_1
    } else {
        0.0
    };
    (rate_0_to_1, rate_1_to_0)
}

/// Query slot0() directly from a V3 pool contract using low-level eth_call
async fn fetch_initial_pool_price<M: Middleware + 'static>(
    provider: &M,
    pool_addr: Address,
) -> Result<U256, Box<dyn std::error::Error>> {
    let tx = TransactionRequest::default()
        .to(pool_addr)
        .data(Bytes::from(vec![0x38, 0x50, 0xc7, 0xbd])); // slot0() selector

    let res = provider.call(&tx.into(), None).await?;
    if res.len() >= 32 {
        let sqrt_price_x96 = U256::from_big_endian(&res[0..32]);
        Ok(sqrt_price_x96)
    } else {
        Err("Invalid slot0 response length".into())
    }
}

// =============================================================================
// QUBO COMPILER — Translates live rates into QUBO for Ising Solver
// =============================================================================
struct QuboCompileResult {
    model: QuboModel,
    edges: Vec<(usize, usize, usize)>, // (pool_idx, from_token_idx, to_token_idx) for each QUBO variable
    num_vars: usize,
}

/// Compiles a rate matrix into a QUBO model using per-pool edges.
/// Each binary variable x_k represents "activate trade on pool_idx from token i → token j".
/// Objective: minimize Σ -log(rate_k) * scale * x_k  (negative log = profit maximization)
/// Constraint: flow conservation at each token node (sum_in = sum_out), penalized quadratically.
fn compile_qubo(
    pools: &[PoolInfo],
    pm: &PriceMatrix,
    penalty: f64,
    scale: f64,
    hop_penalty: f64,
) -> QuboCompileResult {
    // Build list of active directed edges (QUBO variables)
    let mut edges: Vec<(usize, usize, usize)> = Vec::new();
    for (pool_idx, pool) in pools.iter().enumerate() {
        // Direction 0: token0 -> token1
        if let Some(&rate) = pm.get(&(pool.address, pool.token0, pool.token1)) {
            if rate > 0.0 {
                edges.push((pool_idx, pool.token0.index(), pool.token1.index()));
            }
        }
        // Direction 1: token1 -> token0
        if let Some(&rate) = pm.get(&(pool.address, pool.token1, pool.token0)) {
            if rate > 0.0 {
                edges.push((pool_idx, pool.token1.index(), pool.token0.index()));
            }
        }
    }

    let m = edges.len();
    let mut var_idx: HashMap<(usize, usize, usize), usize> = HashMap::new();
    for (k, &edge) in edges.iter().enumerate() {
        var_idx.insert(edge, k);
    }

    let mut linear = vec![0.0_f64; m];
    let mut quad_entries: Vec<(usize, usize, f64)> = Vec::new();

    // 1. Log-Profit Objective & Hop Penalty: minimize -log(rate) * scale + hop_penalty
    for (k, &(pool_idx, from_idx, to_idx)) in edges.iter().enumerate() {
        let pool = &pools[pool_idx];
        let rate = pm
            .get(&(pool.address, ALL_TOKENS[from_idx], ALL_TOKENS[to_idx]))
            .copied()
            .unwrap_or(0.0);
        if rate > 0.0 {
            let log_return = rate.ln();
            linear[k] += -log_return * scale;
        }
        // Apply linear Hop Penalty!
        linear[k] += hop_penalty;
    }

    // 2. Flow Conservation Constraint: sum_in - sum_out = 0 for each token
    // Penalized as: penalty * (sum_in - sum_out)^2
    for token_k in 0..NUM_TOKENS {
        let in_edges: Vec<usize> = edges
            .iter()
            .enumerate()
            .filter_map(|(k, &(_, _, to))| if to == token_k { Some(k) } else { None })
            .collect();
        let out_edges: Vec<usize> = edges
            .iter()
            .enumerate()
            .filter_map(|(k, &(_, from, _))| if from == token_k { Some(k) } else { None })
            .collect();

        // Linear: penalty for each edge variable
        for &u in &in_edges {
            linear[u] += penalty;
        }
        for &v in &out_edges {
            linear[v] += penalty;
        }

        // Pairwise incoming: +2 * penalty
        for i in 0..in_edges.len() {
            for j in (i + 1)..in_edges.len() {
                let (a, b) = (in_edges[i].min(in_edges[j]), in_edges[i].max(in_edges[j]));
                quad_entries.push((a, b, 2.0 * penalty));
            }
        }

        // Pairwise outgoing: +2 * penalty
        for i in 0..out_edges.len() {
            for j in (i + 1)..out_edges.len() {
                let (a, b) = (
                    out_edges[i].min(out_edges[j]),
                    out_edges[i].max(out_edges[j]),
                );
                quad_entries.push((a, b, 2.0 * penalty));
            }
        }

        // Incoming × Outgoing: -2 * penalty (cross terms)
        for &u in &in_edges {
            for &v in &out_edges {
                let (a, b) = (u.min(v), u.max(v));
                quad_entries.push((a, b, -2.0 * penalty));
            }
        }
    }

    // 3. Bidirectional Reversal Penalty (Edge Lock)
    // Avoid trading forward and backward on the same pool in a single cycle
    for (pool_idx, pool) in pools.iter().enumerate() {
        let edge_0 = (pool_idx, pool.token0.index(), pool.token1.index());
        let edge_1 = (pool_idx, pool.token1.index(), pool.token0.index());
        if let (Some(&u), Some(&v)) = (var_idx.get(&edge_0), var_idx.get(&edge_1)) {
            let (a, b) = (u.min(v), u.max(v));
            // Add a massive penalty to block bidirectional trades on the same pool
            quad_entries.push((a, b, 2.0 * penalty));
        }
    }

    // Merge duplicate quadratic entries
    let mut merged: HashMap<(usize, usize), f64> = HashMap::new();
    for (a, b, w) in quad_entries {
        *merged.entry((a, b)).or_insert(0.0) += w;
    }

    // Build CSR matrix from merged quadratic terms
    // Store BOTH directions (i,j) and (j,i) for symmetric CSR
    let mut rows: Vec<Vec<(usize, f64)>> = vec![vec![]; m];
    for (&(a, b), &w) in &merged {
        rows[a].push((b, w));
        rows[b].push((a, w));
    }

    // Sort each row by column index
    for row in &mut rows {
        row.sort_by_key(|&(col, _)| col);
    }

    // Build CSR arrays
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0usize];

    for row in &rows {
        for &(col, val) in row {
            col_indices.push(col);
            values.push(val);
        }
        row_offsets.push(values.len());
    }

    let quadratic = CsrMatrix {
        values,
        col_indices,
        row_offsets,
    };

    let model = QuboModel {
        num_vars: m,
        linear,
        quadratic,
    };

    QuboCompileResult {
        model,
        edges,
        num_vars: m,
    }
}

// =============================================================================
// SOLUTION DECODER — Traces arbitrage cycle from solver output
// =============================================================================
struct ArbitrageResult {
    path: Vec<Token>,
    path_pools: Vec<usize>, // indexes of pools used in each trade hop
    total_yield: f64,
    profit_pct: f64,
    num_hops: usize,
    is_valid_cycle: bool,
}

fn decode_solution(
    state: &[i8],
    edges: &[(usize, usize, usize)],
    pools: &[PoolInfo],
    pm: &PriceMatrix,
) -> Option<ArbitrageResult> {
    // Find active edges
    let mut active: Vec<(usize, usize, usize)> = state
        .iter()
        .enumerate()
        .filter(|(_, &val)| val == 1)
        .map(|(idx, _)| edges[idx])
        .collect();

    if active.is_empty() {
        return None;
    }

    // Trace cycle starting from WETH (index 0)
    let mut current = 0usize;
    let mut path = vec![ALL_TOKENS[0]];
    let mut path_pools = Vec::new();
    let mut total_yield = 1.0_f64;
    let mut steps = 0;
    let mut is_complete_cycle = false;

    loop {
        let next_idx = active.iter().position(|&(_, from, _)| from == current);
        match next_idx {
            Some(idx) => {
                let (pool_idx, from, to) = active.remove(idx);
                let pool = &pools[pool_idx];
                let rate = pm
                    .get(&(pool.address, ALL_TOKENS[from], ALL_TOKENS[to]))
                    .copied()
                    .unwrap_or(0.0);

                total_yield *= rate;
                path.push(ALL_TOKENS[to]);
                path_pools.push(pool_idx);
                current = to;
                steps += 1;

                if to == 0 {
                    is_complete_cycle = true;
                    break;
                }
                if steps > edges.len() + 2 {
                    break;
                }
            }
            None => break,
        }
    }

    // CRITICAL: Only report profit for COMPLETE cycles
    if !is_complete_cycle {
        return Some(ArbitrageResult {
            path,
            path_pools,
            total_yield: 0.0,
            profit_pct: -100.0,
            num_hops: steps,
            is_valid_cycle: false,
        });
    }

    let profit_pct = (total_yield - 1.0) * 100.0;

    Some(ArbitrageResult {
        path,
        path_pools,
        total_yield,
        profit_pct,
        num_hops: steps,
        is_valid_cycle: true,
    })
}

// =============================================================================
// MAIN — Monolithic MEV Engine
// =============================================================================
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=====================================================");
    println!("🧊  ISING MEV ARBITRAGE ENGINE v3.1 (MONOLITH)");
    println!("=====================================================");
    println!("   4 tokens | 9 pools | Multi-Edge Per-Pool QUBO inline");
    println!("=====================================================");

    let wss_url = std::env::var("BASE_WSS_URL").unwrap_or_else(|_| {
        "wss://base-mainnet.g.alchemy.com/v2/F78FDrFMOfuwJXAEgFAJL".to_string()
    });

    println!("🌐 Connecting to Base Network via WebSockets...");

    let ws_provider = match Provider::<Ws>::connect(&wss_url).await {
        Ok(prov) => prov,
        Err(e) => {
            eprintln!("❌ Failed to connect: {:?}", e);
            return Err(e.into());
        }
    };

    let provider = Arc::new(ws_provider);
    println!("✅ Connected to Base blockchain RPC!");

    // Build pool registry and address lookup
    let pools = build_pool_registry();
    let pool_map: HashMap<Address, usize> = pools
        .iter()
        .enumerate()
        .map(|(i, p)| (p.address, i))
        .collect();

    println!(
        "📊 Tracking {} pools across {} tokens:",
        pools.len(),
        ALL_TOKENS.len()
    );
    for pool in &pools {
        println!(
            "   ├─ {} | {}/{} (fee: {})",
            format!("{:?}", pool.address),
            pool.token0.symbol(),
            pool.token1.symbol(),
            pool.fee_label
        );
    }

    // Shared price matrix (thread-safe)
    let prices: Arc<Mutex<PriceMatrix>> = Arc::new(Mutex::new(HashMap::new()));

    // ------------------------------------------------------------------
    // WARMUP: Fetch current slot0 rates for all pools directly
    // ------------------------------------------------------------------
    println!("🔥 [WARMUP] Fetching initial prices for all pools...");
    for pool in &pools {
        match fetch_initial_pool_price(provider.as_ref(), pool.address).await {
            Ok(sqrt_price_x96) => {
                let (raw_0_to_1, raw_1_to_0) =
                    compute_rates_from_sqrt_price(sqrt_price_x96, pool.token0, pool.token1);
                let fee_fraction = pool.fee_bps / 10000.0;
                let rate_0_to_1 = raw_0_to_1 * (1.0 - fee_fraction);
                let rate_1_to_0 = raw_1_to_0 * (1.0 - fee_fraction);

                {
                    let mut pm = prices.lock().unwrap();
                    pm.insert((pool.address, pool.token0, pool.token1), rate_0_to_1);
                    pm.insert((pool.address, pool.token1, pool.token0), rate_1_to_0);
                }
                println!(
                    "   ✅ Loaded initial rate for {}/{} ({}): {:.6}",
                    pool.token0.symbol(),
                    pool.token1.symbol(),
                    pool.fee_label,
                    rate_0_to_1
                );
            }
            Err(e) => {
                println!(
                    "   ⚠️ Failed to fetch initial price for pool {:?}: {:?}",
                    pool.address, e
                );
            }
        }
    }

    // keccak256("Swap(address,address,int256,int256,uint160,uint128,int24)")
    let v3_swap_topic: H256 = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67"
        .parse()
        .unwrap();

    // ------------------------------------------------------------------
    // TASK 1: Block Pulse + ISING SOLVER per block
    // ------------------------------------------------------------------
    let prices_clone = prices.clone();
    let provider_clone1 = provider.clone();
    let pools_clone1 = build_pool_registry();
    tokio::spawn(async move {
        println!("💓 [PULSE] Starting block listener + Ising solver...");

        // Solver config tuned for HFT microsecond speeds (<2ms target)
        let solver = UltimateSolver::new(
            2.0,      // temp_max
            0.1,      // temp_min
            20,       // sweeps_per_exchange
            5,        // total_exchanges (20*5=100 total sweeps)
            Some(42), // deterministic seed
        )
        .with_quantum_dims(1, 1, 1);

        match provider_clone1.subscribe_blocks().await {
            Ok(mut block_stream) => {
                println!("✅ [PULSE] Subscribed!");
                while let Some(block) = block_stream.next().await {
                    let block_num = block.number.unwrap_or_default();
                    let block_time_s = block.timestamp.as_u64() as u128;
                    let local_time_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    let latency = local_time_ms as i128 - (block_time_s * 1000) as i128;

                    // Snapshot the price matrix
                    let pm = prices_clone.lock().unwrap().clone();
                    let num_rates = pm.len();

                    if num_rates < 4 {
                        println!(
                            "\n💓 [BLOCK #{}] Latency: {} ms | Warming up... ({} rates)",
                            block_num, latency, num_rates
                        );
                        continue;
                    }

                    // Build a collapsed rate matrix just for display purposes
                    let mut display_matrix = [[0.0_f64; NUM_TOKENS]; NUM_TOKENS];
                    for i in 0..NUM_TOKENS {
                        display_matrix[i][i] = 1.0;
                    }
                    for (&(_pool_addr, from, to), &rate) in &pm {
                        let fi = from.index();
                        let ti = to.index();
                        if rate > display_matrix[fi][ti] {
                            display_matrix[fi][ti] = rate;
                        }
                    }

                    // Check if we have enough connectivity
                    let mut connected_pairs = 0;
                    for i in 0..NUM_TOKENS {
                        for j in 0..NUM_TOKENS {
                            if i != j && display_matrix[i][j] > 0.0 {
                                connected_pairs += 1;
                            }
                        }
                    }

                    // Print rate matrix
                    println!(
                        "\n💓 [BLOCK #{}] Latency: {} ms | {} rates, {} edges active",
                        block_num, latency, num_rates, connected_pairs
                    );
                    println!("┌─ RATE MATRIX (BEST DISPLAY RATES) ────────────────┐");
                    print!("│ {:>8}", "");
                    for t in &ALL_TOKENS {
                        print!(" {:>10}", t.symbol());
                    }
                    println!(" │");
                    for i in 0..NUM_TOKENS {
                        print!("│ {:>8}", ALL_TOKENS[i].symbol());
                        for j in 0..NUM_TOKENS {
                            if i == j {
                                print!(" {:>10}", "—");
                            } else if display_matrix[i][j] > 0.0 {
                                if display_matrix[i][j] > 100.0 {
                                    print!(" {:>10.2}", display_matrix[i][j]);
                                } else {
                                    print!(" {:>10.6}", display_matrix[i][j]);
                                }
                            } else {
                                print!(" {:>10}", "·");
                            }
                        }
                        println!(" │");
                    }
                    println!("└──────────────────────────────────────────────────┘");

                    // ══════════════════════════════════════════════════
                    // COMPILE QUBO FROM LIVE RATES (PER-POOL EDGES!)
                    // ══════════════════════════════════════════════════
                    let compile_start = Instant::now();
                    let qubo_result = compile_qubo(&pools_clone1, &pm, 500.0, 8.0, 0.05);
                    let compile_time = compile_start.elapsed();

                    if qubo_result.num_vars == 0 {
                        println!("⚠️  [QUBO] Compiled to 0 active edges. Skipping solve.");
                        continue;
                    }

                    // ══════════════════════════════════════════════════
                    // RUN ISING SOLVER (THE CORE ENGINE)
                    // ══════════════════════════════════════════════════
                    // Clamp the WETH→USDC edge that has the BEST live rate to force starting from WETH
                    let mut best_clamp_idx = None;
                    let mut best_rate = 0.0;
                    for (idx, &(pool_idx, from, to)) in qubo_result.edges.iter().enumerate() {
                        if from == 0 && to == 1 {
                            let pool = &pools_clone1[pool_idx];
                            if let Some(&rate) =
                                pm.get(&(pool.address, ALL_TOKENS[from], ALL_TOKENS[to]))
                            {
                                if rate > best_rate {
                                    best_rate = rate;
                                    best_clamp_idx = Some(idx);
                                }
                            }
                        }
                    }
                    let clamped: Vec<(usize, i8)> = match best_clamp_idx {
                        Some(idx) => vec![(idx, 1)],
                        None => vec![],
                    };

                    let solve_start = Instant::now();
                    let state = solver.solve(&qubo_result.model, &clamped);
                    let solve_time = solve_start.elapsed();

                    let energy = qubo_result.model.calculate_total_energy(&state);

                    println!("🧊 [ISING] Compiled: {} vars | QUBO: {:.1}ms | Solve: {:.1}ms | Energy: {:.4}",
                        qubo_result.num_vars,
                        compile_time.as_secs_f64() * 1000.0,
                        solve_time.as_secs_f64() * 1000.0,
                        energy,
                    );

                    // ══════════════════════════════════════════════════
                    // DECODE ARBITRAGE PATH
                    // ══════════════════════════════════════════════════
                    if let Some(arb) =
                        decode_solution(&state, &qubo_result.edges, &pools_clone1, &pm)
                    {
                        let mut path_str = String::new();
                        path_str.push_str(arb.path[0].symbol());
                        for i in 0..arb.path_pools.len() {
                            let pool_idx = arb.path_pools[i];
                            let pool = &pools_clone1[pool_idx];
                            path_str.push_str(&format!(" ─[{}]─> ", pool.fee_label));
                            path_str.push_str(arb.path[i + 1].symbol());
                        }

                        if !arb.is_valid_cycle {
                            println!(
                                "⚠️  [BROKEN] {} | Open path (no cycle) | Hops: {}",
                                path_str, arb.num_hops
                            );
                        } else if arb.profit_pct > 0.0 {
                            println!(
                                "🚨 [ARB FOUND!] {} | Yield: {:.6}x | Profit: +{:.4}% | Hops: {}",
                                path_str, arb.total_yield, arb.profit_pct, arb.num_hops,
                            );
                        } else {
                            println!(
                                "📉 [NO ARB] {} | Yield: {:.6}x | {:.4}% | Hops: {}",
                                path_str, arb.total_yield, arb.profit_pct, arb.num_hops,
                            );
                        }
                    } else {
                        println!("📉 [NO ARB] Solver found empty state.");
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ [PULSE] Failed: {:?}", e);
            }
        }
    });

    // ------------------------------------------------------------------
    // TASK 2: UNIFIED SWAP RADAR (unfiltered → in-code routing)
    // ------------------------------------------------------------------
    let prices_clone2 = prices.clone();
    let provider_clone2 = provider.clone();
    let pools_clone2 = build_pool_registry();
    tokio::spawn(async move {
        let filter = Filter::new();

        println!("🎧 [RADAR] Unified V3 Swap Radar active!");
        match provider_clone2.subscribe_logs(&filter).await {
            Ok(mut log_stream) => {
                println!("✅ [RADAR] Subscribed! Routing swaps in-code...");
                let mut swap_count: u64 = 0;

                while let Some(log) = log_stream.next().await {
                    let topic0 = match log.topics.first() {
                        Some(t) => *t,
                        None => continue,
                    };

                    if topic0 != v3_swap_topic {
                        continue;
                    }

                    let pool_addr = log.address;
                    let pool_idx = match pool_map.get(&pool_addr) {
                        Some(idx) => *idx,
                        None => continue,
                    };

                    let data = log.data.as_ref();
                    if data.len() < 160 {
                        continue;
                    }

                    swap_count += 1;
                    let pool = &pools_clone2[pool_idx];
                    let sqrt_price_x96 = U256::from_big_endian(&data[64..96]);

                    let (raw_0_to_1, raw_1_to_0) =
                        compute_rates_from_sqrt_price(sqrt_price_x96, pool.token0, pool.token1);

                    // Apply pool fee BEFORE storing rates
                    let fee_fraction = pool.fee_bps / 10000.0;
                    let rate_0_to_1 = raw_0_to_1 * (1.0 - fee_fraction);
                    let rate_1_to_0 = raw_1_to_0 * (1.0 - fee_fraction);

                    // Update price matrix atomically with FEE-ADJUSTED rates
                    {
                        let mut pm = prices_clone2.lock().unwrap();
                        pm.insert((pool.address, pool.token0, pool.token1), rate_0_to_1);
                        pm.insert((pool.address, pool.token1, pool.token0), rate_1_to_0);
                    }

                    // Compact swap log
                    let block_num = log.block_number.unwrap_or_default();
                    println!(
                        "⚡ [#{} {}] {}/{} ({}) rate={:.6}",
                        swap_count,
                        block_num,
                        pool.token0.symbol(),
                        pool.token1.symbol(),
                        pool.fee_label,
                        rate_0_to_1,
                    );
                }
            }
            Err(e) => {
                eprintln!("❌ [RADAR] Failed: {:?}", e);
            }
        }
    });

    // Keep the main thread alive
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
