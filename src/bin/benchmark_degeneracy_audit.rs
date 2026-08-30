//! What can a benchmark **provably not** measure?
//!
//! `memory/OPEN_PROBLEMS.md` §1 records a result this project paid for the hard
//! way: G-Set cannot distinguish energy-guided from constraint-guided search,
//! and not for want of samples. For an instance whose edge weights all share one
//! magnitude `c`, with `V` violated and `S` satisfied edges,
//!
//! ```text
//!     E = c(V − S),  V + S = |E|   ⇒   E = 2cV − c|E|
//! ```
//!
//! an affine bijection. Ranking by violation count **is** ranking by energy, so
//! the two methods are one method written twice. **18,570 recorded experiments**
//! ran on such instances before anyone noticed.
//!
//! That is a property of the *file*, not of the sample size, so it can be
//! decided by reading the file. This tool decides it, and two more like it, for
//! every instance of a corpus — before any compute is spent asking a question
//! the corpus cannot answer.
//!
//! It measures the benchmark, never a solver. It reports no performance number
//! and makes no claim about any method's quality.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct Instance {
    name: String,
    n: usize,
    edges: usize,
    self_loops: usize,
    distinct_weights: usize,
    distinct_magnitudes: usize,
    negative_edges: usize,
    triangles: u64,
    frustrated_triangles: u64,
    mean_degree: f64,
}

impl Instance {
    /// The guide axis is unidentifiable when every edge shares one magnitude:
    /// energy and violation count are then affinely equivalent.
    fn guide_axis_unidentifiable(&self) -> bool {
        self.distinct_magnitudes <= 1
    }
    /// Frustration can only be studied where **both** kinds of triangle exist.
    /// A corpus of only-frustrated triangles has no control group, so a
    /// frustration split there measures something else — degree, usually.
    fn has_frustration_control(&self) -> bool {
        self.frustrated_triangles > 0 && self.triangles > self.frustrated_triangles
    }
    fn is_maxcut(&self) -> bool {
        self.self_loops == 0
    }
}

fn parse(path: &Path) -> Result<Instance, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lines = text
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>())
        .filter(|t| !t.is_empty());
    let header = lines.next().ok_or("empty file")?;
    let n: usize = header.first().ok_or("no n")?.parse().map_err(|_| "bad n")?;
    if n == 0 || n > 5_000_000 {
        return Err(format!("implausible n={n}"));
    }
    let mut inst = Instance {
        name: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string(),
        n,
        ..Default::default()
    };
    let mut adj: Vec<Vec<(u32, f64)>> = vec![Vec::new(); n];
    let mut weights = HashSet::new();
    let mut magnitudes = HashSet::new();
    for t in lines {
        if t.len() < 2 {
            continue;
        }
        let (u, v) = (
            t[0].parse::<usize>().map_err(|_| "bad u")?,
            t[1].parse::<usize>().map_err(|_| "bad v")?,
        );
        let w: f64 = if t.len() > 2 {
            t[2].parse().map_err(|_| "bad w")?
        } else {
            1.0
        };
        if u == v {
            inst.self_loops += 1;
            continue;
        }
        if u < 1 || v < 1 || u > n || v > n {
            return Err(format!("index out of range: {u} {v}"));
        }
        inst.edges += 1;
        if w < 0.0 {
            inst.negative_edges += 1;
        }
        weights.insert(w.to_bits());
        magnitudes.insert(w.abs().to_bits());
        adj[u - 1].push((v as u32 - 1, w));
        adj[v - 1].push((u as u32 - 1, w));
    }
    inst.distinct_weights = weights.len();
    inst.distinct_magnitudes = magnitudes.len();
    inst.mean_degree = if n > 0 {
        2.0 * inst.edges as f64 / n as f64
    } else {
        0.0
    };
    for row in adj.iter_mut() {
        row.sort_unstable_by_key(|&(j, _)| j);
    }
    // Triangle census. A triangle is frustrated iff the product of its three
    // couplings is positive: with the MaxCut convention (minimise Σ w s s) an
    // all-positive triangle cannot have all three products negative. So an
    // all-positive-weight graph is frustrated *by construction* and offers no
    // control group — which is exactly the confound §1 recorded.
    for u in 0..n {
        for &(v, w_uv) in &adj[u] {
            if (v as usize) <= u {
                continue;
            }
            let (a, b) = (&adj[u], &adj[v as usize]);
            let (mut i, mut j) = (0usize, 0usize);
            while i < a.len() && j < b.len() {
                match a[i].0.cmp(&b[j].0) {
                    std::cmp::Ordering::Less => i += 1,
                    std::cmp::Ordering::Greater => j += 1,
                    std::cmp::Ordering::Equal => {
                        if a[i].0 as usize > v as usize {
                            inst.triangles += 1;
                            if w_uv * a[i].1 * b[j].1 > 0.0 {
                                inst.frustrated_triangles += 1;
                            }
                        }
                        i += 1;
                        j += 1;
                    }
                }
            }
        }
    }
    Ok(inst)
}

