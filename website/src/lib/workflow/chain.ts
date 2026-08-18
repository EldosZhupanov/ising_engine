/**
 * The scientific workflow chain.
 *
 * Problem → Hypothesis → Theory → Experiment → Evidence → Statistics →
 * Knowledge → Reasoning → Discovery → Publication → Applications
 *
 * Every workspace occupies a position in this chain. Rendering the chain on each
 * screen answers the navigational question no dashboard answers: *where am I in
 * the scientific process, and what comes next?*
 */

export type Link = {
  id: string;
  label: string;
  /** What happens at this link, in the researcher's terms. */
  act: string;
  route: string;
  /** Whether the platform can actually perform this step today. */
  live: "real" | "offline";
  /** What is discoverable here. */
  discover: string;
};

export const CHAIN: Link[] = [
  { id: "problem", label: "Problem", act: "Choose a structure worth attacking — an instance, a family, a regime.",
    route: "/experiments", live: "real",
    discover: "Which problem structures the engine has and has not explored." },
  { id: "hypothesis", label: "Hypothesis", act: "State a falsifiable claim and the kill-criterion that would end it.",
    route: "/design", live: "real",
    discover: "A testable claim about why an operator composition should work." },
  { id: "theory", label: "Theory", act: "Promote a claim to a causal mechanism that predicts something.",
    route: "/theories", live: "real",
    discover: "Mechanisms that survived ablation — and the ones that did not." },
  { id: "experiment", label: "Experiment", act: "Specify the exact run: instance, operators, budget, seed.",
    route: "/design", live: "real",
    discover: "A reproducible specification — the unit of scientific work." },
  { id: "evidence", label: "Evidence", act: "Read outcomes against baselines across the append-only record.",
    route: "/experiments", live: "real",
    discover: "Which compositions actually improve, and by how much." },
  { id: "statistics", label: "Statistics", act: "Ask whether the effect survives a real test, with real power.",
    route: "/benchmarks", live: "real",
    discover: "Whether an apparent win is an effect or an artifact." },
  { id: "knowledge", label: "Knowledge", act: "Fold the result into a conditional, weighted, attributed fact.",
    route: "/graph", live: "real",
    discover: "The conditions under which each operator helps or fails." },
  { id: "reasoning", label: "Reasoning", act: "Let a model read the evidence and propose — or attack — hypotheses.",
    route: "/llm", live: "real",
    discover: "Compositions no human enumerated, argued from real weights." },
  { id: "discovery", label: "Discovery", act: "Locate the gap the record itself points at.",
    route: "/discover", live: "real",
    discover: "New operators worth inventing, and weaknesses worth closing." },
  { id: "publication", label: "Publication", act: "Assemble the claim with its evidence, confidence and unknowns.",
    route: "/publish", live: "real",
    discover: "Whether the finding is communicable — and honest." },
  { id: "applications", label: "Applications", act: "Say exactly how far the result travels, and no further.",
    route: "/applications", live: "real",
    discover: "Which real-world problems the result can and cannot reach." },
];

/** Which chain links a route participates in (a route can serve several). */
export function linksForRoute(route: string): Link[] {
  return CHAIN.filter((l) => l.route === route);
}
export const linkById = (id: string) => CHAIN.find((l) => l.id === id);
