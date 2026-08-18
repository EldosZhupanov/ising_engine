"use client";

/** Ranked research gaps. Opening one records the mission action. */

import { useState } from "react";
import Link from "next/link";
import type { Gap, GapKind } from "@/lib/workflow/gaps";
import { recordAction } from "@/lib/workflow/progress";
import { Icon, Panel, Pill } from "@/components/os/kit";

const TONE: Record<GapKind, "confirmed" | "refuted" | "open" | "neutral"> = {
  operator: "confirmed", evidence: "open", reproduction: "open", power: "refuted", data: "neutral",
};

export default function GapList({
  gaps, meta,
}: { gaps: Gap[]; meta: Record<GapKind, { label: string; blurb: string }> }) {
  const [filter, setFilter] = useState<GapKind | null>(null);
  const kinds = Array.from(new Set(gaps.map((g) => g.kind)));
  const shown = filter ? gaps.filter((g) => g.kind === filter) : gaps;

  return (
    <Panel className="overflow-hidden">
      <div className="flex flex-wrap items-center gap-2 border-b border-[var(--os-hair)] px-5 py-3">
        <span className="mono text-[11px] uppercase tracking-[.16em] text-[var(--os-muted)]">Ranked gaps</span>
        <button onClick={() => setFilter(null)}
          className={`mono rounded-full border px-2.5 py-0.5 text-[11px] ${!filter ? "border-[var(--os-accent)] text-[var(--os-ink)]" : "border-[var(--os-hair-2)] text-[var(--os-muted)]"}`}>all</button>
        {kinds.map((k) => (
          <button key={k} onClick={() => setFilter(filter === k ? null : k)}
            className={`mono rounded-full border px-2.5 py-0.5 text-[11px] ${filter === k ? "border-[var(--os-accent)] text-[var(--os-ink)]" : "border-[var(--os-hair-2)] text-[var(--os-muted)]"}`}>
            {meta[k].label}
          </button>
        ))}
        <span className="mono ml-auto text-[10.5px] text-[var(--os-faint)]">{shown.length} of {gaps.length}</span>
      </div>

      {filter && (
        <div className="border-b border-[var(--os-hair)] bg-[var(--os-ground-2)] px-5 py-2.5">
          <p className="text-[12.5px] leading-[1.5] text-[var(--os-muted)]">{meta[filter].blurb}</p>
        </div>
      )}

      <div className="divide-y divide-[var(--os-hair)]">
        {shown.map((g, i) => (
          <div key={i} className="px-5 py-3.5">
            <div className="flex flex-wrap items-center gap-2">
              <Pill tone={TONE[g.kind]}>{meta[g.kind].label}</Pill>
              <h3 className="text-[13.5px] font-medium text-[var(--os-ink)]">{g.title}</h3>
            </div>
            <p className="mt-1.5 max-w-[92ch] text-[12.5px] leading-[1.55] text-[var(--os-muted)]">{g.evidence}</p>
            <p className="mt-1.5 max-w-[92ch] text-[12.5px] leading-[1.55] text-[var(--os-ink)]">
              <span className="mono text-[9.5px] uppercase tracking-[.14em] text-[var(--os-accent)]">next</span>{" "}{g.action}
            </p>
            <div className="mono mt-2 flex flex-wrap items-center gap-3 text-[10.5px] text-[var(--os-faint)]">
              <Link href={g.route} onClick={() => recordAction("gap.opened")}
                className="inline-flex items-center gap-1 text-[var(--os-accent)] hover:underline">
                investigate <Icon name="ArrowRight" size={10} />
              </Link>
              <span className="inline-flex items-center gap-1"><Icon name="FileCode2" size={10} /> {g.source}</span>
            </div>
          </div>
        ))}
        {shown.length === 0 && <div className="px-5 py-8 text-center text-[13px] text-[var(--os-faint)]">No gaps of this kind.</div>}
      </div>
    </Panel>
  );
}
