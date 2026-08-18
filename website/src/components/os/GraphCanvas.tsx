"use client";

/** Interactive knowledge-graph over the real facts (knowledge_graph.txt). */

import { useMemo, useState } from "react";
import type { Fact } from "@/data";
import { recordAction } from "@/lib/workflow/progress";

const REL_COLOR: Record<string, string> = {
  "fails-on": "var(--os-refuted)",
  "effective-on": "var(--os-confirmed)",
  "precedes-well": "var(--os-cyan)",
};
const relColor = (r: string) => REL_COLOR[r] ?? "var(--os-violet)";

export default function GraphCanvas({ facts, relations }: { facts: Fact[]; relations: string[] }) {
  const [rel, setRel] = useState<string | null>(null);
  const [hover, setHover] = useState<string | null>(null);
  const [sel, setSel] = useState<Fact | null>(null);

  // Selecting a fact IS the mission action (Stage 2) — record it once.
  const onSelect = (f: Fact) => { setSel(f); recordAction("graph.fact.inspected"); };

  const shown = useMemo(() => (rel ? facts.filter((f) => f.relation === rel) : facts), [facts, rel]);
  const nodes = useMemo(() => [...new Set(shown.flatMap((f) => [f.subject, f.object].filter(Boolean)))], [shown]);

  const cx = 320, cy = 300, R = 235;
  const pos: Record<string, { x: number; y: number }> = {};
  nodes.forEach((n, i) => {
    const a = (i / Math.max(1, nodes.length)) * Math.PI * 2 - Math.PI / 2;
    pos[n] = { x: cx + Math.cos(a) * R, y: cy + Math.sin(a) * R };
  });
  const maxSup = Math.max(...shown.map((f) => f.support), 1);

  return (
    <div className="grid gap-4 lg:grid-cols-[1.5fr_1fr]">
      <div className="os-panel os-ticks overflow-hidden">
        <div className="flex flex-wrap items-center gap-2 border-b border-[var(--os-hair)] px-4 py-2.5">
          <span className="mono text-[11px] uppercase tracking-[.14em] text-[var(--os-muted)]">Relation</span>
          <button onClick={() => setRel(null)} className={`mono rounded-full border px-2.5 py-0.5 text-[11px] ${!rel ? "border-[var(--os-accent)] text-[var(--os-ink)]" : "border-[var(--os-hair-2)] text-[var(--os-muted)]"}`}>all</button>
          {relations.map((r) => (
            <button key={r} onClick={() => { setRel(rel === r ? null : r); recordAction("graph.relation.filtered"); }} className={`mono inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11px] ${rel === r ? "border-[var(--os-accent)] text-[var(--os-ink)]" : "border-[var(--os-hair-2)] text-[var(--os-muted)]"}`}>
              <span className="h-1.5 w-1.5 rounded-full" style={{ background: relColor(r) }} />{r}
            </button>
          ))}
          <span className="mono ml-auto text-[10.5px] text-[var(--os-faint)]">{nodes.length} nodes · {shown.length} facts</span>
        </div>
        <svg viewBox="0 0 640 600" className="h-auto w-full" role="group"
          aria-label={`Knowledge graph: ${nodes.length} entities, ${shown.length} facts. Use Tab to move between facts and Enter to inspect one.`}>
          {shown.map((f, i) => {
            const A = pos[f.subject], B = pos[f.object];
            if (!A || !B) return null;
            const lit = hover === f.subject || hover === f.object || sel === f;
            const label = `${f.subject} ${f.relation} ${f.object || "unspecified"}, support ${f.support}${f.confidence !== null ? `, confidence ${f.confidence.toFixed(2)}` : ""}`;
            return (
              // Each edge is a focusable, activatable element so the graph is
              // fully usable without a mouse.
              <g key={i} role="button" tabIndex={0} aria-label={label}
                onFocus={() => setHover(f.subject)} onBlur={() => setHover(null)}
                onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onSelect(f); } }}
                onClick={() => onSelect(f)} style={{ cursor: "pointer" }}>
                <title>{label}</title>
                {/* invisible fat hit-area makes thin edges clickable/focusable */}
                <line x1={A.x} y1={A.y} x2={B.x} y2={B.y} stroke="transparent" strokeWidth={10} />
                <line x1={A.x} y1={A.y} x2={B.x} y2={B.y} stroke={relColor(f.relation)}
                  strokeWidth={lit ? 2 : 0.4 + (f.support / maxSup) * 1.6}
                  opacity={hover && !lit ? 0.08 : (f.confidence ?? 0.6) * 0.7 + 0.15} />
              </g>
            );
          })}
          {nodes.map((n) => {
            const p = pos[n], on = hover === n;
            const incident = shown.filter((f) => f.subject === n || f.object === n).length;
            return (
              <g key={n} transform={`translate(${p.x},${p.y})`} role="button" tabIndex={0}
                aria-label={`${n} — ${incident} incident facts`}
                onMouseEnter={() => setHover(n)} onMouseLeave={() => setHover(null)}
                onFocus={() => setHover(n)} onBlur={() => setHover(null)}
                style={{ cursor: "pointer" }}>
                <title>{`${n} — ${incident} incident facts`}</title>
                <circle r={on ? 6 : 4} fill={on ? "var(--os-accent)" : "var(--os-ground)"} stroke={on ? "var(--os-accent)" : "var(--os-hair-2)"} strokeWidth="1.5" />
                <text y={-11} textAnchor="middle" className="mono" fontSize="10" fill={on ? "var(--os-ink)" : "var(--os-faint)"}>{n}</text>
              </g>
            );
          })}
        </svg>
        {/* text equivalent — the graph's content, available to screen readers and keyboard users */}
        <details className="border-t border-[var(--os-hair)] px-5 py-3">
          <summary className="mono cursor-pointer text-[11px] text-[var(--os-muted)]">Facts as a table ({shown.length})</summary>
          <table className="mt-3 w-full text-left">
            <caption className="sr-only">Knowledge-graph facts with evidence weight, support and confidence</caption>
            <thead><tr className="mono text-[10px] uppercase tracking-[.08em] text-[var(--os-faint)]">
              <th scope="col" className="py-1.5 font-normal">subject</th><th scope="col" className="py-1.5 font-normal">relation</th>
              <th scope="col" className="py-1.5 font-normal">object</th><th scope="col" className="py-1.5 font-normal">support</th>
              <th scope="col" className="py-1.5 font-normal">conf</th></tr></thead>
            <tbody>
              {/* Precise selection path: edges overlap in a dense layout, so the
                  table is the reliable way to pick an exact fact (and works
                  without a mouse). */}
              {shown.map((f, i) => (
                <tr key={i}
                  onClick={() => onSelect(f)}
                  onMouseEnter={() => setHover(f.subject)}
                  onMouseLeave={() => setHover(null)}
                  className={`mono cursor-pointer border-t border-[var(--os-hair)] text-[11px] transition-colors ${sel === f ? "bg-[var(--os-accent-soft)]" : "hover:bg-[var(--os-raised)]"}`}>
                  <td className="py-1.5">
                    <button className="text-left text-[var(--os-ink)]"
                      aria-label={`Inspect: ${f.subject} ${f.relation} ${f.object || "unspecified"}`}>
                      {f.subject}
                    </button>
                  </td>
                  <td className="py-1.5" style={{ color: relColor(f.relation) }}>{f.relation}</td>
                  <td className="py-1.5 text-[var(--os-muted)]">{f.object || "—"}</td>
                  <td className="py-1.5 tnum text-[var(--os-muted)]">{f.support}</td>
                  <td className="py-1.5 tnum text-[var(--os-muted)]">{f.confidence?.toFixed(2) ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </details>
      </div>

      <div className="os-panel os-ticks p-5">
        <div className="mono mb-3 text-[11px] uppercase tracking-[.16em] text-[var(--os-muted)]">Fact inspector</div>
        {sel ? (
          <div className="space-y-3">
            <div className="mono flex flex-wrap items-center gap-2 text-[13px]">
              <span className="text-[var(--os-ink)]">{sel.subject}</span>
              <span style={{ color: relColor(sel.relation) }}>—{sel.relation}→</span>
              <span className="text-[var(--os-ink)]">{sel.object || "∅"}</span>
            </div>
            <div className="grid grid-cols-2 gap-3 border-t border-[var(--os-hair)] pt-3">
              {[["weight", sel.weight.toFixed(2)], ["support", String(sel.support)], ["confidence", sel.confidence?.toFixed(2) ?? "—"], ["variance", sel.variance.toFixed(1)]].map(([k, v]) => (
                <div key={k}><div className="mono text-[10px] uppercase tracking-[.08em] text-[var(--os-faint)]">{k}</div><div className="mono text-[15px] text-[var(--os-ink)] tnum">{v}</div></div>
              ))}
            </div>
            <div className="mono text-[11px] text-[var(--os-faint)]">{sel.source}</div>
          </div>
        ) : (
          <p className="text-[13px] leading-[1.6] text-[var(--os-muted)]">Click an edge to inspect a fact — its evidence weight, support count, variance, and confidence. Hover a node to trace its incident relations.</p>
        )}
      </div>
    </div>
  );
}
