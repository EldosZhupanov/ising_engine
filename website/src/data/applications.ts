/**
 * Application Engine — a reduction atlas, evidence-gated (architecture AD-4).
 *
 * HARD RULE: no domain appears without either (a) an experiment run on this
 * platform, or (b) a citation to a published reduction. Nothing is asserted from
 * plausibility. Tiers keep "we demonstrated this" rigorously separate from
 * "this is reducible in principle" and from "open question".
 *
 * T1 Demonstrated       — the engine has actually run this family here.
 * T2 Reduction-known    — a published QUBO/Ising formulation exists; UNTESTED here.
 *                         No performance claim is made or implied.
 * T3 Open question      — no reduction and no test; shown as a question only.
 */

export type Tier = "T1" | "T2" | "T3";

export type Citation = { ref: string; note?: string };

export type Application = {
  id: string;
  domain: string;
  problem: string;
  tier: Tier;
  /** What the reduction to QUBO/Ising looks like, in one honest sentence. */
  reduction: string;
  /** Only present for T1 — measured on this platform. */
  evidence?: string;
  /** What must NOT be concluded. Mandatory for T2/T3. */
  notClaimed?: string;
  unknowns: string[];
  citations?: Citation[];
  sectors: string[];
};

export const CITATIONS: Record<string, Citation> = {
  lucas: { ref: "A. Lucas, “Ising formulations of many NP problems,” Frontiers in Physics 2 (2014)", note: "The standard catalogue of Ising/QUBO reductions for NP problems." },
  glover: { ref: "F. Glover, G. Kochenberger, Y. Du, “A Tutorial on Formulating and Using QUBO Models” (arXiv, 2018)", note: "Practical construction of QUBO formulations." },
  koch: { ref: "G. Kochenberger et al., “The unconstrained binary quadratic programming problem: a survey,” J. Comb. Optim. (2014)", note: "Survey of UBQP/QUBO problem classes and applications." },
};

export const TIER_META: Record<Tier, { label: string; blurb: string; tone: "confirmed" | "open" | "neutral" }> = {
  T1: { label: "Demonstrated here", tone: "confirmed",
    blurb: "The engine has actually solved this problem family on this platform, and the result is in the append-only record." },
  T2: { label: "Reduction known — untested here", tone: "open",
    blurb: "A published reduction to QUBO/Ising exists, so the problem is expressible for this engine. It has NOT been run here; no performance claim is made." },
  T3: { label: "Open question", tone: "neutral",
    blurb: "No established reduction and no experiment. Listed as a research direction only — not a capability." },
};

