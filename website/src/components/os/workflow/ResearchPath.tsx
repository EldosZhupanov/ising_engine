"use client";

/**
 * The research path — "I opened the platform" → "I discovered a new algorithm".
 *
 * Nine stages of real scientific competence, each with missions completed by
 * doing the work. Stage 7 (run & refute) is honestly marked as needing the
 * control plane; everything before it is completable today.
 */

import Link from "next/link";
import { STAGES, MISSIONS, missionsForStage } from "@/lib/workflow/curriculum";
import { useProgress, useMode } from "@/lib/workflow/progress";
import { term } from "@/lib/workflow/glossary";
import { Icon, Panel, StatusBadge, Pill } from "@/components/os/kit";
import TermChip from "./TermChip";

export default function ResearchPath() {
  const { done, toggleManual, availableCompleted, availableCount, nextMission, stageProgress, reset } = useProgress();
  const { isBeginner } = useMode();
  const pct = Math.round((availableCompleted / Math.max(1, availableCount)) * 100);

  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      {/* header */}
      <div className="mb-7">
        <div className="mono mb-3 flex items-center gap-3 text-[11px] uppercase tracking-[.2em] text-[var(--os-muted)]">
          <span className="text-[var(--os-accent)]">◆</span><span className="h-px w-7 bg-[var(--os-hair-2)]" /><span>Research path</span>
        </div>
        <h1 className="max-w-[30ch] text-[clamp(1.7rem,2.8vw,2.5rem)] font-semibold leading-[1.06] tracking-[-.03em] text-balance">
          From opening this platform to discovering a new algorithm.
        </h1>
        <p className="mt-3 max-w-[76ch] text-[14px] leading-[1.65] text-[var(--os-muted)]">
          This is not a product tour. Each step is something a computational scientist actually does, and it completes
          when you <span className="text-[var(--os-ink)]">do the thing</span> — not when you click next. Where the
          platform genuinely cannot yet act, the step says so and names what is missing.
        </p>
      </div>

      {/* progress + next action */}
      <div className="mb-6 grid gap-4 lg:grid-cols-[1fr_1.3fr]">
        <Panel ticks className="p-5">
          <div className="mono text-[10px] uppercase tracking-[.16em] text-[var(--os-muted)]">Progress</div>
          <div className="mt-2 flex items-baseline gap-2">
            <span className="mono text-[2.4rem] font-semibold leading-none tnum text-[var(--os-accent)]">{availableCompleted}</span>
            <span className="mono text-[13px] text-[var(--os-faint)]">/ {availableCount} completable today</span>
          </div>
          <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-[var(--os-raised)]">
            <div className="h-full rounded-full bg-[var(--os-accent)] transition-[width] duration-500" style={{ width: `${pct}%` }} />
          </div>
          <div className="mono mt-2 flex items-center justify-between text-[10.5px] text-[var(--os-faint)]">
            <span>{MISSIONS.length} steps total · {MISSIONS.length - availableCount} need the control plane</span>
            <button onClick={reset} className="hover:text-[var(--os-ink)]">reset</button>
          </div>
        </Panel>

        <Panel ticks className="p-5">
          <div className="mono text-[10px] uppercase tracking-[.16em] text-[var(--os-accent)]">Do this next</div>
          {nextMission ? (
            <>
              <h2 className="mt-2 text-[16px] font-semibold tracking-[-.01em] text-[var(--os-ink)]">{nextMission.title}</h2>
              <p className="mt-1.5 text-[13px] leading-[1.55] text-[var(--os-muted)]">{nextMission.task}</p>
              <p className="mt-2.5 border-l-2 border-[var(--os-hair-2)] pl-3 text-[12.5px] leading-[1.55] text-[var(--os-muted)]">
                <span className="mono text-[9.5px] uppercase tracking-[.14em] text-[var(--os-faint)]">why</span><br />{nextMission.why}
              </p>
              <Link href={nextMission.route}
                className="mono mt-4 inline-flex items-center gap-1.5 rounded-[8px] bg-[var(--os-accent)] px-3.5 py-2 text-[12px] font-medium text-[var(--os-on-accent)]">
                go to {nextMission.route} <Icon name="ArrowRight" size={13} />
              </Link>
            </>
          ) : (
            <p className="mt-2 text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
              Every step available in this build is complete. What remains needs the control-plane API —
              launching campaigns and running ablations from the browser.
            </p>
          )}
        </Panel>
      </div>

      {/* stages */}
      <div className="space-y-3">
        {STAGES.map((s) => {
          const ms = missionsForStage(s.n);
          const sp = stageProgress(s.n);
          const complete = sp.total > 0 && sp.done === sp.total;
          const offline = ms.every((m) => m.status === "offline");
          return (
            <Panel key={s.n} className="overflow-hidden">
              <div className="flex flex-wrap items-center gap-3 border-b border-[var(--os-hair)] px-5 py-3.5">
                <span className={`mono flex h-7 w-7 shrink-0 items-center justify-center rounded-full border text-[11px] ${
                  complete ? "border-[var(--os-confirmed)] bg-[var(--os-confirmed)] text-[var(--os-ground)]"
                  : "border-[var(--os-hair-2)] text-[var(--os-muted)]"}`}>
                  {complete ? <Icon name="Check" size={13} /> : s.n}
                </span>
                <span className="min-w-0 flex-1">
                  <span className="flex items-center gap-2">
                    <h2 className="text-[15px] font-semibold tracking-[-.01em] text-[var(--os-ink)]">{s.title}</h2>
                    {offline && <StatusBadge status="offline" />}
                  </span>
                  <span className="mono block text-[11px] text-[var(--os-faint)]">{s.arc}</span>
                </span>
                <span className="mono text-[11px] tnum text-[var(--os-muted)]">{sp.done}/{sp.total}</span>
              </div>

              {isBeginner && (
                <div className="border-b border-[var(--os-hair)] bg-[var(--os-ground-2)] px-5 py-2.5">
                  <span className="mono text-[9.5px] uppercase tracking-[.14em] text-[var(--os-accent)]">competence gained</span>
                  <p className="mt-0.5 text-[12.5px] leading-[1.5] text-[var(--os-muted)]">{s.outcome}</p>
                </div>
              )}

              <div className="divide-y divide-[var(--os-hair)]">
                {ms.map((m) => {
                  const ok = done(m);
                  return (
                    <div key={m.id} className="flex items-start gap-3 px-5 py-3.5">
                      {m.verify.kind === "manual" ? (
                        <button onClick={() => toggleManual(m.id)} aria-pressed={ok}
                          aria-label={`Mark "${m.title}" ${ok ? "incomplete" : "complete"}`}
                          className={`mt-[3px] flex h-4.5 w-4.5 shrink-0 items-center justify-center rounded-[4px] border ${ok ? "border-[var(--os-confirmed)] bg-[var(--os-confirmed)]" : "border-[var(--os-hair-2)] hover:border-[var(--os-accent)]"}`}>
                          {ok && <Icon name="Check" size={11} className="text-[var(--os-ground)]" />}
                        </button>
                      ) : (
                        <span title={ok ? "completed by your action" : "completes when you do this"}
                          className={`mt-[3px] flex h-4.5 w-4.5 shrink-0 items-center justify-center rounded-full border ${ok ? "border-[var(--os-confirmed)] bg-[var(--os-confirmed)]" : "border-[var(--os-hair-2)]"}`}>
                          {ok && <Icon name="Check" size={11} className="text-[var(--os-ground)]" />}
                        </span>
                      )}
                      <div className="min-w-0 flex-1">
                        <div className="flex flex-wrap items-center gap-2">
                          <h3 className="text-[13.5px] font-medium text-[var(--os-ink)]">{m.title}</h3>
                          {m.status === "offline" ? <Pill tone="neutral">needs control plane</Pill> : null}
                        </div>
                        <p className="mt-1 text-[12.5px] leading-[1.55] text-[var(--os-muted)]">{m.task}</p>
                        {isBeginner && <p className="mt-1.5 text-[12px] leading-[1.5] text-[var(--os-faint)]"><span className="text-[var(--os-muted)]">Why:</span> {m.why}</p>}
                        {m.blocked && <p className="mono mt-1.5 text-[10.5px] leading-[1.5] text-[var(--os-warn)]">{m.blocked}</p>}
                        <div className="mt-2 flex flex-wrap items-center gap-1.5">
                          <Link href={m.route} className="mono text-[11px] text-[var(--os-accent)] hover:underline">{m.route}</Link>
                          {isBeginner && m.teaches?.map((k) => term(k) && <TermChip key={k} k={k} />)}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </Panel>
          );
        })}
      </div>
    </div>
  );
}
