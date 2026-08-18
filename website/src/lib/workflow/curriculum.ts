/**
 * The research curriculum — the real path from "I opened the platform" to
 * "I discovered a new algorithm."
 *
 * This is not a UI tour. Each mission is a step a computational scientist
 * actually takes, is completed by *doing the thing* (not by clicking "next"),
 * and is honest about whether the platform can currently do it:
 *   real    — completable today against recorded evidence or a live LLM
 *   offline — the step is genuine science but needs the control-plane API
 *
 * Every stage carries WHY it matters, because a method without its rationale is
 * a ritual.
 */

import type { Status } from "@/lib/nav";

/** How a mission is proven complete. No quizzes — you either did it or didn't. */
export type Verify =
  | { kind: "visited"; route: string }
  | { kind: "action"; event: ActionEvent }
  | { kind: "manual" };

/** Real user actions the workspaces emit. Keep this list closed and meaningful. */
export type ActionEvent =
  | "graph.fact.inspected"
  | "graph.relation.filtered"
  | "experiments.filtered"
  | "experiments.recipe.copied"
  | "experiments.replayed"
  | "llm.model.loaded"
  | "llm.reasoned"
  | "llm.critique"
  | "papers.read"
  | "dataset.gap.seen"
  | "benchmark.power.seen"
  | "gap.opened"
  | "application.tier.opened"
  | "playbook.completed";

export type Mission = {
  id: string;
  stage: number;
  title: string;
  /** The scientific act, in one imperative sentence. */
  task: string;
  /** Why a scientist does this — the reasoning, not the button. */
  why: string;
  route: string;
  verify: Verify;
  status: Status;
  /** Concepts this step teaches (glossary keys). */
  teaches?: string[];
  /** What this step cannot yet do, stated plainly. */
  blocked?: string;
};

export type Stage = {
  n: number;
  title: string;
  arc: string;
  /** The scientific competence gained. */
  outcome: string;
};

export const STAGES: Stage[] = [
  { n: 0, title: "Orient", arc: "What is this platform actually claiming to do?",
    outcome: "You can state what counts as a discovery here, and what would falsify one." },
  { n: 1, title: "Read the record", arc: "Evidence before opinion.",
    outcome: "You can navigate 109,758 recorded runs and read an outcome against its baseline." },
  { n: 2, title: "Interrogate knowledge", arc: "A claim is only as good as its support.",
    outcome: "You can trace any belief to the runs that justify it — and find what was refuted." },
  { n: 3, title: "Judge rigour", arc: "Most published wins are underpowered.",
    outcome: "You can tell a real result from a statistical artifact, and say why." },
  { n: 4, title: "Reason with a model", arc: "The LLM proposes; evidence decides.",
    outcome: "You can use a language model as an ideation instrument without being fooled by it." },
  { n: 5, title: "Find the gap", arc: "Discovery starts where the map ends.",
    outcome: "You can locate a genuine gap the platform itself has identified." },
  { n: 6, title: "Design the experiment", arc: "A hypothesis that cannot fail is not science.",
    outcome: "You can specify a falsifiable, reproducible experiment down to the seed." },
  { n: 7, title: "Run and refute", arc: "Try to kill your own idea first.",
    outcome: "You can run an ablation and accept its verdict." },
  { n: 8, title: "Publish honestly", arc: "State the uncertainty, or it isn't a result.",
    outcome: "You can assemble a dossier with evidence, confidence, applications and unknowns." },
];

