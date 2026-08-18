//! The scientist's **shared representation** — one versioned vocabulary of
//! structural features that EVERY faculty consumes (Predictor, Policy, World,
//! Dynamics, Meta-Learner, Curiosity, Scientific Memory).
//!
//! Before this, the 5-scalar encoding
//! `[ln(n+1)/10, density, clustering, mean_degree/10, degree_cv]` was copied
//! **privately and identically** in four modules (`predictor`, `policy`,
//! `dynamics`, `memory_os`). That is the "no subsystem may have a private
//! representation" violation, and it is why a newly discovered concept could not
//! propagate. Centralising the vocabulary here is the prerequisite for the
//! Concept Discovery Engine: [`FeatureRegistry::admit`] appends a concept and it
//! reaches every faculty automatically on their next fit.
//!
//! **v0 is BIT-IDENTICAL** to the prior hand-coded encoders — same features, same
//! order, same transforms. Reordering v0 would invalidate every persisted model
//! trained on this vocabulary version, so the order is frozen.

use std::sync::{Arc, LazyLock, RwLock};

use super::predictor::InstanceSignature;

/// One named structural feature: a deterministic `InstanceSignature → f64`.
/// The closure captures nothing, so it coerces to a plain `fn` pointer (Copy).
#[derive(Clone, Copy)]
pub struct Feature {
    pub name: &'static str,
    pub extract: fn(&InstanceSignature) -> f64,
}

impl std::fmt::Debug for Feature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Feature({})", self.name)
    }
}

/// The ordered, versioned feature vocabulary shared by all faculties.
#[derive(Clone, Debug)]
pub struct FeatureRegistry {
    version: u32,
    features: Vec<Feature>,
}

impl FeatureRegistry {
    /// v0 — the canonical 5 structural scalars, in the exact order and with the
    /// exact transforms the learned models were trained on. FROZEN.
    pub fn v0() -> Self {
        Self {
            version: 0,
            features: vec![
                Feature {
                    name: "log_n",
                    extract: |s| ((s.n as f64) + 1.0).ln() / 10.0,
                },
                Feature {
                    name: "density",
                    extract: |s| s.density,
                },
                Feature {
                    name: "clustering",
                    extract: |s| s.clustering,
                },
                Feature {
                    name: "mean_degree",
                    extract: |s| s.mean_degree / 10.0,
                },
                Feature {
                    name: "degree_cv",
                    extract: |s| s.degree_cv,
                },
            ],
        }
    }

    /// The vocabulary version. Models record this so a model is only ever compared
    /// against models of the same representation (reproducibility, ADR-0004).
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn len(&self) -> usize {
        self.features.len()
    }
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.features.iter().map(|f| f.name)
    }

    /// Append features into an existing buffer — allocation-free, for hot paths
    /// (e.g. the Predictor's leave-one-out sufficient statistics).
    pub fn encode_into(&self, sig: &InstanceSignature, out: &mut Vec<f64>) {
        for f in &self.features {
            out.push((f.extract)(sig));
        }
    }

    /// The ordered feature vector for `sig`.
    pub fn encode(&self, sig: &InstanceSignature) -> Vec<f64> {
        let mut v = Vec::with_capacity(self.features.len());
        self.encode_into(sig, &mut v);
        v
    }

    /// Append the first `k` features into `out` (`k` clamped to len). Because the
    /// vocabulary is append-only, the prefix is STABLE across growth — so a model
    /// fit at width `k` keeps encoding the same `k` features after new concepts
    /// are admitted, and never mismatches its weights.
    pub fn encode_prefix_into(&self, sig: &InstanceSignature, out: &mut Vec<f64>, k: usize) {
        for f in self.features.iter().take(k) {
            out.push((f.extract)(sig));
        }
    }

    /// Admit a discovered concept (Concept Discovery Engine). Append-only: existing
    /// indices never shift, so older-vocabulary reasoning stays valid. Bumps and
    /// returns the new version.
    pub fn admit(&mut self, feature: Feature) -> u32 {
        // Idempotent on name: never register the same concept twice.
        if self.features.iter().any(|f| f.name == feature.name) {
            return self.version;
        }
        self.features.push(feature);
        self.version += 1;
        self.version
    }

    /// Whether a concept of this name is already in the vocabulary.
    pub fn contains(&self, name: &str) -> bool {
        self.features.iter().any(|f| f.name == name)
    }
}

impl Default for FeatureRegistry {
    fn default() -> Self {
        Self::v0()
    }
}

/// The process-wide v0 vocabulary — immutable, so every faculty encodes
/// identically and determinism holds. Concept discovery builds *new* registry
/// versions (persisted with the model); it never mutates this one in place.
static V0: LazyLock<FeatureRegistry> = LazyLock::new(FeatureRegistry::v0);