export const APPLICATIONS: Application[] = [
  /* ───────────────── T1 · demonstrated on this platform ───────────────── */
  {
    id: "maxcut", domain: "Graph partitioning", problem: "Maximum Cut (G-Set)", tier: "T1",
    reduction: "MaxCut is Ising with zero field: maximise cut weight ≡ minimise the coupling energy of ±1 spins.",
    evidence: "The primary corpus: 18,570 recorded runs across 23 G-Set instances; best recorded −26,664 against a −26,508 baseline on G63. Externally compared to OpenJij/neal with a pre-declared kill-criterion.",
    unknowns: ["Performance against state-of-the-art specialised MaxCut heuristics at large n", "Behaviour beyond the tested instance sizes"],
    citations: [CITATIONS.lucas],
    sectors: ["Circuit layout", "Network design", "Image segmentation", "Statistical physics"],
  },
  {
    id: "max2sat", domain: "Satisfiability", problem: "MAX-2-SAT", tier: "T1",
    reduction: "Each 2-clause becomes a quadratic penalty over binary variables; maximising satisfied clauses ≡ minimising the QUBO.",
    evidence: "Runnable family in `families.rs` (`max2sat_qubo`), verified against brute force on small instances; included in the cross-family transfer test.",
    unknowns: ["Scaling to industrial SAT sizes", "Comparison with dedicated MaxSAT solvers"],
    citations: [CITATIONS.lucas, CITATIONS.glover],
    sectors: ["Verification", "Program analysis", "Scheduling constraints"],
  },
  {
    id: "tsp", domain: "Routing", problem: "Travelling Salesman (small)", tier: "T1",
    reduction: "A permutation matrix of city×position binaries, with penalties enforcing one-city-per-position; tour length becomes the quadratic objective.",
    evidence: "Runnable family in `families.rs` (`tsp_qubo` / `decode_tsp`), proven against brute force on small instances; part of the 9/12 cross-family transfer result (ρ +0.747).",
    unknowns: ["The permutation encoding costs O(n²) variables — practical only for small n", "No comparison against LKH or concorde-class solvers"],
    citations: [CITATIONS.lucas],
    sectors: ["Logistics", "Last-mile delivery", "Tool-path planning"],
  },

  /* ───────────────── T2 · reduction known, untested here ───────────────── */
  {
    id: "coloring", domain: "Graphs", problem: "Graph colouring", tier: "T2",
    reduction: "One binary per (vertex, colour) with penalties for adjacent same-colour pairs and for vertices lacking exactly one colour.",
    notClaimed: "Never run on this platform. Nothing about speed or solution quality may be inferred from the MaxCut results.",
    unknowns: ["Whether the learned operator strategies transfer to colouring landscapes", "Penalty weighting sensitivity"],
    citations: [CITATIONS.lucas], sectors: ["Register allocation", "Frequency assignment", "Timetabling"],
  },
  {
    id: "partition", domain: "Combinatorics", problem: "Number partitioning", tier: "T2",
    reduction: "Classic Ising form: minimise (Σ sᵢaᵢ)² over spins sᵢ = ±1.",
    notClaimed: "Untested here. This is among the cleanest reductions but that is a statement about expressibility, not performance.",
    unknowns: ["Known to be pathologically hard near the phase transition", "No measurement on this engine"],
    citations: [CITATIONS.lucas], sectors: ["Load balancing", "Fair division"],
  },
  {
    id: "clique", domain: "Graphs", problem: "Maximum clique / independent set", tier: "T2",
    reduction: "Binary per vertex; reward inclusion, penalise selecting both ends of a non-edge (or an edge, for independent set).",
    notClaimed: "No runs on this platform.",
    unknowns: ["Penalty scaling on dense graphs", "Transfer of operator ordering rules"],
    citations: [CITATIONS.lucas, CITATIONS.koch], sectors: ["Social network analysis", "Bioinformatics", "Fraud rings"],
  },
  {
    id: "knapsack", domain: "Operations research", problem: "Knapsack / set packing", tier: "T2",
    reduction: "Item binaries with a quadratic penalty encoding the capacity constraint (often via slack binaries).",
    notClaimed: "Untested here. Constraint-heavy problems typically need careful penalty tuning that this platform has not explored.",
    unknowns: ["Slack-variable overhead", "Whether penalty tuning dominates operator choice"],
    citations: [CITATIONS.glover, CITATIONS.koch], sectors: ["Capital budgeting", "Cargo loading", "Ad selection"],
  },
  {
    id: "jsp", domain: "Scheduling", problem: "Job-shop / task scheduling", tier: "T2",
    reduction: "Time-indexed binaries per (operation, start-time) with penalties for machine conflicts and precedence violations.",
    notClaimed: "No experiment on this platform. Scheduling is frequently cited as a QUBO application; that citation is about formulation, not about this engine's performance.",
    unknowns: ["Time discretisation blows up variable count", "Feasibility vs. optimality trade-off under penalties"],
    citations: [CITATIONS.lucas, CITATIONS.glover], sectors: ["Manufacturing", "Cloud job placement", "Operating theatres"],
  },
  {
    id: "vrp", domain: "Routing", problem: "Vehicle routing (CVRP)", tier: "T2",
    reduction: "A multi-vehicle extension of the TSP permutation encoding, with capacity penalties.",
    notClaimed: "Only single-tour small TSP has been run here. CVRP has not.",
    unknowns: ["Variable count at realistic fleet sizes", "Whether penalties preserve feasibility"],
    citations: [CITATIONS.lucas], sectors: ["Distribution", "Field service", "Waste collection"],
  },
  {
    id: "portfolio", domain: "Finance", problem: "Portfolio selection (cardinality-constrained)", tier: "T2",
    reduction: "Asset binaries with the covariance matrix as the quadratic term and a penalty for the cardinality/budget constraint.",
    notClaimed: "Untested here. No claim is made about returns, risk or any financial outcome — this is a statement about problem form only.",
    unknowns: ["Covariance estimation error usually dominates optimiser choice", "Discretising weights loses information"],
    citations: [CITATIONS.glover, CITATIONS.koch], sectors: ["Asset management", "Index tracking"],
  },
  {
    id: "folding", domain: "Computational biology", problem: "Lattice protein folding (HP model)", tier: "T2",
    reduction: "Residue-position binaries on a lattice with penalties for self-avoidance and rewards for hydrophobic contacts.",
    notClaimed: "Never run here. The HP lattice model is a coarse abstraction — it is not protein structure prediction, and nothing here bears on real folding accuracy.",
    unknowns: ["Lattice models omit most real biophysics", "Variable count grows quickly with chain length"],
    citations: [CITATIONS.lucas], sectors: ["Structural biology (models)", "Molecular design (models)"],
  },
  {
    id: "covering", domain: "Operations research", problem: "Set cover / facility location", tier: "T2",
    reduction: "Selection binaries with penalties ensuring every element is covered.",
    notClaimed: "Untested on this platform.",
    unknowns: ["Penalty weight vs. coverage feasibility", "Comparison with LP relaxation + rounding"],
    citations: [CITATIONS.lucas, CITATIONS.koch], sectors: ["Telecom coverage", "Warehouse siting", "Sensor placement"],
  },

  /* ───────────────── T3 · open questions ───────────────── */
  {
    id: "compiler", domain: "Compilers", problem: "Instruction scheduling / register allocation", tier: "T3",
    reduction: "Register allocation is graph colouring (T2); broader scheduling has no single accepted QUBO form.",
    notClaimed: "Speculative. No reduction is committed to here and no experiment exists.",
    unknowns: ["Whether a QUBO formulation competes with existing heuristics at compile-time budgets", "Latency requirements are milliseconds, not seconds"],
    sectors: ["Toolchains"],
  },
  {
    id: "rl", domain: "Machine learning", problem: "Discrete policy / architecture search", tier: "T3",
    reduction: "No established reduction; search spaces are usually not naturally quadratic.",
    notClaimed: "This is a research direction, not a capability. Any claim that this engine improves ML training would be unfounded.",
    unknowns: ["Whether combinatorial sub-problems in ML are large enough to matter", "Objective is usually stochastic, unlike QUBO"],
    sectors: ["AutoML"],
  },
  {
    id: "robotics", domain: "Robotics", problem: "Task allocation / motion primitives sequencing", tier: "T3",
    reduction: "Multi-robot task allocation resembles assignment problems (some have QUBO forms); continuous motion planning does not.",
    notClaimed: "No reduction committed, no experiment. Real-time control constraints are not addressed by this engine.",
    unknowns: ["Real-time budgets", "Continuous state spaces are outside the Ising formulation"],
    sectors: ["Warehouse robotics"],
  },
];

export const byTier = (t: Tier) => APPLICATIONS.filter((a) => a.tier === t);
export const applicationById = (id: string) => APPLICATIONS.find((a) => a.id === id);

/** The only cross-family transfer number the platform has actually measured. */
export const TRANSFER_EVIDENCE = {
  claim: "A MaxCut-trained policy ranked operators on non-MaxCut instances",
  result: "9 of 12 instances, Spearman ρ +0.747",
  source: "ROADMAP · portability (bin/portability.rs)",
  caveat: "This is evidence that *operator-selection strategy* transfers between families that were tested — MaxCut, MAX-2-SAT and TSP. It says nothing about untested domains.",
};
