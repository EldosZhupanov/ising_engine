#![allow(warnings)]
use axum::{
    response::{
        sse::{Event, Sse},
        Html,
    },
    routing::get,
    Router,
};
use futures::stream::Stream;
use rand::Rng;
use rayon::prelude::*;
use serde_json::json;
use std::{convert::Infallible, f64::consts::E, net::SocketAddr, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

// ==========================================
// 1. ВЫЧИСЛИТЕЛЬНОЕ ЯДРО И КОМПИЛЯТОР
// ==========================================
#[derive(Debug)]
pub struct CsrMatrix {
    pub values: Vec<f64>,
    pub col_indices: Vec<usize>,
    pub row_offsets: Vec<usize>,
}
impl CsrMatrix {
    pub fn get_row(&self, row: usize) -> impl Iterator<Item = (usize, f64)> + '_ {
        let start = self.row_offsets[row];
        let end = self.row_offsets[row + 1];
        self.col_indices[start..end]
            .iter()
            .copied()
            .zip(self.values[start..end].iter().copied())
    }
}
pub struct QuboModel {
    pub num_vars: usize,
    pub linear: Vec<f64>,
    pub quadratic: CsrMatrix,
}
impl QuboModel {
    pub fn calculate_total_energy(&self, state: &[i8]) -> f64 {
        let mut energy = 0.0;
        for i in 0..self.num_vars {
            if state[i] == 1 {
                energy += self.linear[i];
                for (j, weight) in self.quadratic.get_row(i) {
                    if state[j] == 1 {
                        energy += weight * 0.5;
                    }
                }
            }
        }
        energy
    }
}

pub struct Multiplier2x2 {
    pub a: [usize; 2],
    pub b: [usize; 2],
    pub p: [usize; 4],
}

pub struct LogicBuilder {
    linear: Vec<f64>,
    quadratic_edges: Vec<(usize, usize, f64)>,
}
impl LogicBuilder {
    pub fn new() -> Self {
        Self {
            linear: vec![],
            quadratic_edges: vec![],
        }
    }
    pub fn add_var(&mut self) -> usize {
        let id = self.linear.len();
        self.linear.push(0.0);
        id
    }
    fn add_edge(&mut self, u: usize, v: usize, weight: f64) {
        self.quadratic_edges.push((u, v, weight));
        self.quadratic_edges.push((v, u, weight));
    }
    pub fn add_and_gate(&mut self, a: usize, b: usize, z: usize) {
        self.linear[z] += 3.0;
        self.add_edge(a, b, 1.0);
        self.add_edge(a, z, -2.0);
        self.add_edge(b, z, -2.0);
    }
    pub fn add_xor_gate(&mut self, x: usize, y: usize, z: usize, w: usize) {
        self.linear[x] += 1.0;
        self.linear[y] += 1.0;
        self.linear[z] += 1.0;
        self.linear[w] += 4.0;
        self.add_edge(x, y, 2.0);
        self.add_edge(x, z, -2.0);
        self.add_edge(y, z, -2.0);
        self.add_edge(x, w, -4.0);
        self.add_edge(y, w, -4.0);
        self.add_edge(z, w, 4.0);
    }
    pub fn add_half_adder(&mut self, a: usize, b: usize, sum: usize, carry: usize) {
        let ancilla = self.add_var();
        self.add_xor_gate(a, b, sum, ancilla);
        self.add_and_gate(a, b, carry);
    }
    pub fn add_multiplier_2x2(&mut self, m: Multiplier2x2) {
        self.add_and_gate(m.a[0], m.b[0], m.p[0]);
        let m10 = self.add_var();
        self.add_and_gate(m.a[1], m.b[0], m10);
        let m01 = self.add_var();
        self.add_and_gate(m.a[0], m.b[1], m01);
        let c1 = self.add_var();
        self.add_half_adder(m10, m01, m.p[1], c1);
        let m11 = self.add_var();
        self.add_and_gate(m.a[1], m.b[1], m11);
        self.add_half_adder(m11, c1, m.p[2], m.p[3]);
    }
    pub fn build(self) -> QuboModel {
        let n = self.linear.len();
        let mut row_edges: Vec<Vec<(usize, f64)>> = vec![vec![]; n];
        for (u, v, w) in self.quadratic_edges {
            row_edges[u].push((v, w));
        }
        let mut values = vec![];
        let mut col_indices = vec![];
        let mut row_offsets = vec![0];
        for mut edges in row_edges {
            edges.sort_by_key(|&(v, _)| v);
            let mut merged = vec![];
            for (v, w) in edges {
                if let Some(&mut (last_v, ref mut last_w)) = merged.last_mut() {
                    if last_v == v {
                        *last_w += w;
                    } else {
                        merged.push((v, w));
                    }
                } else {
                    merged.push((v, w));
                }
            }
            for (v, w) in merged {
                if w != 0.0 {
                    col_indices.push(v);
                    values.push(w);
                }
            }
            row_offsets.push(col_indices.len());
        }
        QuboModel {
            num_vars: n,
            linear: self.linear,
            quadratic: CsrMatrix {
                values,
                col_indices,
                row_offsets,
            },
        }
    }
}

