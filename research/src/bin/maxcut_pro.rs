#![allow(warnings)]
use rand::prelude::*;
use std::time::Instant;

#[derive(Clone)]
struct Edge {
    to: usize,
    weight: f64,
}

struct Graph {
    adj: Vec<Vec<Edge>>,
    n: usize,
}

impl Graph {
    fn new(n: usize) -> Self {
        Self {
            adj: vec![Vec::new(); n],
            n,
        }
    }

    fn add_edge(&mut self, u: usize, v: usize, w: f64) {
        self.adj[u].push(Edge { to: v, weight: w });
        self.adj[v].push(Edge { to: u, weight: w });
    }

    fn random(n: usize, density: f64) -> Self {
        let mut rng = rand::thread_rng();
        let mut g = Graph::new(n);

        for i in 0..n {
            for j in (i + 1)..n {
                if rng.r#gen::<f64>() < density {
                    let w = rng.gen_range(1.0f64..10.0f64).round();
                    g.add_edge(i, j, w);
                }
            }
        }
        g
    }
}

struct IsingSolver {
    graph: Graph,
    spins: Vec<i8>,
    delta: Vec<f64>,
    energy: f64,
}

impl IsingSolver {
    fn new(graph: Graph) -> Self {
        let mut rng = rand::thread_rng();
        let n = graph.n;

        let spins: Vec<i8> = (0..n)
            .map(|_| if rng.gen_bool(0.5) { 1 } else { -1 })
            .collect();

        let mut solver = Self {
            graph,
            spins,
            delta: vec![0.0; n],
            energy: 0.0,
        };

        solver.initialize();
        solver
    }

    fn initialize(&mut self) {
        let n = self.graph.n;

        for i in 0..n {
            for edge in &self.graph.adj[i] {
                if i < edge.to {
                    if self.spins[i] != self.spins[edge.to] {
                        self.energy += edge.weight;
                    }
                }
            }
        }

        for i in 0..n {
            self.delta[i] = self.compute_delta(i);
        }
    }

    fn compute_delta(&self, node: usize) -> f64 {
        let mut gain = 0.0;
        let s = self.spins[node];

        for edge in &self.graph.adj[node] {
            let neighbor_spin = self.spins[edge.to];

            if s == neighbor_spin {
                gain += edge.weight;
            } else {
                gain -= edge.weight;
            }
        }

        gain
    }

    fn flip(&mut self, node: usize) {
        let delta_e = self.delta[node];

        self.spins[node] *= -1;
        self.energy += delta_e;

        for edge in &self.graph.adj[node] {
            let j = edge.to;
            self.delta[j] = self.compute_delta(j);
        }

        self.delta[node] = -delta_e;
    }

    fn optimize(&mut self, iterations: usize) {
        let mut rng = rand::thread_rng();

        for _ in 0..iterations {
            let node = rng.gen_range(0..self.graph.n);

            if self.delta[node] > 0.0 {
                self.flip(node);
            }
        }
    }
}

fn run_scaling_test(n: usize) {
    println!("--- Scaling test: {} nodes ---", n);

    let graph = Graph::random(n, 0.02);

    let start = Instant::now();
    let mut solver = IsingSolver::new(graph);
    solver.optimize(n * 50);
    let duration = start.elapsed();

    println!("Energy (cut value): {}", solver.energy);
    println!("Time: {:?}\n", duration);
}

fn main() {
    let sizes = vec![1000, 2000, 3000, 4000, 5000];

    for n in sizes {
        run_scaling_test(n);
    }
}