export const MISSIONS: Mission[] = [
  // ── Stage 0 · Orient
  { id: "orient.telemetry", stage: 0, title: "Read the instrument panel", route: "/",
    task: "Open Mission Control and find how many experiments the platform has recorded, and what its best result is.",
    why: "Every claim downstream rests on this record. Knowing its size tells you how much statistical weight any single result can bear.",
    verify: { kind: "visited", route: "/" }, status: "real", teaches: ["append-only", "provenance"] },
  { id: "orient.refuted", stage: 0, title: "Find something the platform admits it got wrong", route: "/",
    task: "In the Confirmed vs. refuted ledger, read a refuted claim and its reason.",
    why: "A platform that only reports wins is marketing. The refutations are what make the confirmations credible — this is the single fastest way to judge whether a research system is honest.",
    verify: { kind: "visited", route: "/" }, status: "real", teaches: ["falsification", "refutation"] },

  // ── Stage 1 · Read the record
  { id: "record.filter", stage: 1, title: "Isolate one operator's behaviour", route: "/experiments",
    task: "Filter the experiment record to a single operator and read its mean improvement.",
    why: "Aggregate numbers hide mechanism. Conditioning on one operator is the first move in separating 'what happened' from 'what caused it'.",
    verify: { kind: "action", event: "experiments.filtered" }, status: "real", teaches: ["operator", "baseline", "improvement"] },
  { id: "record.recipe", stage: 1, title: "Take a reproduce recipe", route: "/experiments",
    task: "Open a run and copy its exact reproduction command, seed included.",
    why: "Determinism is this platform's core guarantee: the same seed replays the same trajectory bit-for-bit. A result you cannot replay is an anecdote.",
    verify: { kind: "action", event: "experiments.recipe.copied" }, status: "real", teaches: ["determinism", "seed", "bit-identical"] },

  { id: "record.replay", stage: 1, title: "Verify a record instead of trusting it", route: "/experiments",
    task: "Replay a recorded run from its seed and confirm the score comes back bit-identical.",
    why: "This is the difference between a log and a corpus. Determinism (ADR-0004) means the engine must reproduce any recorded score exactly — so you never have to take a number on faith. A mismatch would be a genuine finding about the platform itself.",
    verify: { kind: "action", event: "experiments.replayed" }, status: "real",
    teaches: ["determinism", "bit-identical", "seed", "provenance"] },

  // ── Stage 2 · Interrogate knowledge
  { id: "know.fact", stage: 2, title: "Inspect a fact's evidence", route: "/graph",
    task: "Click an edge in the knowledge graph and read its support count and confidence.",
    why: "Facts here are conditional and weighted, never bare assertions. Support tells you how many observations back a belief; confidence tells you how consistent they were.",
    verify: { kind: "action", event: "graph.fact.inspected" }, status: "real", teaches: ["typed-triple", "support", "confidence", "welford"] },
  { id: "know.failure", stage: 2, title: "Study a failure relation", route: "/graph",
    task: "Filter the graph to `fails-on` and read what the platform learned by failing.",
    why: "Negative knowledge prevents wasted compute: the loop will not re-explore a dead end it has recorded. Failures are assets.",
    verify: { kind: "action", event: "graph.relation.filtered" }, status: "real", teaches: ["negative-result"] },

  // ── Stage 3 · Judge rigour
  { id: "rigour.power", stage: 3, title: "Catch an underpowered benchmark", route: "/benchmarks",
    task: "Find a head-to-head with a 100% win rate that is still not strong evidence, and explain why.",
    why: "3 instances at p = 0.25 is not a result, however good the win rate looks. Learning to see sample size before effect size is the most transferable skill in empirical computing.",
    verify: { kind: "action", event: "benchmark.power.seen" }, status: "real", teaches: ["statistical-power", "wilcoxon", "kill-criterion"] },
  { id: "rigour.reproduce", stage: 3, title: "Check whether a rule reproduces", route: "/evaluation",
    task: "Find a rule signature that emerged independently across many instances — and one that barely reproduced.",
    why: "A pattern found on one instance is a coincidence until it recurs. Cross-instance reproduction is the platform's internal replication test.",
    verify: { kind: "visited", route: "/evaluation" }, status: "real", teaches: ["reproducibility", "signature"] },
  { id: "rigour.scale", stage: 3, title: "Confront the dataset gap", route: "/dataset",
    task: "Read how far the corpus is from foundation-model scale.",
    why: "18,570 examples is a training corpus, not a foundation model — 27× short of the first milestone. Knowing the limit of your data prevents the most common failure in ML research: over-modelling.",
    verify: { kind: "action", event: "dataset.gap.seen" }, status: "real", teaches: ["overfitting", "foundation-scale"] },

  // ── Stage 4 · Reason with a model
  { id: "llm.load", stage: 4, title: "Bring a model online", route: "/llm",
    task: "Load a local model and watch its VRAM, context and token rate.",
    why: "A reasoning instrument whose cost you cannot see will be misused. Knowing tokens/sec and context limits tells you what questions are affordable.",
    verify: { kind: "action", event: "llm.model.loaded" }, status: "real", teaches: ["local-inference", "context-window", "vram"] },
  { id: "llm.reason", stage: 4, title: "Have the model reason over real evidence", route: "/llm",
    task: "Run the Ideator prompt — the model reads the platform's actual knowledge graph and proposes operator sequences with causal arguments.",
    why: "This is the engine's own ideation tier. The value is not the text; it is a cheap, diverse hypothesis stream that the deterministic runtime can then test.",
    verify: { kind: "action", event: "llm.reasoned" }, status: "real", teaches: ["ideation", "hypothesis"] },
  { id: "llm.skeptic", stage: 4, title: "Turn the model against the evidence", route: "/llm",
    task: "Run the critique task and make the model attack the weakest claims in the graph.",
    why: "An LLM is more trustworthy as a skeptic than as an oracle: refutation is checkable, generation is not. Anything it proposes is unverified until the runtime records it.",
    verify: { kind: "action", event: "llm.critique" }, status: "real", teaches: ["adversarial-review", "unverified"] },

  // ── Stage 5 · Find the gap
  { id: "gap.operator", stage: 5, title: "Open a real operator gap", route: "/discover",
    task: "Read a gap the platform mined from its own record — including the adjacency evidence that motivated it.",
    why: "This is where discovery actually begins. The meta-learner noticed two operators that keep appearing adjacent in the best solutions and proposed fusing them; that observation is the seed of a new algorithm.",
    verify: { kind: "action", event: "gap.opened" }, status: "real", teaches: ["operator-gap", "meta-learning"] },
  { id: "gap.application", stage: 5, title: "Map a result to a domain — carefully", route: "/applications",
    task: "Open a demonstrated (T1) application and a reduction-known (T2) one, and note the difference in what may be claimed.",
    why: "A QUBO result transfers only as far as its reduction and its evidence. Confusing 'reducible in principle' with 'demonstrated here' is how research gets oversold.",
    verify: { kind: "action", event: "application.tier.opened" }, status: "real", teaches: ["reduction", "transfer", "evidence-tier"] },

  // ── Stage 6 · Design the experiment
  { id: "design.playbook", stage: 6, title: "Work a research playbook end to end", route: "/playbooks",
    task: "Complete a playbook — a real multi-step procedure, not a tutorial.",
    why: "Method is what separates a scientist from a person running code. A playbook encodes the order of operations that keeps you from fooling yourself.",
    verify: { kind: "action", event: "playbook.completed" }, status: "real", teaches: ["method", "protocol"] },
  { id: "design.spec", stage: 6, title: "Specify a falsifiable experiment", route: "/design",
    task: "State a claim with its kill-criterion, compose an operator sequence, and copy the verified reproduce command.",
    why: "Without a kill-criterion decided in advance, any outcome can be rationalised into support. Pre-registration is the cheapest defence against self-deception — and the spec, not the run, is the scientific unit.",
    verify: { kind: "action", event: "experiments.recipe.copied" }, status: "real", teaches: ["kill-criterion", "pre-registration", "determinism"] },

  // ── Stage 7 · Run and refute  (needs the control plane)
  { id: "run.campaign", stage: 7, title: "Launch a campaign", route: "/campaigns",
    task: "Start an autonomous campaign with a budget cap, step one tick, then let it run.",
    why: "The loop only produces knowledge when it runs. Budget caps and health gates are what make an unattended scientist safe to leave alone — and stepping one tick shows you exactly what a cycle does.",
    verify: { kind: "visited", route: "/campaigns" }, status: "real",
    teaches: ["orchestrator", "budget", "health-gate"] },
  { id: "run.ablation", stage: 7, title: "Try to refute your own mechanism", route: "/theories",
    task: "Run an ablation that removes the mechanism and see whether performance collapses.",
    why: "A mechanism that survives deletion was never load-bearing. This is the single test that separates a theory from a correlation.",
    verify: { kind: "manual" }, status: "offline",
    blocked: "Requires the control-plane API to invoke research_platform --investigate.",
    teaches: ["ablation", "causal", "theory"] },

  // ── Stage 8 · Publish honestly
  { id: "publish.dossier", stage: 8, title: "Assemble a discovery dossier", route: "/papers",
    task: "Read how the platform writes its own findings: claim, evidence, support, and what remains unknown.",
    why: "A discovery that cannot be communicated with its uncertainty is not yet a contribution. The report archive is append-only, so the record of what you believed and when is permanent.",
    verify: { kind: "action", event: "papers.read" }, status: "real", teaches: ["provenance", "uncertainty"] },
  { id: "publish.compose", stage: 8, title: "Compose a discovery dossier", route: "/publish",
    task: "Assemble a claim with its supporting facts, its statistics (with the power caveat), its applications by tier — and its unknowns.",
    why: "This is the final discipline: a finding is only a contribution when someone else can audit it. Every figure must be transcluded from the record, and the unknowns are not optional.",
    verify: { kind: "visited", route: "/publish" }, status: "real", teaches: ["uncertainty", "provenance", "evidence-tier"] },
];