// ==========================================
// 2. ULTIMATE SOLVER (С трансляцией в браузер)
// ==========================================
#[derive(Clone)]
struct Replica {
    state: Vec<i8>,
    temp: f64,
    energy: f64,
}

pub struct StreamingSolver {
    pub num_replicas: usize,
    pub temp_max: f64,
    pub temp_min: f64,
    pub sweeps_per_exchange: usize,
    pub total_exchanges: usize,
    pub adaptation_interval: usize,
}

impl StreamingSolver {
    fn try_cluster_flip(
        model: &QuboModel,
        replica: &mut Replica,
        clamped: &[(usize, i8)],
        rng: &mut rand::rngs::ThreadRng,
    ) {
        let seed = rng.gen_range(0..model.num_vars);
        if clamped.iter().any(|&(i, _)| i == seed) {
            return;
        }
        let mut cluster = vec![seed];
        let mut in_cluster = vec![false; model.num_vars];
        in_cluster[seed] = true;
        let mut queue = vec![seed];

        while let Some(current) = queue.pop() {
            for (neighbor, weight) in model.quadratic.get_row(current) {
                if !in_cluster[neighbor] && weight.abs() > 1.0 {
                    if clamped.iter().any(|&(i, _)| i == neighbor) {
                        continue;
                    }
                    let prob = 1.0 - (-2.0 * weight.abs() / replica.temp).exp();
                    if rng.gen_range(0.0..1.0) < prob {
                        in_cluster[neighbor] = true;
                        cluster.push(neighbor);
                        queue.push(neighbor);
                    }
                }
            }
        }
        if cluster.len() < 2 {
            return;
        }
        let e_old = model.calculate_total_energy(&replica.state);
        for &idx in &cluster {
            replica.state[idx] = 1 - replica.state[idx];
        }
        let e_new = model.calculate_total_energy(&replica.state);
        let delta_e = e_new - e_old;
        if delta_e > 0.0
            && (replica.temp <= 1e-8 || rng.gen_range(0.0..1.0) >= E.powf(-delta_e / replica.temp))
        {
            for &idx in &cluster {
                replica.state[idx] = 1 - replica.state[idx];
            }
        }
    }

