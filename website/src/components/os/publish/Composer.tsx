"use client";

/**
 * Publication Composer (architecture AD-7).
 *
 * Assembles a discovery dossier from artifacts that already exist — nothing is
 * generated. Every number is transcluded from the ingested record and labelled
 * with its source file. Uncertainty and unknowns are REQUIRED fields: a claim
 * without them is not a finding.
 *
 * Exports Markdown (and BibTeX); PDF comes from the browser's print path, which
 * uses the dedicated ink-on-paper theme.
 */

import { useMemo, useState } from "react";
import Link from "next/link";
import type { Fact } from "@/data";
import { TIER_META, type Application } from "@/data/applications";
import { Icon, Panel, PanelHeader, Pill, StatusBadge } from "@/components/os/kit";

export type Claim = {
  id: string;
  statement: string;
  status: "confirmed" | "refuted" | "open";
  detail: string;
  source: string;
};

export type Ev = {
  runsAll: number; runsPrimary: number; instances: number; operators: number;
  best: { score: number; baseline: number; instance: string; seq: string[]; seed: string } | null;
  primary: string;
};

export type Bench = { exp: string; engine: string; vs: string; instances: number; win: number; p: number };

export default function Composer({
  claims, facts, ev, benches, applications, reproSources,
}: {
  claims: Claim[]; facts: Fact[]; ev: Ev; benches: Bench[];
  applications: Application[]; reproSources: string[];
}) {
  const [claimId, setClaimId] = useState(claims[0]?.id ?? "");
  const [factKeys, setFactKeys] = useState<number[]>([0]);
  const [benchIdx, setBenchIdx] = useState<number | null>(benches.length ? 0 : null);
  const [appIds, setAppIds] = useState<string[]>(applications.filter((a) => a.tier === "T1").slice(0, 1).map((a) => a.id));
  const [unknowns, setUnknowns] = useState("");
  const [copied, setCopied] = useState(false);

  const claim = claims.find((c) => c.id === claimId) ?? claims[0];
  const chosenFacts = factKeys.map((i) => facts[i]).filter(Boolean);
  const bench = benchIdx !== null ? benches[benchIdx] : null;
  const chosenApps = applications.filter((a) => appIds.includes(a.id));
  const ready = !!claim && chosenFacts.length > 0 && unknowns.trim().length > 15;

  const md = useMemo(() => {
    if (!claim) return "";
    const L: string[] = [];
    L.push(`# ${claim.statement}`, "");
    L.push(`**Status:** ${claim.status.toUpperCase()}  |  **Source:** ${claim.source}`, "");
    L.push(claim.detail, "");
    L.push("## Evidence", "");
    L.push(`Drawn from an append-only record of **${ev.runsAll.toLocaleString()} experiments** across ${ev.instances} instances and ${ev.operators} operators (primary campaign \`${ev.primary}\`: ${ev.runsPrimary.toLocaleString()} runs).`, "");
    if (chosenFacts.length) {
      L.push("| subject | relation | object | weight | support | confidence |", "|---|---|---|---|---|---|");
      for (const f of chosenFacts) {
        L.push(`| \`${f.subject}\` | ${f.relation} | ${f.object || "—"} | ${f.weight >= 0 ? "+" : ""}${f.weight.toFixed(1)} | ${f.support} | ${f.confidence?.toFixed(2) ?? "—"} |`);
      }
      L.push("", "_Facts are conditional, evidence-weighted triples from `knowledge_graph.txt`; confidence accrues by Welford statistics over repeated observation._", "");
    }
    if (bench) {
      L.push("## Statistics", "");
      L.push(`${bench.engine} vs ${bench.vs} (${bench.exp}): win fraction **${(bench.win * 100).toFixed(0)}%** over **${bench.instances}** instance(s), Wilcoxon p = ${bench.p}.`, "");
      const weak = bench.instances < 6 || bench.p > 0.05;
      L.push(weak
        ? `> **Power caveat.** With n = ${bench.instances} and p = ${bench.p} this comparison is underpowered. It is reported as a pilot, not as a finding.`
        : `> Sample size and p-value support the comparison as stated. Scope is limited to the instances tested.`, "");
    }
    if (ev.best) {
      L.push("## Reproduction", "");
      L.push(`Best recorded outcome: **${ev.best.score.toLocaleString()}** against a baseline of ${ev.best.baseline.toLocaleString()} on \`${ev.best.instance}\`, from the sequence \`[${ev.best.seq.join(" → ")}]\` at seed \`${ev.best.seed}\`.`, "");
      L.push("The engine is deterministic (ADR-0004): the same seed replays the same trajectory bit-for-bit, so every figure above is checkable.", "");
    }
    if (chosenApps.length) {
      L.push("## Where this can be applied", "");
      for (const a of chosenApps) {
        const m = TIER_META[a.tier];
        L.push(`### ${a.problem} — ${a.tier}: ${m.label}`);
        L.push(`*Domain:* ${a.domain}. *Reduction:* ${a.reduction}`);
        if (a.evidence) L.push(`*Evidence here:* ${a.evidence}`);
        if (a.notClaimed) L.push(`*What may NOT be concluded:* ${a.notClaimed}`);
        if (a.citations?.length) L.push(`*Reduction literature:* ${a.citations.map((c) => c.ref).join("; ")}`);
        L.push("");
      }
    }
    L.push("## Unknowns and open questions", "");
    L.push(unknowns.trim() || "_(required — a result without its uncertainty is a claim, not a finding)_", "");
    L.push("## Provenance", "");
    for (const s of reproSources) L.push(`- \`${s}\``);
    L.push("", "---", "", "_Assembled by the Ising Engine Publication Composer. Every figure is transcluded from the append-only record; nothing in this document was generated._");
    return L.join("\n");
  }, [claim, chosenFacts, bench, ev, chosenApps, unknowns, reproSources]);

  const bib = useMemo(() => {
    if (!claim) return "";
    const key = `isingengine${new Date().getFullYear()}${claim.id.replace(/[^a-z0-9]/gi, "")}`;
    return `@techreport{${key},\n  title  = {${claim.statement}},\n  author = {Ising Engine (autonomous research platform)},\n  year   = {${new Date().getFullYear()}},\n  note   = {Assembled from an append-only record of ${ev.runsAll} experiments; status: ${claim.status}},\n  url    = {https://localhost/publish}\n}`;
  }, [claim, ev.runsAll]);

  const copy = (t: string) => {
    navigator.clipboard?.writeText(t).then(() => { setCopied(true); setTimeout(() => setCopied(false), 1800); }).catch(() => {});
  };

  if (!claim) return <Panel className="p-6 text-[13px] text-[var(--os-faint)]">No claims available to publish.</Panel>;

  return (
    <div className="grid gap-4 lg:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
      {/* ─── selection ─── */}
      <div className="space-y-4">
        <Panel>
          <PanelHeader title="1 · The claim" status="real" sub="only claims already in the record" />
          <div className="divide-y divide-[var(--os-hair)]">
            {claims.map((c) => (
              <button key={c.id} onClick={() => setClaimId(c.id)}
                className={`flex w-full items-start gap-3 px-5 py-3 text-left transition-colors ${claimId === c.id ? "bg-[var(--os-accent-soft)]" : "hover:bg-[var(--os-raised)]"}`}>
                <Pill tone={c.status}>{c.status}</Pill>
                <span className="min-w-0 flex-1">
                  <span className="block text-[13px] text-[var(--os-ink)]">{c.statement}</span>
                  <span className="mono block text-[10.5px] text-[var(--os-faint)]">{c.source}</span>
                </span>
              </button>
            ))}
          </div>
        </Panel>

        <Panel>
          <PanelHeader title="2 · Supporting facts" status="real" sub="knowledge_graph.txt — pick what actually supports it" />
          <div className="max-h-[220px] overflow-y-auto divide-y divide-[var(--os-hair)]">
            {facts.slice(0, 24).map((f, i) => {
              const on = factKeys.includes(i);
              return (
                <button key={i} onClick={() => setFactKeys((k) => on ? k.filter((x) => x !== i) : [...k, i])}
                  className={`flex w-full items-center gap-2.5 px-5 py-2 text-left transition-colors ${on ? "bg-[var(--os-accent-soft)]" : "hover:bg-[var(--os-raised)]"}`}>
                  <span className={`flex h-3.5 w-3.5 shrink-0 items-center justify-center rounded-[3px] border ${on ? "border-[var(--os-confirmed)] bg-[var(--os-confirmed)]" : "border-[var(--os-hair-2)]"}`}>
                    {on && <Icon name="Check" size={9} className="text-[var(--os-ground)]" />}
                  </span>
                  <span className="mono min-w-0 flex-1 truncate text-[11px] text-[var(--os-muted)]">
                    {f.subject} <span className="text-[var(--os-faint)]">{f.relation}</span> {f.object || "—"}
                  </span>
                  <span className="mono shrink-0 text-[10px] text-[var(--os-faint)]">s{f.support}·{f.confidence?.toFixed(2) ?? "—"}</span>
                </button>
              );
            })}
          </div>
        </Panel>

        <Panel>
          <PanelHeader title="3 · Statistics" status="real" sub="results/index.json — the power caveat is added automatically" />
          <div className="divide-y divide-[var(--os-hair)]">
            {benches.length === 0 && <div className="px-5 py-4 text-[12.5px] text-[var(--os-faint)]">No comparisons ingested.</div>}
            {benches.map((b, i) => {
              const weak = b.instances < 6 || b.p > 0.05;
              return (
                <button key={i} onClick={() => setBenchIdx(benchIdx === i ? null : i)}
                  className={`flex w-full items-center gap-3 px-5 py-2.5 text-left transition-colors ${benchIdx === i ? "bg-[var(--os-accent-soft)]" : "hover:bg-[var(--os-raised)]"}`}>
                  <span className="mono min-w-0 flex-1 truncate text-[11.5px] text-[var(--os-muted)]">{b.engine} vs {b.vs}</span>
                  <span className="mono shrink-0 text-[11px] text-[var(--os-ink)]">{(b.win * 100).toFixed(0)}%</span>
                  <span className={`mono shrink-0 text-[10px] ${weak ? "text-[var(--os-warn)]" : "text-[var(--os-confirmed)]"}`}>n={b.instances} p={b.p}</span>
                </button>
              );
            })}
          </div>
        </Panel>

        <Panel>
          <PanelHeader title="4 · Applications" status="real" sub="tiered — T2 carries its own 'may not be concluded'" />
          <div className="max-h-[200px] overflow-y-auto divide-y divide-[var(--os-hair)]">
            {applications.map((a) => {
              const on = appIds.includes(a.id);
              return (
                <button key={a.id} onClick={() => setAppIds((k) => on ? k.filter((x) => x !== a.id) : [...k, a.id])}
                  className={`flex w-full items-center gap-2.5 px-5 py-2 text-left transition-colors ${on ? "bg-[var(--os-accent-soft)]" : "hover:bg-[var(--os-raised)]"}`}>
                  <span className={`flex h-3.5 w-3.5 shrink-0 items-center justify-center rounded-[3px] border ${on ? "border-[var(--os-confirmed)] bg-[var(--os-confirmed)]" : "border-[var(--os-hair-2)]"}`}>
                    {on && <Icon name="Check" size={9} className="text-[var(--os-ground)]" />}
                  </span>
                  <Pill tone={TIER_META[a.tier].tone}>{a.tier}</Pill>
                  <span className="min-w-0 flex-1 truncate text-[12px] text-[var(--os-muted)]">{a.problem}</span>
                </button>
              );
            })}
          </div>
        </Panel>

        <Panel>
          <PanelHeader title="5 · Unknowns" status="real" sub="required — this is what makes it a finding" />
          <div className="p-5">
            <textarea value={unknowns} onChange={(e) => setUnknowns(e.target.value)} rows={4}
              placeholder="Untested beyond n ≤ 2000. No comparison against specialised MaxCut heuristics. The mechanism has not been ablated, so the causal claim is provisional."
              className="mono w-full rounded-[8px] border border-[var(--os-hair-2)] bg-[var(--os-ground)] px-3 py-2 text-[12.5px] leading-[1.55] text-[var(--os-ink)] placeholder:text-[var(--os-faint)] focus:border-[var(--os-accent)] focus:outline-none" />
          </div>
        </Panel>
      </div>

      {/* ─── dossier ─── */}
      <div className="space-y-4">
        <Panel ticks>
          <PanelHeader title="Discovery dossier" status="real"
            sub="every figure transcluded from the record — nothing generated"
            right={
              <span className="flex gap-1.5">
                <button onClick={() => copy(md)} className="mono rounded-[7px] border border-[var(--os-hair-2)] px-2.5 py-1 text-[11px] text-[var(--os-muted)] hover:text-[var(--os-ink)]">{copied ? "copied ✓" : "copy md"}</button>
                <button onClick={() => window.print()} className="mono rounded-[7px] border border-[var(--os-hair-2)] px-2.5 py-1 text-[11px] text-[var(--os-muted)] hover:text-[var(--os-ink)]">print / pdf</button>
              </span>
            } />
          <div className="p-5">
            {!ready && (
              <div className="mb-3 flex flex-wrap items-center gap-2 rounded-[8px] border border-[color:rgba(232,177,74,.3)] bg-[rgba(232,177,74,.06)] px-3 py-2">
                <StatusBadge status="offline" />
                <span className="mono text-[10.5px] text-[var(--os-warn)]">
                  Incomplete: a dossier needs at least one supporting fact and a stated set of unknowns.
                </span>
              </div>
            )}
            <pre className="mono max-h-[560px] overflow-auto whitespace-pre-wrap rounded-[8px] border border-[var(--os-hair)] bg-[var(--os-ground)] p-4 text-[11.5px] leading-[1.65] text-[var(--os-ink)]">{md}</pre>
          </div>
        </Panel>

        <Panel>
          <PanelHeader title="Citation" status="real" sub="BibTeX"
            right={<button onClick={() => copy(bib)} className="mono rounded-[7px] border border-[var(--os-hair-2)] px-2.5 py-1 text-[11px] text-[var(--os-muted)] hover:text-[var(--os-ink)]">copy</button>} />
          <div className="p-5">
            <pre className="mono overflow-x-auto rounded-[8px] border border-[var(--os-hair)] bg-[var(--os-ground)] p-3.5 text-[11px] leading-[1.6] text-[var(--os-muted)]">{bib}</pre>
            <p className="mono mt-2.5 text-[10.5px] leading-[1.5] text-[var(--os-faint)]">
              The platform also writes its own numbered reports, never rewritten — see{" "}
              <Link href="/papers" className="text-[var(--os-accent)] hover:underline">Research Papers</Link>. This
              composer is for a human assembling one finding for an outside audience.
            </p>
          </div>
        </Panel>
      </div>
    </div>
  );
}
