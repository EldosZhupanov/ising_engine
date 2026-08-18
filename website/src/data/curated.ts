/**
 * Curated real constants — sourced from the repository (ROADMAP.md honesty
 * section, orchestrator.rs loop, CLAUDE.md). These are real facts, attributed;
 * not artifact rows but not invented either.
 */

export const LOOP_STAGES = [
  { n: "01", name: "Observe", module: "memory_os", desc: "Scientific Memory keys the instance signature — “have we seen this structure?”" },
  { n: "02", name: "Learn", module: "meta_learner", desc: "Mines the DB for statistically-supported rules, conditioned on structure" },
  { n: "03", name: "Discover", module: "concept", desc: "Proposes → tests → gates → admits new structural concepts" },
  { n: "04", name: "Theorize", module: "theory", desc: "Promotes a rule to a falsifiable mechanism; ablates to refute it" },
  { n: "05", name: "Plan", module: "planner", desc: "Ranks the next question by expected new knowledge, not energy" },
  { n: "06", name: "Run", module: "executor · runtime", desc: "Provenance-stamped experiments on the deterministic runtime" },
  { n: "07", name: "Refute", module: "monitor · theory", desc: "Failed hypotheses recorded, never hidden; health gates can pause" },
  { n: "08", name: "Record", module: "db · graph", desc: "Append-only DB + knowledge graph — nothing is lost" },
] as const;

export type LedgerItem = { status: "confirmed" | "refuted"; claim: string; detail: string; source: string };
export const LEDGER: LedgerItem[] = [
  { status: "confirmed", claim: "Cross-family transfer", detail: "MaxCut-trained policy ranks operators on non-MaxCut instances — 9/12, Spearman +0.747.", source: "ROADMAP · portability" },
  { status: "confirmed", claim: "World-model realism", detail: "Imagined operator ranking tracks reality at ρ 0.975.", source: "ROADMAP · confirmed" },
  { status: "confirmed", claim: "Determinism firewall", detail: "DenseByte backend 4.8× the f64 oracle and bit-identical on replay.", source: "ADR-0004 · ROADMAP" },
  { status: "confirmed", claim: "Causal separation", detail: "Theory Engine separates causal from spurious via ablation (metropolis_sweep costs 5% when removed).", source: "ROADMAP · Stage 8" },
  { status: "refuted", claim: "Self-proposed operator was weak", detail: "The loop's own extremal_metropolis appears in the worst solutions ~8× — tested and failed, honestly.", source: "ROADMAP · honest negatives" },
  { status: "refuted", claim: "No superiority over production", detail: "Evolved plans lose 5/5 to UltimateSolver at equal budget — no superiority claimed.", source: "ROADMAP · honest negatives" },
  { status: "refuted", claim: "Single structural scalar", detail: "One scalar predicting operator choice was refuted — density and ruggedness were confounded.", source: "memory · universal-law" },
];

/** Real backends (state.rs). */
export const BACKENDS = [
  { name: "ReferenceState", role: "f64 oracle — the ground-truth trajectory", tag: "exact" },
  { name: "SparseBitSlice", role: "exact integer, cross-checked past 100k spins", tag: "verified" },
  { name: "DenseByte", role: "production-shaped, 4.8× the oracle, bit-identical", tag: "fast" },
] as const;

/** Concept Discovery pipeline (concept.rs). */
export const CONCEPT_PIPELINE = [
  { stage: "Propose", desc: "A candidate is a deterministic InstanceSignature → f64 — an interaction the linear vocabulary can't express (e.g. density × clustering)." },
  { stage: "Test", desc: "Does it improve out-of-sample prediction? Leave-one-instance-out ridge RMSE, base vocabulary vs. base + candidate." },
  { stage: "Gate", desc: "Admit only if the OOD gain beats an Occam penalty — the discipline that killed the naive 11-feature descriptor." },
  { stage: "Admit", desc: "The survivor joins one shared Feature Registry vocabulary that every faculty reads; growth is bit-identical on replay." },
] as const;

/** Theory Engine pipeline (theory.rs). */
export const THEORY_PIPELINE = [
  { stage: "Rule", desc: "Every analyzer stops here: a correlation from StepEvent signatures (entropy, acceptance, diversity)." },
  { stage: "Mechanism", desc: "The rule is promoted to a causal MechanismHypothesis about why it holds." },
  { stage: "Prediction", desc: "The mechanism must make a falsifiable prediction the runtime can test." },
  { stage: "Ablation", desc: "An ablation on the runtime tries to break it — the refutation is recorded either way." },
  { stage: "Theory", desc: "What survives becomes a Theory: a mechanism an experiment failed to refute." },
] as const;