    pub fn solve(
        &self,
        model: &QuboModel,
        clamped: &[(usize, i8)],
        tx: mpsc::Sender<String>,
    ) -> Vec<i8> {
        let mut rng = rand::thread_rng();
        let mut replicas: Vec<Replica> = (0..self.num_replicas)
            .map(|i| {
                let mut state: Vec<i8> =
                    (0..model.num_vars).map(|_| rng.gen_range(0..=1)).collect();
                for &(idx, val) in clamped {
                    state[idx] = val;
                }
                let fraction = i as f64 / (self.num_replicas - 1).max(1) as f64;
                let temp = self.temp_max * (self.temp_min / self.temp_max).powf(fraction);
                let energy = model.calculate_total_energy(&state);
                Replica {
                    state,
                    temp,
                    energy,
                }
            })
            .collect();

        let mut swap_accepts = vec![0; self.num_replicas - 1];

        for exchange_step in 0..self.total_exchanges {
            replicas.par_iter_mut().for_each(|replica| {
                let mut local_rng = rand::thread_rng();
                for step in 0..self.sweeps_per_exchange {
                    if step % 10 == 0 {
                        Self::try_cluster_flip(model, replica, clamped, &mut local_rng);
                    } else {
                        let var_idx = local_rng.gen_range(0..model.num_vars);
                        if clamped.iter().any(|&(i, _)| i == var_idx) {
                            continue;
                        }
                        let flip_multiplier = 1.0 - 2.0 * (replica.state[var_idx] as f64);
                        let mut sum_j = 0.0;
                        for (col, weight) in model.quadratic.get_row(var_idx) {
                            sum_j += weight * (replica.state[col] as f64);
                        }
                        let delta_e = flip_multiplier * (model.linear[var_idx] + sum_j);

                        if delta_e < 0.0
                            || (replica.temp > 1e-8
                                && local_rng.gen_range(0.0..1.0) < E.powf(-delta_e / replica.temp))
                        {
                            replica.state[var_idx] = 1 - replica.state[var_idx];
                        }
                    }
                }
                replica.energy = model.calculate_total_energy(&replica.state);
            });

            for i in 0..(self.num_replicas - 1) {
                let delta_beta = (1.0 / replicas[i].temp) - (1.0 / replicas[i + 1].temp);
                let delta_energy = replicas[i].energy - replicas[i + 1].energy;
                let swap_prob = (delta_beta * delta_energy).exp();
                if swap_prob >= 1.0 || rng.gen_range(0.0..1.0) < swap_prob {
                    swap_accepts[i] += 1;
                    let temp_state = replicas[i].state.clone();
                    replicas[i].state = replicas[i + 1].state.clone();
                    replicas[i + 1].state = temp_state;
                    for &(idx, val) in clamped {
                        replicas[i].state[idx] = val;
                        replicas[i + 1].state[idx] = val;
                    }
                    let temp_energy = replicas[i].energy;
                    replicas[i].energy = replicas[i + 1].energy;
                    replicas[i + 1].energy = temp_energy;
                }
            }

            if (exchange_step + 1) % self.adaptation_interval == 0 {
                for i in 1..(self.num_replicas - 1) {
                    let rate = swap_accepts[i] as f64 / self.adaptation_interval as f64;
                    let mut new_temp = replicas[i].temp * (1.0 - (0.23 - rate) * 0.05);
                    new_temp =
                        new_temp.clamp(replicas[i + 1].temp + 0.001, replicas[i - 1].temp - 0.001);
                    replicas[i].temp = new_temp;
                    swap_accepts[i] = 0;
                }
            }

            // МАГИЯ ДАШБОРДА: Отправляем текущую статистику в браузер
            let min_energy = replicas
                .iter()
                .map(|r| r.energy)
                .fold(f64::INFINITY, f64::min);
            let payload = json!({ "step": exchange_step, "energy": min_energy }).to_string();
            let _ = tx.blocking_send(payload);

            // Искусственная задержка для WOW-эффекта (чтобы график рисовался плавно)
            std::thread::sleep(Duration::from_millis(30));

            if min_energy == 0.0 {
                break;
            } // Досрочный выход, если дно найдено!
        }
        replicas
            .into_iter()
            .min_by(|a, b| a.energy.partial_cmp(&b.energy).unwrap())
            .unwrap()
            .state
    }
}

