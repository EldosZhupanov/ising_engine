"use client";

/**
 * The two questions every workspace must answer: "Why does this matter?" and
 * "What should I do next?".
 *
 * In Beginner mode it is expanded by default; in Professional mode it collapses
 * to a single line so it never gets in an expert's way (progressive disclosure).
 * Missions for the current route are shown with their real completion state.
 */

import { useState } from "react";
import Link from "next/link";
import { guideFor, missionsForRoute } from "@/lib/workflow/curriculum";
import { useMode, useProgress } from "@/lib/workflow/progress";
import { term } from "@/lib/workflow/glossary";
import { Icon, StatusBadge } from "@/components/os/kit";
import TermChip from "./TermChip";

export default function Guide({ route }: { route: string }) {
  const g = guideFor(route);
  const { isBeginner } = useMode();
  const { done, toggleManual } = useProgress();
  // Derive from the mode store rather than seeding state from it: during
  // hydration the store returns its SERVER snapshot, so `useState(isBeginner)`
  // would freeze the initial value and mode switches would never apply.
  // An explicit user toggle overrides the mode default.
  const [manualOpen, setManualOpen] = useState<boolean | null>(null);
  const open = manualOpen ?? isBeginner;
  const setOpen = (v: boolean) => setManualOpen(v);
  const missions = missionsForRoute(route);
  if (!g) return null;

  return (
    <section aria-label="Workspace guidance" className="os-panel mb-5 overflow-hidden">
      <button
        onClick={() => setOpen(!open)}
        aria-expanded={open}
        className="flex w-full items-center gap-3 px-5 py-3 text-left transition-colors hover:bg-[var(--os-raised)]"
      >
        <Icon name="Compass" size={15} className="shrink-0 text-[var(--os-accent)]" />
        <span className="min-w-0 flex-1">
          <span className="mono block text-[10px] uppercase tracking-[.16em] text-[var(--os-muted)]">Why this matters</span>
          <span className={`block text-[13px] leading-[1.5] text-[var(--os-ink)] ${open ? "" : "truncate"}`}>{g.matters}</span>
        </span>
        <Icon name={open ? "ChevronUp" : "ChevronDown"} size={14} className="shrink-0 text-[var(--os-faint)]" />
      </button>

      {open && (
        <div className="border-t border-[var(--os-hair)] px-5 py-4">
          <div className="grid gap-5 lg:grid-cols-[1.1fr_1fr]">
            <div>
              <div className="mono mb-2 text-[10px] uppercase tracking-[.16em] text-[var(--os-accent)]">What to do next</div>
              <ol className="space-y-1.5">
                {g.next.map((n, i) => (
                  <li key={i} className="flex gap-2.5 text-[13px] leading-[1.55] text-[var(--os-muted)]">
                    <span className="mono mt-[2px] shrink-0 text-[10px] text-[var(--os-faint)]">{i + 1}</span>
                    <span>{n}</span>
                  </li>
                ))}
              </ol>
              {g.teaches?.length ? (
                <div className="mt-3.5 flex flex-wrap items-center gap-1.5">
                  <span className="mono text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">concepts</span>
                  {g.teaches.map((k) => term(k) && <TermChip key={k} k={k} />)}
                </div>
              ) : null}
            </div>

            {missions.length > 0 && (
              <div>
                <div className="mono mb-2 text-[10px] uppercase tracking-[.16em] text-[var(--os-muted)]">
                  Missions here
                </div>
                <div className="space-y-1.5">
                  {missions.map((m) => {
                    const ok = done(m);
                    return (
                      <div key={m.id} className={`rounded-[9px] border px-3 py-2.5 ${ok ? "border-[color:rgba(63,201,138,.35)] bg-[rgba(63,201,138,.06)]" : "border-[var(--os-hair)] bg-[var(--os-ground-2)]"}`}>
                        <div className="flex items-start gap-2.5">
                          {m.verify.kind === "manual" ? (
                            <button onClick={() => toggleManual(m.id)} aria-pressed={ok}
                              aria-label={`Mark "${m.title}" ${ok ? "incomplete" : "complete"}`}
                              className={`mt-[1px] flex h-4 w-4 shrink-0 items-center justify-center rounded-[4px] border ${ok ? "border-[var(--os-confirmed)] bg-[var(--os-confirmed)]" : "border-[var(--os-hair-2)]"}`}>
                              {ok && <Icon name="Check" size={11} className="text-[var(--os-ground)]" />}
                            </button>
                          ) : (
                            <span className={`mt-[1px] flex h-4 w-4 shrink-0 items-center justify-center rounded-full border ${ok ? "border-[var(--os-confirmed)] bg-[var(--os-confirmed)]" : "border-[var(--os-hair-2)]"}`}>
                              {ok && <Icon name="Check" size={11} className="text-[var(--os-ground)]" />}
                            </span>
                          )}
                          <span className="min-w-0 flex-1">
                            <span className="block text-[12.5px] font-medium text-[var(--os-ink)]">{m.title}</span>
                            <span className="mt-0.5 block text-[12px] leading-[1.5] text-[var(--os-muted)]">{m.task}</span>
                            {m.status === "offline" && (
                              <span className="mt-1.5 flex flex-wrap items-center gap-1.5">
                                <StatusBadge status="offline" />
                                <span className="mono text-[10px] text-[var(--os-faint)]">{m.blocked}</span>
                              </span>
                            )}
                          </span>
                        </div>
                      </div>
                    );
                  })}
                </div>
                <Link href="/start" className="mono mt-2.5 inline-flex items-center gap-1 text-[11px] text-[var(--os-muted)] hover:text-[var(--os-accent)]">
                  see the whole research path <Icon name="ArrowRight" size={11} />
                </Link>
              </div>
            )}
          </div>
        </div>
      )}
    </section>
  );
}