/** Per-workspace guidance: the two questions every screen must answer. */
export type Guide = {
  route: string;
  /** Why this workspace exists, scientifically. */
  matters: string;
  /** Concrete next moves — ordered. */
  next: string[];
  teaches?: string[];
};

export const GUIDES: Guide[] = [
  { route: "/", matters: "Mission Control is the one-glance verdict: how much evidence exists, what the loop last learned, and — decisively — what it has refuted.",
    next: ["Read the best recorded result and its baseline", "Open the Confirmed vs. refuted ledger and find a refutation", "Follow a discovery into the Knowledge Graph"],
    teaches: ["append-only", "falsification"] },
  { route: "/experiments", matters: "This is the raw record — every run the platform has ever done, with the seed that reproduces it. Aggregates persuade; individual runs let you check.",
    next: ["Filter to one operator and read its mean improvement", "Sort by improvement to see the extremes", "Open a run and copy its reproduce recipe"],
    teaches: ["baseline", "improvement", "determinism"] },
  { route: "/graph", matters: "Knowledge here is a graph of conditional, source-attributed facts — each carrying how much evidence backs it. This is what the platform believes, and why.",
    next: ["Click an edge to read its support and confidence", "Filter to fails-on to study negative knowledge", "Pivot from a fact to the runs behind it"],
    teaches: ["typed-triple", "support", "confidence"] },
  { route: "/benchmarks", matters: "External comparison is the only honest arbiter of speed — but a win on 3 instances proves little. Both the result and its statistical power are shown.",
    next: ["Read a head-to-head win fraction", "Check the sample size and p-value before believing it", "Read the pre-declared kill-criterion"],
    teaches: ["statistical-power", "kill-criterion"] },
  { route: "/evaluation", matters: "The platform grades itself: do the rules it mined on one instance reappear independently on others? That is internal replication.",
    next: ["Find a signature reproduced across all instances", "Find one that barely reproduced and ask why", "Note the predictor's leave-one-out accuracy"],
    teaches: ["reproducibility", "signature"] },
  { route: "/dataset", matters: "The corpus is the substrate for any learned model. Its honest size — and distance from foundation scale — decides what may be trained on it.",
    next: ["Read the scale gap to the first milestone", "Note the feature schema: structure → algorithm → outcome", "Decide what data is worth growing"],
    teaches: ["foundation-scale", "overfitting"] },
  { route: "/llm", matters: "The reasoning subsystem. A language model is an ideation instrument here — cheap, diverse hypotheses that the deterministic runtime then tries to refute.",
    next: ["Load a model and watch VRAM and token rate", "Run the Ideator over the real knowledge graph", "Run the critique task and let it attack the weakest claims"],
    teaches: ["ideation", "unverified", "context-window"] },
  { route: "/theories", matters: "A theory is a mechanism that survived an attempt to break it. This workspace shows the pipeline from correlation to causal claim — and the refutations kept along the way.",
    next: ["Follow the rule → mechanism → prediction → ablation chain", "Read a surviving mechanism and its ablation cost", "Read a refuted one and why it was kept"],
    teaches: ["ablation", "causal", "falsification"] },
  { route: "/concepts", matters: "The platform can extend its own vocabulary. A concept is admitted only if it improves out-of-sample prediction past an Occam penalty.",
    next: ["Follow propose → test → gate → admit", "Read why the 11-feature descriptor was rejected", "Note that in-sample fit is never sufficient"],
    teaches: ["occam", "out-of-sample", "overfitting"] },
  { route: "/papers", matters: "The platform's cumulative written memory — numbered, never rewritten. This is how a machine keeps a lab notebook.",
    next: ["Read the newest report's discoveries with their support", "Compare against an earlier report to see what changed", "Note that reports are append-only"],
    teaches: ["provenance", "uncertainty"] },
  { route: "/discover", matters: "Where discovery starts: gaps the platform found in its own knowledge — unexplored operators, weak rules, thin data, underpowered comparisons.",
    next: ["Open an operator gap and read its adjacency evidence", "Check which weaknesses are statistical vs. structural", "Pick one gap and specify an experiment for it"],
    teaches: ["operator-gap", "meta-learning"] },
  { route: "/applications", matters: "A QUBO result travels exactly as far as its reduction and its evidence. This atlas separates what was demonstrated here from what is merely reducible in principle.",
    next: ["Open a T1 demonstrated domain and read its evidence", "Open a T2 reduction-known domain and note what may NOT be claimed", "Read the open questions"],
    teaches: ["reduction", "transfer", "evidence-tier"] },
  { route: "/design", matters: "The unit of scientific work is not a run — it is a specification: a falsifiable claim, a kill-criterion fixed in advance, and an experiment precise enough for someone else to repeat.",
    next: ["State a claim an outcome could contradict", "Write the kill-criterion BEFORE choosing parameters", "Compose an operator sequence and set at least 5 seeds", "Copy the spec and the verified command"],
    teaches: ["kill-criterion", "pre-registration", "hypothesis", "determinism"] },
  { route: "/publish", matters: "A finding that cannot be communicated with its uncertainty is not a contribution. This composer assembles evidence, statistics, applications and unknowns into one auditable document.",
    next: ["Pick a claim that is already in the record", "Attach the facts that actually support it", "Attach the comparison — the power caveat is added for you", "State the unknowns; the dossier is incomplete without them"],
    teaches: ["uncertainty", "provenance", "statistical-power", "evidence-tier"] },
  { route: "/campaigns", matters: "This is where the platform stops being a record and becomes a running scientist: the loop generates hypotheses, tests them, refutes them and writes down what it learned — unattended, under a budget.",
    next: ["Pick instances and a tick budget", "Start paused, then step one tick and read what it did", "Resume and watch the tick log accumulate", "Stop when the budget or your patience runs out"],
    teaches: ["orchestrator", "budget", "health-gate", "falsification"] },
  { route: "/operators", matters: "The operators ARE the algorithm space. Everything the platform discovers is a statement about which of these to use, in what order, at what budget.",
    next: ["Find an operator flagged as an antipattern and read why", "Compare an ordering rule against the operator's solo behaviour", "Pick a pair with a strong adjacency and design a fusion"],
    teaches: ["operator", "signature", "meta-learning", "reproducibility"] },
  { route: "/models", matters: "A result belongs to a specific model version, not to \"the model\". Append-only lineage is what makes that attribution possible months later.",
    next: ["Read the newest version of each kind and its training corpus size", "Follow a parent chain back through the lineage", "Note that weights stay on disk — only metadata is shipped"],
    teaches: ["provenance", "out-of-sample", "overfitting"] },
  { route: "/runtime", matters: "One deterministic substrate computes everything. Its read-only discipline is precisely why any recorded result can be replayed and trusted.",
    next: ["Compare which backends the record actually used", "Read the determinism firewall", "Replay a row from Experiments to see it hold"],
    teaches: ["determinism", "bit-identical", "seed"] },
  { route: "/features", matters: "Every learned model reads one shared, versioned vocabulary. Without that, a change in one model silently diverges from the others.",
    next: ["Read the five structural scalars every model sees", "Follow how a concept must earn admission", "Note what is still in-memory only"],
    teaches: ["occam", "out-of-sample", "overfitting"] },
  { route: "/docs", matters: "Direction is written down and ranked so that no single exciting result can quietly redefine what the project is for.",
    next: ["Read the governance order top to bottom", "Open ADR-0004 — it is why Replay exists", "Note the release-only verification gates"],
    teaches: ["method", "protocol", "provenance"] },
  { route: "/solve", matters: "The production solver is the one component with no research caveats: it is the shipped path, and you can hand-check its answers on tiny instances.",
    next: ["Solve the frustrated triangle and reason about the result", "Pin a seed and confirm the answer repeats", "Note that a stochastic solver may vary without one"],
    teaches: ["qubo", "ising", "determinism"] },
  { route: "/settings", matters: "Everything configurable here is local to your browser — appearance, mode, control-plane address and your own mission history.",
    next: ["Try the print theme before exporting a dossier", "Switch to Professional once the guidance feels redundant", "Point the control-plane address at your own host if needed"],
    teaches: [] },
  { route: "/memory", matters: "Recall is what stops the platform re-learning the same thing forever. Every tick begins by asking what the record already knows about a structure like this one — and the answer is computed live, not cached.",
    next: ["Recall a familiar instance and read the narrative", "Widen the radius and watch specificity drop", "Compare 'what worked there' against the operator's global profile", "Read which regimes are well-characterised enough to summarise"],
    teaches: ["append-only", "provenance", "signature", "meta-learning"] },
  { route: "/playbooks", matters: "Method, encoded. A playbook is the order of operations that keeps a researcher from fooling themselves.",
    next: ["Work the skeptic's playbook on a claim you doubt", "Use the benchmark-power playbook before believing a win", "Use the gap playbook to design your next experiment"],
    teaches: ["method", "protocol"] },
];

export const guideFor = (route: string) => GUIDES.find((g) => g.route === route);
export const missionsForStage = (n: number) => MISSIONS.filter((m) => m.stage === n);
export const missionsForRoute = (route: string) => MISSIONS.filter((m) => m.route === route);
