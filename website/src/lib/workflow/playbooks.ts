/**
 * Research playbooks — method, encoded.
 *
 * These are the procedures a careful computational scientist follows. They are
 * deliberately not tutorials: each step is a real check with a real place to do
 * it, and a stated failure mode it protects against. A playbook you can "pass"
 * without learning anything would be worthless, so every step names the mistake
 * it prevents.
 */

export type Step = {
  do: string;
  where?: string;
  /** The specific self-deception this step blocks. */
  guards: string;
};

export type Playbook = {
  id: string;
  title: string;
  question: string;
  /** When a researcher reaches for this. */
  use: string;
  minutes: number;
  level: "beginner" | "professional";
  steps: Step[];
  /** What you are entitled to conclude at the end — and what you are not. */
  conclusion: string;
  teaches: string[];
};

export const PLAYBOOKS: Playbook[] = [
  {
    id: "skeptic",
    title: "The skeptic's pass",
    question: "This claim looks strong — should I believe it?",
    use: "Before repeating, building on, or publishing any claim the platform makes.",
    minutes: 8, level: "beginner",
    teaches: ["support", "confidence", "reproducibility", "causal"],
    steps: [
      { do: "Find the claim as a typed fact and read its support count.", where: "/graph",
        guards: "A confident-sounding fact with support 1 is a single observation wearing a suit." },
      { do: "Read its confidence alongside support — never one without the other.", where: "/graph",
        guards: "High confidence on tiny support is the most common way statistics mislead." },
      { do: "Pivot to the runs behind it: filter the record to that operator and instance class.", where: "/experiments",
        guards: "Believing a summary without ever looking at a single underlying run." },
      { do: "Check whether the rule reproduced independently across instances.", where: "/evaluation",
        guards: "Mistaking an instance-specific coincidence for a general law." },
      { do: "Ask what would falsify it, and whether an ablation was actually run.", where: "/theories",
        guards: "Accepting a correlation dressed up as a mechanism." },
    ],
    conclusion: "You may now say: 'this holds with support N and confidence C, reproduced on K instances, and an ablation did / did not test it.' You may not say it is a general law.",
  },
  {
    id: "power",
    title: "Is that benchmark win real?",
    question: "The engine won 100% of comparisons — is that evidence?",
    use: "Any time you see a head-to-head result, internal or external.",
    minutes: 6, level: "beginner",
    teaches: ["statistical-power", "wilcoxon", "kill-criterion"],
    steps: [
      { do: "Read the number of instances before you read the win rate.", where: "/benchmarks",
        guards: "Anchoring on a 100% win rate that rests on 3 samples." },
      { do: "Read the Wilcoxon and sign-test p-values.", where: "/benchmarks",
        guards: "Treating a descriptive win fraction as an inferential result." },
      { do: "Find the pre-declared kill-criterion for that experiment.", where: "/benchmarks",
        guards: "Post-hoc goalpost-moving — the criterion must predate the data." },
      { do: "Check whether the comparison was at matched wall-clock.", where: "/benchmarks",
        guards: "Winning by spending more compute, which is not an algorithmic win." },
      { do: "Note which engine version and instance subset were used.", where: "/benchmarks",
        guards: "Generalising a narrow gate (n ≤ 2000) to all problem sizes." },
    ],
    conclusion: "A win with n = 3 and p = 0.25 is a pilot, not a finding. A win across 23 instances at matched wall-clock with p ≪ 0.01 is worth reporting — with its scope stated.",
  },
  {
    id: "gap",
    title: "Find your next experiment",
    question: "I want to discover something — where do I even start?",
    use: "When you have time to run research and need a lead that isn't arbitrary.",
    minutes: 12, level: "beginner",
    teaches: ["operator-gap", "meta-learning", "hypothesis", "kill-criterion"],
    steps: [
      { do: "Open the gaps the platform mined from its own record.", where: "/discover",
        guards: "Picking a topic because it sounds interesting rather than because the evidence points there." },
      { do: "Read the adjacency evidence behind an operator gap — how often the pair co-occurs in the best solutions.", where: "/discover",
        guards: "Chasing a machine suggestion without checking the observation that motivated it." },
      { do: "Look up how each operator behaves alone: ordering effects and budget scaling.", where: "/operators",
        guards: "Proposing a fusion of two operators whose individual behaviour you never checked." },
      { do: "Write the hypothesis as a causal claim, then write the kill-criterion first.", where: "/playbooks",
        guards: "An unfalsifiable proposal that no result could contradict." },
      { do: "Specify the exact run: instance, sequence, sweeps, temperatures, seed.", where: "/experiments",
        guards: "An experiment nobody — including you — can reproduce later." },
    ],
    conclusion: "You now hold a falsifiable, reproducible, evidence-motivated experiment specification. Running it needs the control plane; the specification is the scientific work.",
  },
  {
    id: "llm-discipline",
    title: "Use a model without fooling yourself",
    question: "How do I get value from an LLM in research without hallucinating results?",
    use: "Every time you use the reasoning console for ideation.",
    minutes: 10, level: "professional",
    teaches: ["ideation", "unverified", "adversarial-review"],
    steps: [
      { do: "Load a model and note its token rate and context limit before asking anything.", where: "/llm",
        guards: "Asking questions whose answers you cannot afford to wait for or fit in context." },
      { do: "Use the Ideator preset so the model reads the real knowledge graph, not your paraphrase.", where: "/llm",
        guards: "Feeding a model your own summary and mistaking its echo for independent support." },
      { do: "Check that its argument cites actual weights and support from the graph.", where: "/llm",
        guards: "Accepting fluent prose with invented numbers." },
      { do: "Re-run with the critique preset and let it attack the same evidence.", where: "/llm",
        guards: "One-sided generation: a model will argue either side, so make it argue against." },
      { do: "Treat every surviving proposal as an unverified hypothesis, and queue it for a run.", where: "/discover",
        guards: "The central failure mode — letting model output enter your beliefs without an experiment." },
    ],
    conclusion: "Model output is a hypothesis stream, useful in proportion to how cheaply you can refute it. Nothing from the console enters the knowledge graph.",
  },
  {
    id: "dataset",
    title: "Can this corpus train a model?",
    question: "We have 100k+ runs — is that enough to learn from?",
    use: "Before proposing any learned model, GNN or transformer on the record.",
    minutes: 7, level: "professional",
    teaches: ["foundation-scale", "overfitting", "out-of-sample", "occam"],
    steps: [
      { do: "Read the honest scale gap against the milestones.", where: "/dataset",
        guards: "Assuming 'lots of rows' means 'enough data' — it is 27× short of the first milestone." },
      { do: "Check coverage: how many distinct instances and operators the rows actually span.", where: "/dataset",
        guards: "Confusing volume with diversity; 100k runs on 23 instances is narrow." },
      { do: "Look at how the existing predictor is validated (leave-one-instance-out).", where: "/evaluation",
        guards: "Reporting in-sample fit, which is trivially improvable by adding parameters." },
      { do: "Read why the 11-feature descriptor was rejected despite fitting.", where: "/concepts",
        guards: "Adding features until the training error looks good." },
      { do: "Decide what data to grow before what model to build.", where: "/discover",
        guards: "Model-first research, the most reliable way to overfit a small corpus." },
    ],
    conclusion: "The defensible position: grow the corpus with informative runs (curiosity-directed) and keep models small and out-of-sample validated until the milestones are met.",
  },
  {
    id: "claim-scope",
    title: "How far does this result travel?",
    question: "We got a result on MaxCut — can we say it helps scheduling?",
    use: "Before writing any application, industry or impact claim.",
    minutes: 8, level: "professional",
    teaches: ["reduction", "transfer", "evidence-tier", "uncertainty"],
    steps: [
      { do: "Identify which problem families were actually run here.", where: "/applications",
        guards: "Silently upgrading 'reducible in principle' to 'demonstrated'." },
      { do: "Read the one measured transfer result and its exact scope.", where: "/applications",
        guards: "Citing 9/12 transfer as if it covered domains that were never tested." },
      { do: "For a target domain, check whether a published reduction exists and read it.", where: "/applications",
        guards: "Claiming applicability with no formulation to back it." },
      { do: "Write down what may NOT be concluded, explicitly.", where: "/applications",
        guards: "Omission by enthusiasm — the most common form of technical overclaim." },
      { do: "List the unknowns and open questions alongside the claim.", where: "/applications",
        guards: "Presenting a result without its uncertainty, which makes it a marketing claim." },
    ],
    conclusion: "You may claim exactly: demonstrated on the families run here; expressible for families with published reductions; unknown elsewhere. Anything more is unfounded.",
  },
];

export const playbookById = (id: string) => PLAYBOOKS.find((p) => p.id === id);