fn corpus_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && !matches!(
                        p.file_name().and_then(|s| s.to_str()),
                        Some("README.md") | Some("metadata.json")
                    )
                    && p.extension().and_then(|s| s.to_str()) != Some("json")
                    && p.extension().and_then(|s| s.to_str()) != Some("md")
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    v.sort();
    v
}

fn main() {
    let detail = std::env::args().any(|a| a == "--detail");
    let dirs: Vec<String> = {
        let a: Vec<String> = std::env::args()
            .skip(1)
            .filter(|a| a != "--detail")
            .collect();
        if a.is_empty() {
            ["gset", "biqmac", "dimacs_maxcut"]
                .iter()
                .map(|d| format!("benchmark_suite/data/{d}"))
                .collect()
        } else {
            a
        }
    };
    println!("# benchmark degeneracy audit — what can this corpus provably not measure?");
    println!("# Decided by reading each file. No solver is run and no quality is claimed.");
    println!(
        "corpus\tinstances\tmaxcut\tqubo\tguide_unidentifiable\tno_frustration_control\t\
         weight_magnitudes_min..max\tmean_degree_min..max"
    );
    let mut all = Vec::new();
    for dir in &dirs {
        let files = corpus_files(Path::new(dir));
        let mut parsed = Vec::new();
        for f in &files {
            match parse(f) {
                Ok(i) => parsed.push(i),
                Err(_) => continue, // not a rudy-format instance; excluded, counted below
            }
        }
        if parsed.is_empty() {
            println!("{dir}\t0\t-\t-\t-\t-\t-\t-");
            continue;
        }
        let maxcut: Vec<&Instance> = parsed.iter().filter(|i| i.is_maxcut()).collect();
        let qubo = parsed.len() - maxcut.len();
        let unident = maxcut
            .iter()
            .filter(|i| i.guide_axis_unidentifiable())
            .count();
        let no_ctrl = maxcut
            .iter()
            .filter(|i| !i.has_frustration_control())
            .count();
        let mags: Vec<usize> = maxcut.iter().map(|i| i.distinct_magnitudes).collect();
        let degs: Vec<f64> = maxcut.iter().map(|i| i.mean_degree).collect();
        println!(
            "{dir}\t{}\t{}\t{qubo}\t{unident}/{}\t{no_ctrl}/{}\t{}..{}\t{:.1}..{:.1}",
            parsed.len(),
            maxcut.len(),
            maxcut.len(),
            maxcut.len(),
            mags.iter().min().copied().unwrap_or(0),
            mags.iter().max().copied().unwrap_or(0),
            degs.iter().cloned().fold(f64::INFINITY, f64::min),
            degs.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        );
        all.extend(parsed);
    }
    let mc: Vec<&Instance> = all.iter().filter(|i| i.is_maxcut()).collect();
    let unident: Vec<&&Instance> = mc
        .iter()
        .filter(|i| i.guide_axis_unidentifiable())
        .collect();
    let no_ctrl: Vec<&&Instance> = mc.iter().filter(|i| !i.has_frustration_control()).collect();
    eprintln!("\n=== VERDICT over {} MaxCut instances ===", mc.len());
    eprintln!(
        "guide axis UNIDENTIFIABLE (E = 2cV − c|E|, energy ≡ violation count): {}/{} = {:.1}%",
        unident.len(),
        mc.len(),
        100.0 * unident.len() as f64 / mc.len() as f64
    );
    eprintln!(
        "frustration has NO CONTROL GROUP (every triangle frustrated, or none):  {}/{} = {:.1}%",
        no_ctrl.len(),
        mc.len(),
        100.0 * no_ctrl.len() as f64 / mc.len() as f64
    );
    let both = mc
        .iter()
        .filter(|i| i.guide_axis_unidentifiable() && !i.has_frustration_control())
        .count();
    eprintln!("degenerate on BOTH axes at once: {both}/{} ", mc.len());

    // The actionable half of the audit: which instances can still answer these
    // questions. A corpus is not useless because most of it is degenerate — it
    // is useless if you do not know which part is not.
    let usable: Vec<&&Instance> = mc
        .iter()
        .filter(|i| !i.guide_axis_unidentifiable() && i.has_frustration_control())
        .collect();
    eprintln!(
        "clean on BOTH axes (usable for either question): {}/{}",
        usable.len(),
        mc.len()
    );
    if detail {
        println!("\n# per-instance detail");
        println!(
            "instance\tn\tedges\tmean_degree\tdistinct_weights\tmagnitudes\t\
             triangles\tfrustrated\tguide_identifiable\tfrustration_control"
        );
        for i in &mc {
            println!(
                "{}\t{}\t{}\t{:.1}\t{}\t{}\t{}\t{}\t{}\t{}",
                i.name,
                i.n,
                i.edges,
                i.mean_degree,
                i.distinct_weights,
                i.distinct_magnitudes,
                i.triangles,
                i.frustrated_triangles,
                !i.guide_axis_unidentifiable(),
                i.has_frustration_control()
            );
        }
    } else {
        eprintln!("\nRe-run with --detail for the per-instance table.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(name: &str, body: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("degeneracy_audit_test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let p = dir.join(name);
        std::fs::write(&p, body).expect("write");
        p
    }

    /// The whole audit rests on this: one magnitude ⇒ energy and violation count
    /// are affinely equivalent ⇒ the guide axis cannot be studied. Two
    /// magnitudes breaks the bijection even when the signs are mixed.
    #[test]
    fn one_magnitude_is_unidentifiable_and_two_are_not() {
        let unit = parse(&write("u.mc", "3 3\n1 2 1\n2 3 1\n1 3 1\n")).expect("parse");
        assert!(unit.guide_axis_unidentifiable());
        let signed = parse(&write("s.mc", "3 3\n1 2 1\n2 3 -1\n1 3 1\n")).expect("parse");
        assert_eq!(signed.distinct_weights, 2);
        assert_eq!(signed.distinct_magnitudes, 1);
        assert!(
            signed.guide_axis_unidentifiable(),
            "±1 is still one magnitude: the bijection survives a sign flip"
        );
        let weighted = parse(&write("w.mc", "3 3\n1 2 1\n2 3 3\n1 3 1\n")).expect("parse");
        assert!(!weighted.guide_axis_unidentifiable());
    }

    /// An all-positive triangle is frustrated by construction, so a corpus of
    /// them cannot supply a control group. One sign flip supplies one.
    #[test]
    fn frustration_needs_both_kinds_of_triangle_to_be_studiable() {
        let all_pos = parse(&write("t1.mc", "3 3\n1 2 1\n2 3 1\n1 3 1\n")).expect("parse");
        assert_eq!(all_pos.triangles, 1);
        assert_eq!(all_pos.frustrated_triangles, 1);
        assert!(!all_pos.has_frustration_control());

        let one_flip = parse(&write("t2.mc", "3 3\n1 2 1\n2 3 -1\n1 3 1\n")).expect("parse");
        assert_eq!(one_flip.triangles, 1);
        assert_eq!(
            one_flip.frustrated_triangles, 0,
            "one negative edge relieves it"
        );
        assert!(!one_flip.has_frustration_control(), "still only one kind");

        // Two triangles, one of each kind: now the split has a control group.
        let mixed =
            parse(&write("t3.mc", "4 5\n1 2 1\n2 3 1\n1 3 1\n1 4 1\n3 4 -1\n")).expect("parse");
        assert_eq!(mixed.triangles, 2);
        assert_eq!(mixed.frustrated_triangles, 1);
        assert!(mixed.has_frustration_control());
    }

    #[test]
    fn a_self_loop_marks_the_file_as_qubo_rather_than_maxcut() {
        let q = parse(&write("q.mc", "3 3\n1 1 -5\n1 2 3\n2 3 4\n")).expect("parse");
        assert_eq!(q.self_loops, 1);
        assert!(!q.is_maxcut());
        assert_eq!(q.edges, 2, "the loop is a linear term, not an edge");
    }

    #[test]
    fn triangles_are_counted_once_each() {
        // K4 has exactly four triangles.
        let k4 = parse(&write(
            "k4.mc",
            "4 6\n1 2 1\n1 3 1\n1 4 1\n2 3 1\n2 4 1\n3 4 1\n",
        ))
        .expect("parse");
        assert_eq!(k4.triangles, 4);
        assert_eq!(k4.frustrated_triangles, 4);
        assert_eq!(k4.mean_degree, 3.0);
    }
}
