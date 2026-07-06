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

/// Request schema for POST /api/v1/solve.
///
/// - `sweeps` / `exchanges`: annealing budget (defaults 100/100, clamped to
///   server maxima).
/// - `seed`: optional. Identical requests with the same seed return identical
///   results (the engine is deterministic given a seed). Omitted → random.
/// - `replicas`: accepted for backward compatibility but IGNORED — the engine
///   always runs 64 SIMD replica lanes per temperature. Historical note:
///   this field was previously (incorrectly) bound to the exchange budget;
///   use `exchanges` for that.
#[derive(Deserialize)]
struct QuboRequest {
    num_vars: usize,
    linear: Vec<f64>,
    quadratic: Vec<(usize, usize, f64)>,
    clamped: Option<Vec<(usize, i8)>>,
    #[serde(default, rename = "replicas")]
    _replicas: Option<usize>,
    sweeps: Option<usize>,
    exchanges: Option<usize>,
    seed: Option<u64>,
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

/// Request validation. Runs BEFORE a compute permit is taken, so invalid
/// requests never consume solver capacity.
fn validate(req: &QuboRequest) -> Result<(), String> {
    if req.num_vars == 0 {
        return Err("num_vars must be > 0".to_string());
    }
    if req.num_vars > MAX_NUM_VARS {
        return Err(format!("num_vars exceeds maximum {}", MAX_NUM_VARS));
    }
    if req.linear.len() != req.num_vars {
        return Err("linear.len() must equal num_vars".to_string());
    }
    for &(u, v, _) in &req.quadratic {
        if u >= req.num_vars || v >= req.num_vars {
            return Err(format!(
                "edge index out of range: ({}, {}) for num_vars={}",
                u, v, req.num_vars
            ));
        }
    }
    if let Some(ref clamped) = req.clamped {
        for &(idx, val) in clamped {
            if idx >= req.num_vars {
                return Err(format!("clamped index {} out of range", idx));
            }
            if val != 0 && val != 1 {
                return Err(format!("clamped value must be 0 or 1, got {}", val));
            }
        }
    }
    Ok(())
}

/// Maps request parameters onto a solver configuration.
/// Kept separate so the parameter contract is unit-testable.
fn solver_from(req: &QuboRequest) -> UltimateSolver {
    let sweeps = req.sweeps.unwrap_or(100).clamp(1, MAX_SWEEPS);
    let exchanges = req.exchanges.unwrap_or(100).clamp(1, MAX_EXCHANGES);
    UltimateSolver::new(1000.0, 0.01, sweeps, exchanges, req.seed)
}

/// Synchronous compute core: CSR construction with duplicate-edge merging,
/// solve, energy evaluation. Assumes `validate` has passed.
fn compute(req: QuboRequest) -> (Vec<i8>, f64) {
    let solver = solver_from(&req);
    let num_vars = req.num_vars;
    let clamped = req.clamped.unwrap_or_default();

    let mut row_edges: Vec<Vec<(usize, f64)>> = vec![vec![]; num_vars];
    for (u, v, w) in req.quadratic {
        row_edges[u].push((v, w));
        row_edges[v].push((u, w));
    }

    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];

    for edges in row_edges.iter_mut() {
        edges.sort_by_key(|&(v, _)| v);
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
        energy_offset: 0.0,
        num_vars,
        linear: req.linear,
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    };

    // Data-driven temperature range (White 1984-style; see
    // solver::schedule): removes the O(1)-coefficient assumption of the
    // previous hardcoded 1000/0.01 schedule.
    let (t_max, t_min) = ising_engine::solver::schedule::suggested_temp_range(&model);
    let mut solver = solver;
    solver.temp_max = t_max;
    solver.temp_min = t_min;

    let solution = solver.solve(&model, &clamped);
    let energy = model.calculate_total_energy(&solution);
    (solution, energy)
}

/// Full request pipeline (validation + compute) — the unit-testable core of
/// the /solve handler. The production handler calls the two halves
/// separately so validation runs before a compute permit is taken.
#[cfg(test)]
fn handle_solve(req: QuboRequest) -> Result<(Vec<i8>, f64), String> {
    validate(&req)?;
    Ok(compute(req))
}

async fn solve_qubo(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<QuboRequest>,
) -> Json<QuboResponse> {
    if let Err(msg) = validate(&payload) {
        return error_response(&msg);
    }

    let permit = match state.compute_semaphore.try_acquire() {
        Ok(p) => p,
        Err(_) => {
            return error_response("server overloaded, try again later");
        }
    };

    let start = Instant::now();
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        state_clone.compute_pool.install(move || compute(payload))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Dense mixed-sign 32-var instance (fixed construction seed).
    /// Mixed signs keep it presolve-immune, so the solver genuinely anneals
    /// rather than returning a presolve-determined state.
    fn dense_request() -> QuboRequest {
        use rand::{Rng, SeedableRng};
        let n = 32;
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(0xC0FFEE);
        let mut quadratic = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                quadratic.push((i, j, rng.gen_range(-1.0..1.0)));
            }
        }
        QuboRequest {
            num_vars: n,
            linear: (0..n).map(|_| rng.gen_range(-1.0..1.0)).collect(),
            quadratic,
            clamped: None,
            _replicas: None,
            // Minimal budget: the returned state is dominated by the random
            // initialization, so reproducibility requires a seed mechanism —
            // convergence luck cannot mask its absence.
            sweeps: Some(1),
            exchanges: Some(1),
            seed: Some(42),
        }
    }

    /// The exchange budget must come from its own parameter (or default),
    /// never from `replicas`: replica lanes are a fixed property of the
    /// engine (64), not an annealing-length knob.
    #[test]
    fn replicas_parameter_does_not_control_exchange_budget() {
        let req = QuboRequest {
            num_vars: 4,
            linear: vec![0.0; 4],
            quadratic: vec![],
            clamped: None,
            _replicas: Some(7),
            sweeps: None,
            exchanges: None,
            seed: None,
        };
        let solver = solver_from(&req);
        assert_eq!(
            solver.total_exchanges, 100,
            "'replicas' must not set total_exchanges; the budget must use its own parameter/default"
        );

        let req = QuboRequest {
            exchanges: Some(37),
            ..dense_request()
        };
        assert_eq!(
            solver_from(&req).total_exchanges,
            37,
            "'exchanges' must set the exchange budget"
        );
    }

    /// Old-client payloads carrying `replicas` must remain accepted on the
    /// wire (deserialized, then ignored — never bound to the budget).
    #[test]
    fn old_client_payload_with_replicas_still_deserializes() {
        let json =
            r#"{"num_vars":2,"linear":[0.0,0.0],"quadratic":[[0,1,0.5]],"replicas":16,"sweeps":5}"#;
        let req: QuboRequest = serde_json::from_str(json).expect("old schema must remain accepted");
        assert_eq!(req._replicas, Some(16));
        assert_eq!(
            solver_from(&req).total_exchanges,
            100,
            "legacy 'replicas' must not leak into the exchange budget"
        );
    }

    /// The API must offer a reproducibility mechanism: identical requests
    /// (using whatever mechanism the schema provides) must be able to return
    /// identical results.
    #[test]
    fn identical_requests_are_reproducible() {
        let a = handle_solve(dense_request()).expect("valid request");
        let b = handle_solve(dense_request()).expect("valid request");
        assert_eq!(
            a, b,
            "identical requests returned different results — no reproducibility mechanism"
        );
    }
}
