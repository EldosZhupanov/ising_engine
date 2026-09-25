//! Structured Dataset Generators for EXP-TEN-002:
//! Generates structured, correlated, clustered, and adversarial memory sets.

use crate::types::SpinState;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Suite A: Random i.i.d. Rademacher patterns (P(s_i = +-1) = 0.5).
pub fn generate_random_suite(n: usize, p: usize, seed: u64) -> Vec<SpinState> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut patterns = Vec::with_capacity(p);
    for _ in 0..p {
        let spins: Vec<i8> = (0..n)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();
        patterns.push(SpinState::from_slice(&spins));
    }
    patterns
}

/// Suite B: Correlated patterns with average correlation rho in [0.0, 1.0].
pub fn generate_correlated_suite(n: usize, p: usize, rho: f64, seed: u64) -> Vec<SpinState> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut patterns = Vec::with_capacity(p);

    // Root ancestral pattern
    let root: Vec<i8> = (0..n)
        .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
        .collect();

    // P(match root) = (1 + rho) / 2
    let prob_match = ((1.0 + rho) / 2.0).clamp(0.0, 1.0);

    for _ in 0..p {
        let spins: Vec<i8> = (0..n)
            .map(|i| {
                if rng.gen::<f64>() < prob_match {
                    root[i]
                } else {
                    -root[i]
                }
            })
            .collect();
        patterns.push(SpinState::from_slice(&spins));
    }
    patterns
}

/// Suite C: Clustered patterns with K prototypes and mutation rate.
pub fn generate_clustered_suite(
    n: usize,
    p: usize,
    num_clusters: usize,
    mutation_rate: f64,
    seed: u64,
) -> Vec<SpinState> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut prototypes = Vec::with_capacity(num_clusters);
    for _ in 0..num_clusters {
        let spins: Vec<i8> = (0..n)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();
        prototypes.push(spins);
    }

    let mut patterns = Vec::with_capacity(p);
    for idx in 0..p {
        let proto = &prototypes[idx % num_clusters];
        let spins: Vec<i8> = (0..n)
            .map(|i| {
                if rng.gen::<f64>() < mutation_rate {
                    -proto[i]
                } else {
                    proto[i]
                }
            })
            .collect();
        patterns.push(SpinState::from_slice(&spins));
    }
    patterns
}

/// Suite D: Low-rank subspace patterns generated from k basis vectors: s = sign(B * w).
pub fn generate_low_rank_suite(n: usize, p: usize, rank_k: usize, seed: u64) -> Vec<SpinState> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    // Generate N x k continuous basis
    let mut basis = vec![0.0; n * rank_k];
    for v in &mut basis {
        *v = rng.gen_range(-1.0..1.0);
    }

    let mut patterns = Vec::with_capacity(p);
    for _ in 0..p {
        let weights: Vec<f64> = (0..rank_k).map(|_| rng.gen_range(-1.0..1.0)).collect();
        let mut spins = vec![1i8; n];
        for i in 0..n {
            let mut dot = 0.0;
            let offset = i * rank_k;
            for k in 0..rank_k {
                dot += basis[offset + k] * weights[k];
            }
            spins[i] = if dot >= 0.0 { 1 } else { -1 };
        }
        patterns.push(SpinState::from_slice(&spins));
    }
    patterns
}

/// Suite E: Adversarially close patterns with Hamming distance <= 2.
pub fn generate_adversarial_pairs_suite(n: usize, p: usize, seed: u64) -> Vec<SpinState> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut patterns = Vec::with_capacity(p);

    // Generate p/2 base patterns, and for each create an adversarial twin differing by 2 bits
    let num_pairs = p / 2;
    for _ in 0..num_pairs {
        let base: Vec<i8> = (0..n)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();
        let mut twin = base.clone();
        let flip1 = rng.gen_range(0..n);
        let mut flip2 = rng.gen_range(0..n);
        while flip2 == flip1 {
            flip2 = rng.gen_range(0..n);
        }
        twin[flip1] = -twin[flip1];
        twin[flip2] = -twin[flip2];

        patterns.push(SpinState::from_slice(&base));
        patterns.push(SpinState::from_slice(&twin));
    }

    while patterns.len() < p {
        let spins: Vec<i8> = (0..n)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();
        patterns.push(SpinState::from_slice(&spins));
    }

    patterns
}

