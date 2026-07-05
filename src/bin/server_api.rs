use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::UltimateSolver;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

const MAX_NUM_VARS: usize = 50_000;
const MAX_SWEEPS: usize = 10_000;
const MAX_EXCHANGES: usize = 10_000;

#[derive(Deserialize)]
struct QuboRequest {
    num_vars: usize,
    linear: Vec<f64>,
    quadratic: Vec<(usize, usize, f64)>,
    clamped: Option<Vec<(usize, i8)>>,
    replicas: Option<usize>,
    sweeps: Option<usize>,
}

#[derive(Serialize)]
struct QuboResponse {
    status: String,
    energy: f64,
    state: Vec<i8>,
    compute_time_ms: u128,
}

struct AppState {
    compute_pool: rayon::ThreadPool,
    compute_semaphore: Semaphore,
}

fn error_response(msg: &str) -> Json<QuboResponse> {
    Json(QuboResponse {
        status: format!("error: {}", msg),
        energy: 0.0,
        state: vec![],
        compute_time_ms: 0,
    })
}

async fn health_check() -> &'static str {
    "⚡ QUBO Engine API is running."
}

async fn solve_qubo(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<QuboRequest>,
) -> Json<QuboResponse> {
    // --- Input Validation (Fix B) ---
    if payload.num_vars == 0 {
        return error_response("num_vars must be > 0");
    }
    if payload.num_vars > MAX_NUM_VARS {
        return error_response(&format!("num_vars exceeds maximum {}", MAX_NUM_VARS));
    }
    if payload.linear.len() != payload.num_vars {
        return error_response("linear.len() must equal num_vars");
    }
    for &(u, v, _) in &payload.quadratic {
        if u >= payload.num_vars || v >= payload.num_vars {
            return error_response(&format!(
                "edge index out of range: ({}, {}) for num_vars={}",
                u, v, payload.num_vars
            ));
        }
    }
    if let Some(ref clamped) = payload.clamped {
        for &(idx, val) in clamped {
            if idx >= payload.num_vars {
                return error_response(&format!("clamped index {} out of range", idx));
            }
            if val != 0 && val != 1 {
                return error_response(&format!("clamped value must be 0 or 1, got {}", val));
            }
        }
    }
    let sweeps = payload.sweeps.unwrap_or(100).clamp(1, MAX_SWEEPS);
    let exchanges = payload.replicas.unwrap_or(100).clamp(1, MAX_EXCHANGES);

    // --- Backpressure (Fix A) ---
    let permit = match state.compute_semaphore.try_acquire() {
        Ok(p) => p,
        Err(_) => {
            return error_response("server overloaded, try again later");
        }
    };

    let start = Instant::now();

    // --- CSR Construction with duplicate-edge merging (Fix B) ---
    let num_vars = payload.num_vars;
    let linear = payload.linear;
    let quadratic = payload.quadratic;
    let clamped = payload.clamped.unwrap_or_default();

    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        state_clone.compute_pool.install(move || {
            let mut row_edges: Vec<Vec<(usize, f64)>> = vec![vec![]; num_vars];
            for (u, v, w) in quadratic {
                row_edges[u].push((v, w));
                row_edges[v].push((u, w));
            }

            let mut values = Vec::new();
            let mut col_indices = Vec::new();
            let mut row_offsets = vec![0];

            for edges in row_edges.iter_mut() {
                edges.sort_by_key(|&(v, _)| v);
                // Merge duplicate edges (Fix B correctness)
                let mut merged: Vec<(usize, f64)> = Vec::new();
                for &(v, w) in edges.iter() {
                    if let Some(last) = merged.last_mut() {
                        if last.0 == v {
                            last.1 += w;
                            continue;
                        }
                    }
                    merged.push((v, w));
                }
                for (v, w) in merged {
                    if w.abs() > 1e-9 {
                        col_indices.push(v);
                        values.push(w);
                    }
                }
                row_offsets.push(col_indices.len());
            }

            let model = QuboModel {
                num_vars,
                linear,
                quadratic: CsrMatrix {
                    values,
                    col_indices,
                    row_offsets,
                },
            };

            let solver = UltimateSolver::new(1000.0, 0.01, sweeps, exchanges, None);
            let clamped_slice = &clamped;
            let solution = solver.solve(&model, clamped_slice);

            let mut energy = 0.0;
            for i in 0..model.num_vars {
                if solution[i] == 1 {
                    energy += model.linear[i];
                    for (j, weight) in model.quadratic.get_row(i) {
                        if solution[j] == 1 {
                            energy += weight * 0.5;
                        }
                    }
                }
            }

            (solution, energy)
        })
    })
    .await;

    drop(permit);

    match result {
        Ok((solution, energy)) => Json(QuboResponse {
            status: "success".to_string(),
            energy,
            state: solution,
            compute_time_ms: start.elapsed().as_millis(),
        }),
        Err(e) => error_response(&format!("solver panicked: {}", e)),
    }
}

#[tokio::main]
async fn main() {
    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let compute_threads = num_cpus.saturating_sub(2).max(1);

    let shared_state = Arc::new(AppState {
        compute_pool: rayon::ThreadPoolBuilder::new()
            .num_threads(compute_threads)
            .start_handler(|index| {
                if let Some(core_ids) = core_affinity::get_core_ids() {
                    if index < core_ids.len() {
                        core_affinity::set_for_current(core_ids[index]);
                    }
                }
            })
            .build()
            .expect("failed to create compute thread pool"),
        compute_semaphore: Semaphore::new(compute_threads),
    });

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/solve", post(solve_qubo))
        .with_state(shared_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!(
        "🚀 QUBO API Server starting on http://{} ({} compute threads)",
        addr, compute_threads
    );

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
