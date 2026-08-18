/**
 * Research gaps — "what should I study next?"
 *
 * Architecture AD-5: the ENGINE answers this question, not the interface. Every
 * item below is derived from a real ingested artifact and carries its source.
 * The UI must never invent a research suggestion.
 *
 * Currently derivable from artifacts on disk:
 *   • operator gaps          ← operator_proposal_*.md (mined by meta_learner)
 *   • weak / thin evidence   ← knowledge_graph.txt support counts
 *   • unreproduced rules     ← evaluation_report.md (instance counts, confidence)
 *   • underpowered benchmarks← results/index.json (n, p-values)
 *   • dataset scale gap      ← foundation_manifest.md
 *
 * Still in-memory only inside the engine (needs an export seam before it can
 * appear here honestly): Curiosity's disagreement / coverage / anomaly signals
 * and the Planner's ranked agenda.
 */

import { graph, evaluation, benchmarks, dataset, proposals, experiments } from "@/data";

export type GapKind = "operator" | "evidence" | "reproduction" | "power" | "data";

export type Gap = {
  kind: GapKind;
  title: string;
  /** The observation that motivates it — real numbers only. */
  evidence: string;
  /** The concrete next action a scientist would take. */
  action: string;
  route: string;
  /** Which artifact this came from. */
  source: string;
  /** Rough leverage: how much knowledge closing it would add. */
  weight: number;
};

export const KIND_META: Record<GapKind, { label: string; blurb: string }> = {
  operator: { label: "Operator gap", blurb: "The meta-learner noticed two operators that keep appearing adjacent in the best solutions and proposed fusing them." },
  evidence: { label: "Thin evidence", blurb: "A belief the platform holds on very few observations — cheap to strengthen or overturn." },
  reproduction: { label: "Unreproduced rule", blurb: "A mined rule that has not recurred independently on enough instances to be trusted." },
  power: { label: "Underpowered comparison", blurb: "A benchmark whose sample size cannot support the conclusion it appears to offer." },
  data: { label: "Data deficit", blurb: "The corpus is too small or too narrow for what someone might want to train on it." },
};

export function computeGaps(): Gap[] {
  const out: Gap[] = [];

  // 1 ── operator gaps: real mined proposals (highest leverage: a new algorithm)
  for (const p of proposals ?? []) {
    out.push({
      kind: "operator",
      title: `Fuse into \`${p.name}\``,
      evidence: p.provenance || "mined from operator adjacency in top solutions",
      action: "Read the proposal's mathematics and pseudocode, then specify a falsifiable experiment for it.",
      route: "/discover", source: p.file, weight: 100,
    });
  }

  // 2 ── thin evidence: facts standing on very little support
  const thin = (graph.facts ?? [])
    .filter((f) => f.support > 0 && f.support <= 3)
    .sort((a, b) => a.support - b.support)
    .slice(0, 4);
  for (const f of thin) {
    out.push({
      kind: "evidence",
      title: `\`${f.subject}\` ${f.relation} ${f.object || "?"} rests on support ${f.support}`,
      evidence: `weight ${f.weight >= 0 ? "+" : ""}${f.weight.toFixed(1)}, support ${f.support}${f.confidence !== null ? `, confidence ${f.confidence.toFixed(2)}` : ""} — one more campaign could confirm or overturn it.`,
      action: "Run more trials on this operator/structure pair, or treat the fact as provisional.",
      route: "/graph", source: "knowledge_graph.txt", weight: 60 - f.support,
    });
  }

  // 3 ── rules that barely reproduced across instances
  const rows = evaluation?.rows ?? [];
  if (rows.length) {
    const maxInst = Math.max(...rows.map((r) => r.instances));
    for (const r of rows.filter((r) => r.confidence < 0.65 || r.instances < Math.max(2, maxInst * 0.5)).slice(0, 4)) {
      out.push({
        kind: "reproduction",
        title: `\`${r.signature}\` is weakly reproduced`,
        evidence: `emerged on ${r.instances} instance(s) at mean confidence ${r.confidence.toFixed(2)} — below the level at which a rule should be trusted.`,
        action: "Extend to more instances, or record it as instance-specific rather than general.",
        route: "/evaluation", source: "evaluation_report.md", weight: 50 + (1 - r.confidence) * 20,
      });
    }
  }

  // 4 ── underpowered benchmark comparisons.
  // The index can hold several recorded runs of the same experiment; the same
  // weakness would then appear repeatedly, so key on (exp, engine, opponent).
  const seenH2H = new Set<string>();
  for (const run of benchmarks?.index?.runs ?? []) {
    for (const [engine, v] of Object.entries(run.per_engine)) {
      const h = v.head_to_head;
      if (!h) continue;
      const key = `${run.exp}|${engine}|${h.vs}`;
      if (seenH2H.has(key)) continue;
      seenH2H.add(key);
      if (h.instances < 6 || h.wilcoxon_p > 0.05) {
        out.push({
          kind: "power",
          title: `${run.exp}: ${engine} vs ${h.vs} is underpowered`,
          evidence: `win fraction ${(h.win_fraction * 100).toFixed(0)}% but only ${h.instances} instance(s), Wilcoxon p = ${h.wilcoxon_p} — not significant.`,
          action: "Re-run across more instances at matched wall-clock before drawing any conclusion.",
          route: "/benchmarks", source: "results/index.json", weight: 70,
        });
      }
    }
  }

  // 5 ── dataset scale deficit
  const next = (dataset?.milestones ?? []).find((m) => !m.reached);
  if (next) {
    out.push({
      kind: "data",
      title: `Corpus is ${next.multiplier}× short of ${next.milestone.toLocaleString()} examples`,
      evidence: `${(dataset.examples ?? 0).toLocaleString()} examples across ${dataset.instances ?? "?"} instances and ${dataset.operators ?? "?"} operators — a training corpus, not a foundation-model corpus.`,
      action: "Grow informative runs (curiosity-directed) and widen instance coverage before training anything large.",
      route: "/dataset", source: "foundation_manifest.md", weight: 55,
    });
  }

  // 6 ── instance coverage skew (thinly-sampled instances)
  const inst = experiments?.rollups?.byInstance ?? [];
  if (inst.length > 3) {
    const thinnest = inst[inst.length - 1];
    const richest = inst[0];
    if (thinnest && richest && richest.runs > thinnest.runs * 4) {
      out.push({
        kind: "data",
        title: `Instance coverage is skewed (\`${thinnest.key}\` has ${thinnest.runs} runs vs \`${richest.key}\` ${richest.runs})`,
        evidence: `A ${(richest.runs / Math.max(1, thinnest.runs)).toFixed(1)}× imbalance means aggregate conclusions are dominated by a few instances.`,
        action: "Balance sampling across instances before claiming any cross-instance generality.",
        route: "/experiments", source: "ai_experiments.txt", weight: 45,
      });
    }
  }

  return out.sort((a, b) => b.weight - a.weight);
}