/// Suite F: Biased / non-zero magnetization patterns: E[s_i] = m in (-1, 1).
pub fn generate_biased_suite(n: usize, p: usize, magnetization: f64, seed: u64) -> Vec<SpinState> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let prob_plus = ((1.0 + magnetization) / 2.0).clamp(0.0, 1.0);
    let mut patterns = Vec::with_capacity(p);

    for _ in 0..p {
        let spins: Vec<i8> = (0..n)
            .map(|_| if rng.gen::<f64>() < prob_plus { 1 } else { -1 })
            .collect();
        patterns.push(SpinState::from_slice(&spins));
    }
    patterns
}

/// Compositional Feature Grammar Dataset for EXP-TEN-003:
/// Creates a synthetic world of 2^(k-1) grammatical patterns generated by a k-dimensional
/// orthonormal basis G in R^(n x k): x(z) = sign(G z) with z_0 = +1.
/// Returns (train_patterns, unseen_patterns, generator_matrix G of shape [n, k]).
pub fn generate_compositional_grammar_suite(
    n: usize,
    k: usize,
    p_train: usize,
    seed: u64,
) -> (Vec<SpinState>, Vec<SpinState>, Vec<f64>) {
    assert!((2..=16).contains(&k), "k must be between 2 and 16");
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    // 1. Generate orthonormal matrix G in R^(n x k) via Gaussian sampling + Gram-Schmidt
    let mut g = vec![0.0; n * k];
    for col in 0..k {
        // Sample standard normal via Box-Muller
        for row in (0..n).step_by(2) {
            let u1: f64 = rng.gen_range(1e-7..1.0);
            let u2: f64 = rng.gen_range(0.0..std::f64::consts::TAU);
            let r = (-2.0 * u1.ln()).sqrt();
            let z0 = r * u2.cos();
            let z1 = r * u2.sin();
            g[row * k + col] = z0;
            if row + 1 < n {
                g[(row + 1) * k + col] = z1;
            }
        }

        // Gram-Schmidt orthogonalization against prior columns
        for prev in 0..col {
            let mut dot = 0.0;
            for row in 0..n {
                dot += g[row * k + col] * g[row * k + prev];
            }
            for row in 0..n {
                g[row * k + col] -= dot * g[row * k + prev];
            }
        }

        // Normalize
        let mut norm_sq = 0.0;
        for row in 0..n {
            let val = g[row * k + col];
            norm_sq += val * val;
        }
        let norm = norm_sq.sqrt().max(1e-10);
        for row in 0..n {
            g[row * k + col] /= norm;
        }
    }

    // 2. Enumerate all 2^(k-1) grammatical patterns (fixing z_0 = +1)
    let total_valid = 1usize << (k - 1);
    assert!(
        p_train < total_valid,
        "p_train must be less than total valid patterns 2^(k-1)"
    );
    let mut all_patterns = Vec::with_capacity(total_valid);

    for code in 0..total_valid {
        let mut z = vec![1.0; k]; // z_0 = +1
        for bit in 0..(k - 1) {
            if (code >> bit) & 1 == 1 {
                z[bit + 1] = 1.0;
            } else {
                z[bit + 1] = -1.0;
            }
        }

        // Compute x = sign(G z)
        let mut spins = Vec::with_capacity(n);
        for row in 0..n {
            let mut sum = 0.0;
            for col in 0..k {
                sum += g[row * k + col] * z[col];
            }
            spins.push(if sum >= 0.0 { 1i8 } else { -1i8 });
        }
        all_patterns.push(SpinState::from_slice(&spins));
    }

    // 3. Shuffle patterns
    for i in (1..total_valid).rev() {
        let j = rng.gen_range(0..=i);
        all_patterns.swap(i, j);
    }

    // 4. Split into train and unseen
    let train = all_patterns[0..p_train].to_vec();
    let unseen = all_patterns[p_train..].to_vec();

    (train, unseen, g)
}