// ==========================================
// 3. WEB SERVER (Axum + SSE)
// ==========================================
#[tokio::main]
async fn main() {
    println!("🦀 Стартуем ZeroClaw Quantum-Inspired Web Dashboard...");
    let app = Router::new()
        .route("/", get(index_html))
        .route("/solve", get(sse_handler));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("🚀 ВНИМАНИЕ! Открой в браузере: http://127.0.0.1:3000");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index_html() -> Html<&'static str> {
    Html(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>ZeroClaw Ising Solver</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        body { background-color: #0d1117; color: #c9d1d9; font-family: monospace; text-align: center; padding: 20px; }
        h1 { color: #58a6ff; }
        #canvas-container { width: 80%; margin: 20px auto; background: #161b22; border-radius: 8px; padding: 15px; border: 1px solid #30363d; }
        button { background-color: #238636; color: white; border: none; padding: 15px 30px; font-size: 18px; cursor: pointer; border-radius: 6px; font-weight: bold; }
        button:hover { background-color: #2ea043; }
        #result { margin-top: 20px; font-size: 24px; color: #3fb950; font-weight: bold; }
    </style>
</head>
<body>
    <h1>🦀 ZeroClaw Quantum-Inspired Solver</h1>
    <p>Target: Factorize 6 (Find A and B where A * B = 6)</p>
    <button onclick="startSolving()">Start Annealing Process</button>
    <div id="canvas-container"><canvas id="energyChart"></canvas></div>
    <div id="result">Waiting to start...</div>

    <script>
        let chart = new Chart(document.getElementById('energyChart'), {
            type: 'line',
            data: { labels: [], datasets: [{ label: 'Global Minimum Energy', borderColor: '#ff7b72', data: [], tension: 0.1, borderWidth: 2 }] },
            options: { animation: false, scales: { y: { beginAtZero: true, title: { display: true, text: 'Energy (Lower is better)'} } } }
        });

        function startSolving() {
            chart.data.labels = []; chart.data.datasets[0].data = []; chart.update();
            document.getElementById('result').innerText = "Simulating 64 Parallel Universes...";
            
            const eventSource = new EventSource('/solve');
            eventSource.onmessage = function(event) {
                if (event.data.startsWith("DONE")) {
                    document.getElementById('result').innerText = "✅ " + event.data;
                    eventSource.close(); return;
                }
                const data = JSON.parse(event.data);
                chart.data.labels.push(data.step);
                chart.data.datasets[0].data.push(data.energy);
                chart.update();
            };
        }
    </script>
</body>
</html>
    "#,
    )
}

async fn sse_handler() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, mut rx) = mpsc::channel(100);

    tokio::task::spawn_blocking(move || {
        let mut compiler = LogicBuilder::new();
        let a0 = compiler.add_var();
        let a1 = compiler.add_var();
        let b0 = compiler.add_var();
        let b1 = compiler.add_var();
        let p0 = compiler.add_var();
        let p1 = compiler.add_var();
        let p2 = compiler.add_var();
        let p3 = compiler.add_var();
        compiler.add_multiplier_2x2(Multiplier2x2 {
            a: [a0, a1],
            b: [b0, b1],
            p: [p0, p1, p2, p3],
        });
        let model = compiler.build();

        let solver = StreamingSolver {
            num_replicas: 64,
            temp_max: 300.0,
            temp_min: 0.01,
            sweeps_per_exchange: 500,
            total_exchanges: 300,
            adaptation_interval: 20,
        };

        let clamped = vec![(p3, 0), (p2, 1), (p1, 1), (p0, 0)]; // Target = 6
        let result = solver.solve(&model, &clamped, tx.clone());

        let val_a = (result[a1] << 1) | result[a0];
        let val_b = (result[b1] << 1) | result[b0];
        let _ = tx.blocking_send(format!("DONE: Factored! A = {}, B = {}", val_a, val_b));
    });

    let stream = ReceiverStream::new(rx).map(|data| Ok(Event::default().data(data)));
    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new())
}
