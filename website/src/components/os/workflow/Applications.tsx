"use client";

/**
 * Application Explorer — a reduction atlas with an honesty gate (AD-4).
 *
 * The tier is the product. A domain may only appear with an experiment run here
 * (T1) or a published reduction (T2); anything else is an open question (T3) and
 * is rendered as a question, never as a capability. T2/T3 always show what may
 * NOT be concluded.
 */

import { useState } from "react";
import { APPLICATIONS, TIER_META, TRANSFER_EVIDENCE, type Tier } from "@/data/applications";
import { recordAction } from "@/lib/workflow/progress";
import { Icon, Panel, Pill } from "@/components/os/kit";

const ORDER: Tier[] = ["T1", "T2", "T3"];

export default function Applications() {
  const [tier, setTier] = useState<Tier | null>(null);
  const [open, setOpen] = useState<string | null>(null);
  const shown = tier ? APPLICATIONS.filter((a) => a.tier === tier) : APPLICATIONS;

  return (
    <>
      {/* the one measured transfer result */}
      <Panel ticks className="mb-4 p-5">
        <div className="mono mb-1.5 text-[10px] uppercase tracking-[.16em] text-[var(--os-accent)]">the only measured transfer result</div>
        <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
          <span className="text-[14px] text-[var(--os-ink)]">{TRANSFER_EVIDENCE.claim}</span>
          <span className="mono text-[14px] font-semibold text-[var(--os-confirmed)]">{TRANSFER_EVIDENCE.result}</span>
        </div>
        <p className="mt-2 max-w-[90ch] text-[12.5px] leading-[1.55] text-[var(--os-muted)]">{TRANSFER_EVIDENCE.caveat}</p>
        <div className="mono mt-2 text-[10.5px] text-[var(--os-faint)]">{TRANSFER_EVIDENCE.source}</div>
      </Panel>

      {/* tier legend / filter */}
      <div className="mb-3 grid gap-2 md:grid-cols-3">
        {ORDER.map((t) => {
          const m = TIER_META[t];
          const n = APPLICATIONS.filter((a) => a.tier === t).length;
          const on = tier === t;
          return (
            <button key={t} onClick={() => { setTier(on ? null : t); recordAction("application.tier.opened"); }}
              aria-pressed={on}
              className={`os-panel p-4 text-left transition-colors ${on ? "border-[var(--os-accent)]" : "hover:border-[var(--os-hair-2)]"}`}>
              <div className="flex items-center justify-between">
                <Pill tone={m.tone}>{t} · {m.label}</Pill>
                <span className="mono text-[11px] tnum text-[var(--os-faint)]">{n}</span>
              </div>
              <p className="mt-2 text-[12px] leading-[1.5] text-[var(--os-muted)]">{m.blurb}</p>
            </button>
          );
        })}
      </div>

      {/* atlas */}
      <Panel className="overflow-hidden">
        <div className="divide-y divide-[var(--os-hair)]">
          {shown.map((a) => {
            const isOpen = open === a.id;
            const m = TIER_META[a.tier];
            return (
              <div key={a.id}>
                <button onClick={() => { setOpen(isOpen ? null : a.id); recordAction("application.tier.opened"); }}
                  aria-expanded={isOpen}
                  className="flex w-full items-start gap-3 px-5 py-3.5 text-left transition-colors hover:bg-[var(--os-raised)]">
                  <Pill tone={m.tone}>{a.tier}</Pill>
                  <span className="min-w-0 flex-1">
                    <span className="block text-[13.5px] font-medium text-[var(--os-ink)]">{a.problem}</span>
                    <span className="mono block text-[11px] text-[var(--os-faint)]">{a.domain} · {a.sectors.slice(0, 3).join(" · ")}</span>
                  </span>
                  <Icon name={isOpen ? "ChevronUp" : "ChevronDown"} size={14} className="mt-1 shrink-0 text-[var(--os-faint)]" />
                </button>

                {isOpen && (
                  <div className="space-y-3 border-t border-[var(--os-hair)] bg-[var(--os-ground-2)] px-5 py-4">
                    <div>
                      <div className="mono text-[9.5px] uppercase tracking-[.14em] text-[var(--os-faint)]">reduction to QUBO / Ising</div>
                      <p className="mt-1 max-w-[92ch] text-[12.5px] leading-[1.55] text-[var(--os-ink)]">{a.reduction}</p>
                    </div>

                    {a.evidence && (
                      <div>
                        <div className="mono text-[9.5px] uppercase tracking-[.14em] text-[var(--os-confirmed)]">evidence on this platform</div>
                        <p className="mt-1 max-w-[92ch] text-[12.5px] leading-[1.55] text-[var(--os-ink)]">{a.evidence}</p>
                      </div>
                    )}

                    {a.notClaimed && (
                      <div className="rounded-[8px] border border-[color:rgba(240,113,90,.3)] bg-[rgba(240,113,90,.06)] px-3.5 py-2.5">
                        <div className="mono text-[9.5px] uppercase tracking-[.14em] text-[var(--os-refuted)]">what may NOT be concluded</div>
                        <p className="mt-1 max-w-[92ch] text-[12.5px] leading-[1.55] text-[var(--os-ink)]">{a.notClaimed}</p>
                      </div>
                    )}

                    <div className="grid gap-3 md:grid-cols-2">
                      <div>
                        <div className="mono text-[9.5px] uppercase tracking-[.14em] text-[var(--os-faint)]">unknowns</div>
                        <ul className="mt-1 space-y-0.5">
                          {a.unknowns.map((u, i) => <li key={i} className="text-[12px] leading-[1.5] text-[var(--os-muted)]">· {u}</li>)}
                        </ul>
                      </div>
                      <div>
                        <div className="mono text-[9.5px] uppercase tracking-[.14em] text-[var(--os-faint)]">where it would be used</div>
                        <div className="mt-1 flex flex-wrap gap-1.5">
                          {a.sectors.map((s) => (
                            <span key={s} className="mono rounded-md border border-[var(--os-hair-2)] px-2 py-0.5 text-[11px] text-[var(--os-muted)]">{s}</span>
                          ))}
                        </div>
                      </div>
                    </div>

                    {a.citations?.length ? (
                      <div className="border-t border-[var(--os-hair)] pt-2.5">
                        <div className="mono text-[9.5px] uppercase tracking-[.14em] text-[var(--os-faint)]">reduction literature</div>
                        <ul className="mt-1 space-y-1">
                          {a.citations.map((c, i) => (
                            <li key={i} className="text-[11.5px] leading-[1.5] text-[var(--os-muted)]">
                              {c.ref}{c.note ? <span className="text-[var(--os-faint)]"> — {c.note}</span> : null}
                            </li>
                          ))}
                        </ul>
                      </div>
                    ) : (
                      <div className="mono border-t border-[var(--os-hair)] pt-2.5 text-[10.5px] text-[var(--os-warn)]">
                        No reduction citation — this is an open question, listed as a direction only.
                      </div>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </Panel>
    </>
  );
}