/// A relational triple fact: (subject, relation, object).
#[derive(Debug, Clone)]
pub struct RelationalFact {
    pub subject_idx: usize,
    pub relation_idx: usize,
    pub object_idx: usize,
    pub s_subject: SpinState,
    pub s_relation: SpinState,
    pub s_object: SpinState,
}

/// A 2-hop relational reasoning query: (A, R1, R2) -> ? -> C.
#[derive(Debug, Clone)]
pub struct TwoHopQuery {
    pub subject_idx: usize,
    pub relation1_idx: usize,
    pub relation2_idx: usize,
    pub intermediate_target_idx: usize,
    pub final_target_idx: usize,
    pub s_subject: SpinState,
    pub s_relation1: SpinState,
    pub s_relation2: SpinState,
    pub s_intermediate_target: SpinState,
    pub s_final_target: SpinState,
}

/// The synthetic relational microworld for EXP-TEN-004.
#[derive(Debug, Clone)]
pub struct RelationalWorld {
    pub num_entities: usize,
    pub num_relations: usize,
    pub entity_dim: usize,
    pub relation_dim: usize,
    pub entities: Vec<SpinState>,
    pub relations: Vec<SpinState>,
    pub train_facts: Vec<RelationalFact>,
    pub test_queries: Vec<TwoHopQuery>,
}

/// Generate synthetic relational microworld with systematically held-out 2-hop compositions.
pub fn generate_relational_world(
    num_entities: usize,
    num_relations: usize,
    entity_dim: usize,
    relation_dim: usize,
    seed: u64,
) -> RelationalWorld {
    assert!(num_entities >= 8);
    assert!(num_relations >= 2);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    // 1. Generate distinct entity spin patterns
    let mut entities = Vec::with_capacity(num_entities);
    for _ in 0..num_entities {
        let spins: Vec<i8> = (0..entity_dim)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();
        entities.push(SpinState::from_slice(&spins));
    }

    // 2. Generate distinct relation spin patterns
    let mut relations = Vec::with_capacity(num_relations);
    for _ in 0..num_relations {
        let spins: Vec<i8> = (0..relation_dim)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();
        relations.push(SpinState::from_slice(&spins));
    }

    // 3. Define 2 independent permutations on entities for R0 and R1
    let mut perm1: Vec<usize> = (0..num_entities).collect();
    let mut perm2: Vec<usize> = (0..num_entities).collect();
    for i in (1..num_entities).rev() {
        let j1 = rng.gen_range(0..=i);
        perm1.swap(i, j1);
        let j2 = rng.gen_range(0..=i);
        perm2.swap(i, j2);
    }

    // 4. Generate 1-hop training facts: F1 (via R0) and F2 (via R1)
    let mut train_facts = Vec::with_capacity(num_entities * 2);
    for i in 0..num_entities {
        let obj1 = perm1[i];
        train_facts.push(RelationalFact {
            subject_idx: i,
            relation_idx: 0,
            object_idx: obj1,
            s_subject: entities[i].clone(),
            s_relation: relations[0].clone(),
            s_object: entities[obj1].clone(),
        });

        let obj2 = perm2[i];
        train_facts.push(RelationalFact {
            subject_idx: i,
            relation_idx: 1,
            object_idx: obj2,
            s_subject: entities[i].clone(),
            s_relation: relations[1].clone(),
            s_object: entities[obj2].clone(),
        });
    }

    // 5. Generate held-out 2-hop test queries: (e_i, R0, R1) -> intermediate e_{perm1[i]} -> final e_{perm2[perm1[i]]}
    let mut test_queries = Vec::with_capacity(num_entities);
    for i in 0..num_entities {
        let inter = perm1[i];
        let fin = perm2[inter];
        test_queries.push(TwoHopQuery {
            subject_idx: i,
            relation1_idx: 0,
            relation2_idx: 1,
            intermediate_target_idx: inter,
            final_target_idx: fin,
            s_subject: entities[i].clone(),
            s_relation1: relations[0].clone(),
            s_relation2: relations[1].clone(),
            s_intermediate_target: entities[inter].clone(),
            s_final_target: entities[fin].clone(),
        });
    }

    RelationalWorld {
        num_entities,
        num_relations,
        entity_dim,
        relation_dim,
        entities,
        relations,
        train_facts,
        test_queries,
    }
}
