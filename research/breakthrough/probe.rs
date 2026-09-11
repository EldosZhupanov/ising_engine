//! Behavior-changing, isolated EXP001 passes. No production integration.
use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::UltimateSolver;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

const STRIDE: u64 = 0x9e3779b97f4a7c15;
struct SplitMix(u64);
impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(STRIDE);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn uniform(&mut self) -> f64 {
        (self.next() >> 11) as f64 / ((1u64 << 53) as f64)
    }
    fn weight(&mut self) -> f64 {
        let mag = (1 + self.next() % 5) as f64;
        if self.next() & 1 == 0 {
            mag
        } else {
            -mag
        }
    }
}
struct Instance {
    model: QuboModel,
    fields: Vec<f64>,
    edges: Vec<(usize, usize, f64)>,
}
fn model(linear: Vec<f64>, offset: f64, pairs: &[(usize, usize, f64)]) -> QuboModel {
    let n = linear.len();
    let mut rows = vec![BTreeMap::new(); n];
    for &(i, j, w) in pairs {
        assert!(i < j && j < n && w.is_finite());
        *rows[i].entry(j).or_insert(0.0) += w;
        *rows[j].entry(i).or_insert(0.0) += w;
    }
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for row in rows {
        for (j, w) in row {
            if w != 0.0 {
                col_indices.push(j);
                values.push(w);
            }
        }
        row_offsets.push(values.len());
    }
    QuboModel {
        num_vars: n,
        linear,
        energy_offset: offset,
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    }
}
impl Instance {
    fn new(family: &str, n: usize, seed: u64) -> Self {
        assert!(n >= 4 && n.is_multiple_of(8));
        let mut rng = SplitMix(seed);
        let mut topology = BTreeSet::new();
        let pair = |i, j| if i < j { (i, j) } else { (j, i) };
        match family {
            "cycle" => {
                for i in 0..n {
                    topology.insert(pair(i, (i + 1) % n));
                }
            }
            "subdivided" => {
                let k = n / 4;
                let mut core = BTreeSet::new();
                for i in 0..k {
                    core.insert(pair(i, (i + 1) % k));
                    core.insert(pair(i, (i + k / 2) % k));
                }
                let mut next = k;
                for (i, j) in core {
                    topology.insert(pair(i, next));
                    topology.insert((next, next + 1));
                    topology.insert(pair(next + 1, j));
                    next += 2;
                }
                assert_eq!(next, n);
            }
            "sparse" | "dense" | "field" => {
                let p = if family == "dense" {
                    0.5
                } else {
                    (6.0 / (n - 1) as f64).min(1.0)
                };
                for i in 0..n {
                    for j in i + 1..n {
                        if rng.uniform() < p {
                            topology.insert((i, j));
                        }
                    }
                }
            }
            _ => panic!("unknown family"),
        }
        let edges: Vec<_> = topology
            .into_iter()
            .map(|(i, j)| (i, j, rng.weight()))
            .collect();
        let fields: Vec<_> = (0..n)
            .map(|_| {
                if family == "field" {
                    (rng.next() % 5) as f64 - 2.0
                } else {
                    0.0
                }
            })
            .collect();
        let mut linear: Vec<_> = fields.iter().map(|b| 2.0 * b).collect();
        let mut offset = -fields.iter().sum::<f64>();
        let mut pairs = Vec::new();
        for &(i, j, w) in &edges {
            offset += w;
            linear[i] -= 2.0 * w;
            linear[j] -= 2.0 * w;
            pairs.push((i, j, 4.0 * w));
        }
        Self {
            model: model(linear, offset, &pairs),
            fields,
            edges,
        }
    }
    fn native_energy(&self, x: &[i8]) -> f64 {
        self.fields
            .iter()
            .zip(x)
            .map(|(b, &v)| b * (2 * v - 1) as f64)
            .sum::<f64>()
            + self
                .edges
                .iter()
                .map(|&(i, j, w)| w * ((2 * x[i] - 1) * (2 * x[j] - 1)) as f64)
                .sum::<f64>()
    }
    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf29ce484222325u64;
        let mut feed = |bytes: [u8; 8]| {
            for b in bytes {
                h = (h ^ b as u64).wrapping_mul(0x100000001b3);
            }
        };
        feed((self.fields.len() as u64).to_le_bytes());
        for &b in &self.fields {
            feed(b.to_bits().to_le_bytes());
        }
        for &(i, j, w) in &self.edges {
            feed((i as u64).to_le_bytes());
            feed((j as u64).to_le_bytes());
            feed(w.to_bits().to_le_bytes());
        }
        h
    }
}
struct Elimination {
    v: usize,
    neighbors: Vec<(usize, f64)>,
    h: f64,
}
struct Reduced {
    model: QuboModel,
    map: Vec<usize>,
    history: Vec<Elimination>,
    table_evals: u64,
    degree_scans: u64,
}
fn reduce(m: &QuboModel, flags: u8) -> Reduced {
    let n = m.num_vars;
    let mut rows: Vec<BTreeMap<usize, f64>> =
        (0..n).map(|i| m.quadratic.get_row(i).collect()).collect();
    let mut live = vec![true; n];
    let mut h = m.linear.clone();
    let mut offset = m.energy_offset;
    let mut history = Vec::new();
    let mut table_evals = 0;
    let mut degree_scans = 0;
    loop {
        let v = (0..n).find(|&i| {
            degree_scans += 1;
            live[i]
                && ((flags & 1 != 0 && rows[i].len() <= 1)
                    || (flags & 2 != 0 && rows[i].len() == 2))
        });
        let Some(v) = v else { break };
        let neighbors: Vec<_> = rows[v].iter().map(|(&j, &w)| (j, w)).collect();
        let d = neighbors.len();
        let mut f = [0.0f64; 4];
        for (bits, value) in f.iter_mut().enumerate().take(1 << d) {
            *value = h[v];
            for (k, &(_, w)) in neighbors.iter().enumerate() {
                *value += w * ((bits >> k) & 1) as f64;
            }
            *value = value.min(0.0);
            table_evals += 1;
        }
        offset += f[0];
        if d >= 1 {
            h[neighbors[0].0] += f[1] - f[0];
        }
        if d == 2 {
            let (i, j) = (neighbors[0].0, neighbors[1].0);
            h[j] += f[2] - f[0];
            let w = rows[i].get(&j).copied().unwrap_or(0.0) + f[3] - f[1] - f[2] + f[0];
            if w == 0.0 {
                rows[i].remove(&j);
                rows[j].remove(&i);
            } else {
                rows[i].insert(j, w);
                rows[j].insert(i, w);
            }
        }
        for &(j, _) in &neighbors {
            rows[j].remove(&v);
        }
        rows[v].clear();
        live[v] = false;
        history.push(Elimination {
            v,
            neighbors,
            h: h[v],
        });
    }
    let map: Vec<_> = (0..n).filter(|&i| live[i]).collect();
    let mut inverse = vec![usize::MAX; n];
    for (i, &v) in map.iter().enumerate() {
        inverse[v] = i;
    }
    let mut pairs = Vec::new();
    for &i in &map {
        for (&j, &w) in &rows[i] {
            if i < j {
                pairs.push((inverse[i], inverse[j], w));
            }
        }
    }
    Reduced {
        model: model(map.iter().map(|&i| h[i]).collect(), offset, &pairs),
        map,
        history,
        table_evals,
        degree_scans,
    }
}
impl Reduced {
    fn lift(&self, y: &[i8], x: &mut [i8]) {
        assert_eq!(y.len(), self.map.len());
        x.fill(0);
        for (&i, &v) in self.map.iter().zip(y) {
            x[i] = v;
        }
        for e in self.history.iter().rev() {
            let a = e.h
                + e.neighbors
                    .iter()
                    .map(|&(j, w)| w * x[j] as f64)
                    .sum::<f64>();
            x[e.v] = i8::from(a < 0.0);
        }
    }
}
#[derive(Default, Clone, Copy)]
struct Counts {
    delta_evals: u64,
    gain_scans: u64,
    pair_queries: u64,
    neighbor_updates: u64,
    pair_moves: u64,
    single_moves: u64,
}
impl Counts {
    fn difference(self, before: Self) -> Self {
        Self {
            delta_evals: self.delta_evals - before.delta_evals,
            gain_scans: self.gain_scans - before.gain_scans,
            pair_queries: self.pair_queries - before.pair_queries,
            neighbor_updates: self.neighbor_updates - before.neighbor_updates,
            pair_moves: self.pair_moves - before.pair_moves,
            single_moves: self.single_moves - before.single_moves,
        }
    }
    fn add(&mut self, other: Self) {
        self.delta_evals += other.delta_evals;
        self.gain_scans += other.gain_scans;
        self.pair_queries += other.pair_queries;
        self.neighbor_updates += other.neighbor_updates;
        self.pair_moves += other.pair_moves;
        self.single_moves += other.single_moves;
    }
}
struct Refinement {
    delta: Vec<f64>,
    counts: Counts,
}
impl Refinement {
    fn new(n: usize) -> Self {
        Self {
            delta: vec![0.0; n],
            counts: Counts::default(),
        }
    }
    fn rebuild(&mut self, m: &QuboModel, x: &[i8]) {
        for i in 0..m.num_vars {
            self.delta[i] = (1 - 2 * x[i]) as f64
                * (m.linear[i]
                    + m.quadratic
                        .get_row(i)
                        .map(|(j, w)| w * x[j] as f64)
                        .sum::<f64>());
        }
        self.counts.delta_evals += m.num_vars as u64;
    }
    fn flip(&mut self, m: &QuboModel, x: &mut [i8], i: usize) {
        let direction = (1 - 2 * x[i]) as f64;
        for (j, w) in m.quadratic.get_row(i) {
            self.delta[j] += (1 - 2 * x[j]) as f64 * w * direction;
            self.counts.neighbor_updates += 1;
        }
        self.delta[i] = -self.delta[i];
        x[i] ^= 1;
    }
    fn singles(&mut self, m: &QuboModel, x: &mut [i8]) -> f64 {
        let mut improvement = 0.0;
        loop {
            let mut best = None;
            let mut gain = 0.0;
            for (i, &d) in self.delta.iter().enumerate() {
                self.counts.gain_scans += 1;
                if d < gain {
                    gain = d;
                    best = Some(i);
                }
            }
            let Some(i) = best else { break };
            improvement -= gain;
            self.flip(m, x, i);
            self.counts.single_moves += 1;
        }
        improvement
    }
    fn pairs(&mut self, m: &QuboModel, x: &mut [i8]) -> f64 {
        let mut improvement = 0.0;
        for _ in 0..m.num_vars {
            let mut best = None;
            let mut gain = 0.0;
            for i in 0..m.num_vars {
                for (j, w) in m.quadratic.get_row(i) {
                    if i < j {
                        self.counts.pair_queries += 1;
                        let d = self.delta[i]
                            + self.delta[j]
                            + w * ((1 - 2 * x[i]) * (1 - 2 * x[j])) as f64;
                        if d < gain {
                            gain = d;
                            best = Some((i, j));
                        }
                    }
                }
            }
            let Some((i, j)) = best else { break };
            improvement -= gain;
            self.flip(m, x, i);
            self.flip(m, x, j);
            self.counts.pair_moves += 1;
            improvement += self.singles(m, x);
        }
        improvement
    }
}
fn refine_candidate(
    m: &QuboModel,
    x: &mut [i8],
    refine: &mut Refinement,
    flags: u8,
) -> (f64, f64, f64, Counts) {
    let raw = m.calculate_total_energy(x);
    let before = refine.counts;
    refine.rebuild(m, x);
    let common = refine.singles(m, x);
    let common_counts = refine.counts.difference(before);
    let extra = if flags & 4 != 0 {
        refine.pairs(m, x)
    } else {
        0.0
    };
    (raw, common, extra, common_counts)
}
fn ultimate(m: &QuboModel, seed: u64, exchanges: usize) -> Vec<i8> {
    UltimateSolver::new(20.0, 0.2, 1, exchanges, Some(seed))
        .with_quantum_dims(1, 2, 1)
        .solve(m, &[])
}
struct Observation {
    best: f64,
    state: Vec<i8>,
    time_best: f64,
    time_target: Option<f64>,
    events: u64,
}
impl Observation {
    fn new(n: usize) -> Self {
        Self {
            best: f64::INFINITY,
            state: vec![0; n],
            time_best: 0.0,
            time_target: None,
            events: 0,
        }
    }
    fn see(&mut self, x: &[i8], energy: f64, target: f64, start: &Instant) {
        self.events += 1;
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        if energy < self.best {
            self.best = energy;
            self.state.copy_from_slice(x);
            self.time_best = ms;
        }
        if energy <= target && self.time_target.is_none() {
            self.time_target = Some(ms);
        }
    }
}
struct Run {
    obs: Observation,
    elapsed: f64,
    prep_ms: f64,
    audit_ms: f64,
    solves: u64,
    reduced_n: usize,
    reduced_m: usize,
    tables: u64,
    degree_scans: u64,
    refine: Counts,
    common_counts: Counts,
    pre_common_best: f64,
    common_improvement: f64,
    pair_improvement: f64,
    prep_overrun: bool,
}
fn execute(
    inst: &Instance,
    seed: u64,
    flags: u8,
    exchanges: usize,
    wall: bool,
    target: f64,
) -> Run {
    let start = Instant::now();
    let m = &inst.model;
    let n = m.num_vars;
    let reduced = if flags & 3 != 0 {
        Some(reduce(m, flags))
    } else {
        None
    };
    let search = reduced.as_ref().map_or(m, |r| &r.model);
    let mut refine = Refinement::new(n);
    let mut x = vec![0; n];
    let mut obs = Observation::new(n);
    let prep_ms = start.elapsed().as_secs_f64() * 1000.0;
    let mut solves = 0;
    let mut units = 0u64;
    let mut pre_common_best = f64::INFINITY;
    let mut common_counts = Counts::default();
    let mut common_improvement = 0.0;
    let mut pair_improvement = 0.0;
    let prep_overrun = wall && prep_ms >= 100.0;
    if prep_overrun {
        let y = vec![0; search.num_vars];
        if let Some(r) = &reduced {
            r.lift(&y, &mut x);
        } else {
            x.copy_from_slice(&y);
        }
        obs.see(&x, m.calculate_total_energy(&x), target, &start);
    } else {
        loop {
            let child_seed = seed.wrapping_add(units.wrapping_mul(STRIDE));
            let y = if search.num_vars == 0 {
                Vec::new()
            } else {
                solves += 1;
                ultimate(search, child_seed, exchanges)
            };
            if let Some(r) = &reduced {
                r.lift(&y, &mut x);
            } else {
                x.copy_from_slice(&y);
            }
            let (raw, common, extra, counts) = refine_candidate(m, &mut x, &mut refine, flags);
            pre_common_best = pre_common_best.min(raw);
            common_improvement += common;
            pair_improvement += extra;
            common_counts.add(counts);
            // The first target observation is after the complete registered atomic unit.
            obs.see(&x, raw - common - extra, target, &start);
            units += 1;
            if !wall || start.elapsed().as_secs_f64() >= 0.1 {
                break;
            }
        }
    }
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    // Independent full original-objective audit outside timed algorithm section.
    let audit_start = Instant::now();
    assert_eq!(obs.events, if prep_overrun { 1 } else { units });
    assert_eq!(obs.best, m.calculate_total_energy(&obs.state));
    assert_eq!(obs.best, inst.native_energy(&obs.state));
    Run {
        obs,
        elapsed,
        prep_ms,
        audit_ms: audit_start.elapsed().as_secs_f64() * 1000.0,
        solves,
        reduced_n: search.num_vars,
        reduced_m: search.quadratic.values.len() / 2,
        tables: reduced.as_ref().map_or(0, |r| r.table_evals),
        degree_scans: reduced.as_ref().map_or(0, |r| r.degree_scans),
        refine: refine.counts,
        common_counts,
        pre_common_best,
        common_improvement,
        pair_improvement,
        prep_overrun,
    }
}
fn bits(x: &[i8]) -> String {
    x.iter().map(|&b| if b == 0 { '0' } else { '1' }).collect()
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    assert!(a.len()>=5,"target FAMILY N INSTANCE_SEED | run MODE FAMILY N INSTANCE_SEED SOLVER_SEED MASK EXCHANGES TARGET");
    let target_mode = a[1] == "target";
    assert!(target_mode || a[1] == "run");
    let off = if target_mode { 2 } else { 3 };
    assert_eq!(a.len(), if target_mode { 5 } else { 10 });
    let family = &a[off];
    let n: usize = a[off + 1].parse().unwrap();
    let iseed = a[off + 2].parse().unwrap();
    let generation_start = Instant::now();
    let inst = Instance::new(family, n, iseed);
    let generator_ms = generation_start.elapsed().as_secs_f64() * 1000.0;
    if target_mode {
        let start = Instant::now();
        let mut best = f64::INFINITY;
        let mut state = vec![0; n];
        if n <= 16 {
            let mut x = vec![0; n];
            for mask in 0..1u64 << n {
                for (i, v) in x.iter_mut().enumerate() {
                    *v = ((mask >> i) & 1) as i8;
                }
                let e = inst.native_energy(&x);
                if e < best {
                    best = e;
                    state.copy_from_slice(&x);
                }
            }
        } else {
            state = ultimate(&inst.model, 9001, 64);
            let mut polish = Refinement::new(n);
            refine_candidate(&inst.model, &mut state, &mut polish, 0);
            best = inst.native_energy(&state);
        }
        assert_eq!(best, inst.model.calculate_total_energy(&state));
        println!(
            "{family}\t{n}\t{iseed}\t{:016x}\t{best}\t{}\t{}\t{:.6}",
            inst.fingerprint(),
            bits(&state),
            if n <= 16 {
                "optimum"
            } else {
                "feasible_reference"
            },
            start.elapsed().as_secs_f64() * 1000.0
        );
    } else {
        let wall = match a[2].as_str() {
            "fixed" => false,
            "wall" => true,
            _ => panic!("unknown mode"),
        };
        let seed = a[6].parse().unwrap();
        let flags: u8 = a[7].parse().unwrap();
        assert!(flags < 8);
        let exchanges = a[8].parse().unwrap();
        assert!(exchanges > 0);
        assert!(!wall || exchanges == 1);
        let target: f64 = a[9].parse().unwrap();
        assert!(target.is_finite());
        let r = execute(&inst, seed, flags, exchanges, wall, target);
        let ttt = r
            .obs
            .time_target
            .map_or_else(|| "NA".into(), |v| format!("{v:.6}"));
        let nominal = r.solves * 128 * r.reduced_n as u64 * exchanges as u64;
        print!("{}\t{family}\t{n}\t{iseed}\t{seed}\t{flags}\t{exchanges}\t{:016x}\t{}\t{target}\t{}\t{ttt}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",a[2],inst.fingerprint(),r.obs.best,u8::from(r.obs.best<=target),r.obs.time_best,r.elapsed,r.prep_ms,r.solves,r.reduced_n,r.reduced_m,r.tables,r.degree_scans,r.refine.delta_evals,r.refine.gain_scans,r.refine.pair_queries,r.refine.neighbor_updates,r.refine.pair_moves,r.refine.single_moves,r.pre_common_best,r.common_improvement,r.pair_improvement,nominal,u8::from(r.prep_overrun),bits(&r.obs.state));
        println!(
            "\t{generator_ms:.6}\t{:.6}\t{}\t{}\t{}\t{}\t{}",
            r.audit_ms,
            r.common_counts.delta_evals,
            r.common_counts.gain_scans,
            r.common_counts.neighbor_updates,
            r.common_counts.single_moves,
            r.obs.events
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn assignments(n: usize) -> impl Iterator<Item = Vec<i8>> {
        (0..1usize << n).map(move |b| (0..n).map(|i| ((b >> i) & 1) as i8).collect())
    }
    fn close(a: f64, b: f64) {
        assert!(
            (a - b).abs() <= 1e-9 * (1.0 + a.abs().max(b.abs())),
            "{a} != {b}"
        );
    }
    #[test]
    fn elimination_is_exact_for_every_residual_assignment() {
        for seed in 1..25 {
            let mut rng = SplitMix(seed);
            let n = 7;
            let mut pairs = Vec::new();
            for i in 0..n {
                for j in i + 1..n {
                    if rng.uniform() < 0.35 {
                        pairs.push((i, j, rng.weight() / if seed % 2 == 0 { 1.0 } else { 3.0 }));
                    }
                }
            }
            let m = model(
                (0..n)
                    .map(|_| rng.weight() / if seed % 2 == 0 { 1.0 } else { 7.0 })
                    .collect(),
                3.25,
                &pairs,
            );
            for flags in 0..4 {
                let r = reduce(&m, flags);
                for y in assignments(r.map.len()) {
                    let mut x = vec![0; n];
                    r.lift(&y, &mut x);
                    let lifted = m.calculate_total_energy(&x);
                    close(lifted, r.model.calculate_total_energy(&y));
                    let brute = assignments(n)
                        .filter(|z| r.map.iter().zip(&y).all(|(&i, &v)| z[i] == v))
                        .map(|z| m.calculate_total_energy(&z))
                        .fold(f64::INFINITY, f64::min);
                    close(lifted, brute);
                }
            }
        }
    }
    #[test]
    fn pair_deltas_match_full_recomputation_for_all_pairs() {
        let m = model(
            vec![0.3, -2.7, 0.0, 1.25],
            -3.5,
            &[(0, 1, 4.5), (0, 3, -3.2), (1, 2, -0.75), (2, 3, 1.7)],
        );
        for x in assignments(4) {
            let mut f = Refinement::new(4);
            f.rebuild(&m, &x);
            let before = m.calculate_total_energy(&x);
            for i in 0..4 {
                for j in i + 1..4 {
                    let w = m
                        .quadratic
                        .get_row(i)
                        .find(|&(v, _)| v == j)
                        .map_or(0.0, |(_, w)| w);
                    let expected =
                        f.delta[i] + f.delta[j] + w * ((1 - 2 * x[i]) * (1 - 2 * x[j])) as f64;
                    let mut y = x.clone();
                    y[i] ^= 1;
                    y[j] ^= 1;
                    close(m.calculate_total_energy(&y) - before, expected);
                }
            }
        }
    }
    #[test]
    fn nonvacuous_pair_barrier_and_full_variable_restoration() {
        let m = model(vec![1.0, 1.0], 0.0, &[(0, 1, -3.0)]);
        let mut x = vec![0, 0];
        let mut f = Refinement::new(2);
        f.rebuild(&m, &x);
        assert_eq!(f.singles(&m, &mut x), 0.0);
        assert_eq!(f.pairs(&m, &mut x), 1.0);
        assert_eq!(x, vec![1, 1]);
        assert_eq!(f.counts.pair_moves, 1);
        let m = model(vec![-2.0], 0.0, &[]);
        let mut x = vec![0];
        ising_engine::solver::local_search::steepest_descent_1opt(&m, &mut x, &[true]);
        assert_eq!(x[0], 0);
        let mut f = Refinement::new(1);
        f.rebuild(&m, &x);
        assert_eq!(f.singles(&m, &mut x), 2.0);
        assert_eq!(x[0], 1);
    }
    #[test]
    fn no_reduction_and_null_identity() {
        let m = model(
            vec![0.0; 4],
            2.0,
            &[
                (0, 1, 1.0),
                (0, 2, 1.0),
                (0, 3, 1.0),
                (1, 2, 1.0),
                (1, 3, 1.0),
                (2, 3, 1.0),
            ],
        );
        for flags in 0..4 {
            let r = reduce(&m, flags);
            assert!(r.history.is_empty());
            assert_eq!(r.model.quadratic.values, m.quadratic.values);
            assert_eq!(r.model.linear, m.linear);
            assert_eq!(r.model.energy_offset, m.energy_offset);
        }
    }
    #[test]
    fn field_generation_and_binary_conversion_are_exact() {
        for family in ["cycle", "subdivided", "sparse", "dense", "field"] {
            let n = if family == "subdivided" { 16 } else { 8 };
            let i = Instance::new(family, n, 71);
            for x in assignments(n) {
                assert_eq!(i.native_energy(&x), i.model.calculate_total_energy(&x));
            }
        }
    }
    #[test]
    fn refinement_never_worsens_and_ledger_survives() {
        let i = Instance::new("field", 8, 33);
        for mut x in assignments(8) {
            let before = i.native_energy(&x);
            let mut f = Refinement::new(8);
            f.rebuild(&i.model, &x);
            let improvement = f.singles(&i.model, &mut x) + f.pairs(&i.model, &mut x);
            assert!(improvement >= 0.0);
            assert_eq!(before - improvement, i.native_energy(&x));
            let old = f.delta.clone();
            f.rebuild(&i.model, &x);
            assert_eq!(old, f.delta);
            assert!(f.delta.iter().all(|&d| d >= 0.0));
        }
    }
    #[test]
    fn fixed_replay_and_one_observation_per_atomic_unit() {
        let i = Instance::new("field", 8, 32);
        for flags in 0..8 {
            let a = execute(&i, 81, flags, 1, false, -9999.0);
            let b = execute(&i, 81, flags, 1, false, -9999.0);
            assert_eq!(a.obs.state, b.obs.state);
            assert_eq!(a.obs.best, b.obs.best);
            assert_eq!(a.refine.pair_queries, b.refine.pair_queries);
            assert!(a.obs.time_target.is_none());
        }
    }
    #[test]
    fn gauge_counterexample_does_not_destroy_relational_information() {
        let states = [[1i8, 1], [-1, -1]];
        assert_eq!(states.iter().map(|s| s[1]).sum::<i8>(), 0);
        for s in states {
            let aligned = [s[0] * s[0], s[1] * s[0]];
            assert_eq!(aligned, [1, 1]);
            assert_eq!(-s[0] * s[1], -aligned[0] * aligned[1]);
        }
    }
    #[test]
    fn common_and_pair_work_are_separated_on_a_real_barrier() {
        let m = model(vec![1.0, 1.0], 0.0, &[(0, 1, -3.0)]);
        let mut x = vec![0, 0];
        let mut f = Refinement::new(2);
        let (raw, common, extra, counts) = refine_candidate(&m, &mut x, &mut f, 4);
        assert_eq!((raw, common, extra), (0.0, 0.0, 1.0));
        assert_eq!(
            (
                counts.delta_evals,
                counts.gain_scans,
                counts.neighbor_updates,
                counts.single_moves
            ),
            (2, 2, 0, 0)
        );
        let pair = f.counts.difference(counts);
        assert_eq!(
            (
                pair.pair_moves,
                pair.pair_queries,
                pair.neighbor_updates,
                pair.gain_scans
            ),
            (1, 2, 2, 2)
        );
        assert_eq!(m.calculate_total_energy(&x), raw - common - extra);
    }
}