/// Borrow the shared v0 vocabulary (the frozen SEED).
pub fn v0() -> &'static FeatureRegistry {
    &V0
}

/// The LIVE shared vocabulary — starts at v0 and grows as the Concept Discovery
/// Engine admits concepts. Grown only at orchestrator tick boundaries, so within
/// a single run the vocabulary is fixed (determinism holds; and the state is
/// recomputable from the append-only DB — STAGE_8 §3). Cheap to read (Arc clone).
static CURRENT: LazyLock<RwLock<Arc<FeatureRegistry>>> =
    LazyLock::new(|| RwLock::new(Arc::new(FeatureRegistry::v0())));

/// Snapshot the live vocabulary. Callers hold this for the duration of one fit or
/// predict so encoding is consistent even if a concept is admitted concurrently.
pub fn current() -> Arc<FeatureRegistry> {
    CURRENT
        .read()
        .expect("feature registry lock poisoned")
        .clone()
}

/// Replace the live vocabulary (append-only growth only). Returns its version.
pub fn set_current(reg: FeatureRegistry) -> u32 {
    let v = reg.version();
    *CURRENT.write().expect("feature registry lock poisoned") = Arc::new(reg);
    v
}

/// Admit one concept into the live vocabulary; returns the new version.
pub fn admit_current(feature: Feature) -> u32 {
    let mut reg = (*current()).clone();
    let v = reg.admit(feature);
    set_current(reg);
    v
}

/// Reset the live vocabulary to v0 (determinism boundary / test hygiene).
pub fn reset_current_to_v0() {
    *CURRENT.write().expect("feature registry lock poisoned") = Arc::new(FeatureRegistry::v0());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig() -> InstanceSignature {
        InstanceSignature {
            n: 100,
            density: 0.25,
            clustering: 0.4,
            mean_degree: 24.75,
            degree_cv: 0.13,
        }
    }

    #[test]
    fn v0_is_bit_identical_to_the_prior_hand_coded_encoder() {
        let s = sig();
        // The exact vector the private encoders in predictor/policy/dynamics/memory
        // produced before centralisation.
        let expected = [
            ((s.n as f64) + 1.0).ln() / 10.0,
            s.density,
            s.clustering,
            s.mean_degree / 10.0,
            s.degree_cv,
        ];
        let got = v0().encode(&s);
        assert_eq!(got.len(), 5);
        for (g, e) in got.iter().zip(expected.iter()) {
            assert_eq!(g.to_bits(), e.to_bits(), "bit-identical encoding required");
        }
        assert_eq!(v0().version(), 0);
        assert_eq!(
            v0().names().collect::<Vec<_>>(),
            ["log_n", "density", "clustering", "mean_degree", "degree_cv"]
        );
    }

    #[test]
    fn encode_into_matches_encode() {
        let s = sig();
        let mut buf = vec![1.0]; // pre-existing content (e.g. a bias term)
        v0().encode_into(&s, &mut buf);
        assert_eq!(buf.len(), 6);
        assert_eq!(&buf[1..], v0().encode(&s).as_slice());
    }

    #[test]
    fn admit_appends_bumps_version_and_is_idempotent() {
        let mut reg = FeatureRegistry::v0();
        let v = reg.admit(Feature {
            name: "ruggedness",
            extract: |s| s.density * 2.0, // placeholder concept
        });
        assert_eq!(v, 1);
        assert_eq!(reg.len(), 6);
        // existing indices unchanged (append-only)
        assert_eq!(reg.encode(&sig())[..5], v0().encode(&sig())[..]);
        // idempotent on name
        assert_eq!(
            reg.admit(Feature {
                name: "ruggedness",
                extract: |_| 0.0
            }),
            1
        );
        assert_eq!(reg.len(), 6);
    }

    #[test]
    fn live_vocabulary_grows_and_encode_prefix_is_stable() {
        reset_current_to_v0();
        assert_eq!(current().len(), 5);
        let s = sig();
        let seed = current().encode(&s);
        let v = admit_current(Feature {
            name: "density_x_clustering",
            extract: |s| s.density * s.clustering,
        });
        assert_eq!(v, 1);
        assert_eq!(current().len(), 6);
        // A model fit at width 5 keeps encoding the SAME 5 features (append-only).
        let mut prefix = Vec::new();
        current().encode_prefix_into(&s, &mut prefix, 5);
        assert_eq!(prefix, seed, "prefix stable across growth");
        reset_current_to_v0(); // hygiene for other tests
        assert_eq!(current().len(), 5);
    }
}
