use axum::{
    routing::{get, post},
    Router, Json,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::UltimateSolver;
use std::time::Instant;
use tokio::net::TcpListener;

#[derive(Deserialize)]
struct QuboRequest {
    num_vars: usize,
    linear: Vec<f64>,
    quadratic: Vec<(usize, usize, f64)>,
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

async fn health_check() -> &'static str {
    "⚡ ZeroClaw QUBO Engine (Enterprise API) is running."
}

async fn solve_qubo(Json(payload): Json<QuboRequest>) -> Json<QuboResponse> {
    let start = Instant::now();
    
    let mut row_edges: Vec<Vec<(usize, f64)>> = vec![vec![]; payload.num_vars];
    for (u, v, w) in payload.quadratic {
        row_edges[u].push((v, w));
        row_edges[v].push((u, w));
    }
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for edges in row_edges.iter_mut() {
        edges.sort_by_key(|&(v, _)| v);
        for &(v, w) in edges.iter() {
            col_indices.push(v);
            values.push(w);
        }
        row_offsets.push(col_indices.len());
    }

    let model = QuboModel {
        num_vars: payload.num_vars,
        linear: payload.linear,
        quadratic: CsrMatrix { values, col_indices, row_offsets },
    };

    let solver = UltimateSolver::new(
        1000.0, 0.01, 
        payload.sweeps.unwrap_or(100), 
        payload.replicas.unwrap_or(100), 
        None
    );

    let state = solver.solve(&model, &[]);
    
    let mut energy = 0.0;
    for i in 0..model.num_vars {
        if state[i] == 1 {
            energy += model.linear[i];
            for (j, weight) in model.quadratic.get_row(i) {
                if state[j] == 1 { energy += weight * 0.5; }
            }
        }
    }

    Json(QuboResponse {
        status: "success".to_string(),
        energy,
        state,
        compute_time_ms: start.elapsed().as_millis(),
    })
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/solve", post(solve_qubo));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("🚀 QUBO API Server starting on http://{}", addr);
    
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
