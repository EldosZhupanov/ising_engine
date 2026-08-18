"use client";

/** Research playbooks — method, encoded. Each step names the mistake it prevents. */

import { useState } from "react";
import Link from "next/link";
import { PLAYBOOKS } from "@/lib/workflow/playbooks";
import { useMode, recordAction } from "@/lib/workflow/progress";
import { term } from "@/lib/workflow/glossary";
import { Icon, Panel, Pill } from "@/components/os/kit";
import TermChip from "./TermChip";

export default function Playbooks() {
  const { isBeginner } = useMode();
  const [open, setOpen] = useState<string | null>(PLAYBOOKS[0].id);
  const [checked, setChecked] = useState<Record<string, boolean>>({});

  const key = (p: string, i: number) => `${p}:${i}`;
  const stepsDone = (id: string, total: number) => {
    const n = Array.from({ length: total }, (_, i) => checked[key(id, i)]).filter(Boolean).length;
    return n;
  };

  return (
    <div className="space-y-3">
      {PLAYBOOKS.map((p) => {
        const isOpen = open === p.id;
        const n = stepsDone(p.id, p.steps.length);
        const complete = n === p.steps.length;
        return (
          <Panel key={p.id} className="overflow-hidden">
            <button onClick={() => setOpen(isOpen ? null : p.id)} aria-expanded={isOpen}
              className="flex w-full items-start gap-3 px-5 py-4 text-left transition-colors hover:bg-[var(--os-raised)]">
              <span className={`mono mt-[2px] flex h-6 w-6 shrink-0 items-center justify-center rounded-full border text-[10px] ${complete ? "border-[var(--os-confirmed)] bg-[var(--os-confirmed)] text-[var(--os-ground)]" : "border-[var(--os-hair-2)] text-[var(--os-muted)]"}`}>
                {complete ? <Icon name="Check" size={12} /> : n || ""}
              </span>
              <span className="min-w-0 flex-1">
                <span className="flex flex-wrap items-center gap-2">
                  <h2 className="text-[15px] font-semibold tracking-[-.01em] text-[var(--os-ink)]">{p.title}</h2>
                  <Pill tone="neutral">{p.level}</Pill>
                  <span className="mono text-[10.5px] text-[var(--os-faint)]">~{p.minutes} min · {p.steps.length} steps</span>
                </span>
                <span className="mt-1 block text-[13px] italic leading-[1.5] text-[var(--os-muted)]">&ldquo;{p.question}&rdquo;</span>
                {!isOpen && <span className="mono mt-1 block text-[11px] text-[var(--os-faint)]">{p.use}</span>}
              </span>
              <Icon name={isOpen ? "ChevronUp" : "ChevronDown"} size={15} className="mt-1 shrink-0 text-[var(--os-faint)]" />
            </button>

            {isOpen && (
              <div className="border-t border-[var(--os-hair)]">
                <div className="border-b border-[var(--os-hair)] bg-[var(--os-ground-2)] px-5 py-2.5">
                  <span className="mono text-[9.5px] uppercase tracking-[.14em] text-[var(--os-accent)]">when to use</span>
                  <p className="mt-0.5 text-[12.5px] text-[var(--os-muted)]">{p.use}</p>
                </div>
                <ol className="divide-y divide-[var(--os-hair)]">
                  {p.steps.map((s, i) => {
                    const k = key(p.id, i);
                    const on = !!checked[k];
                    return (
                      <li key={i} className="flex items-start gap-3 px-5 py-3.5">
                        <button onClick={() => {
                            const nx = { ...checked, [k]: !on };
                            setChecked(nx);
                            const total = Array.from({ length: p.steps.length }, (_, j) => nx[key(p.id, j)]).filter(Boolean).length;
                            if (total === p.steps.length) recordAction("playbook.completed");
                          }}
                          aria-pressed={on} aria-label={`Step ${i + 1} ${on ? "done" : "not done"}`}
                          className={`mt-[2px] flex h-4.5 w-4.5 shrink-0 items-center justify-center rounded-[4px] border ${on ? "border-[var(--os-confirmed)] bg-[var(--os-confirmed)]" : "border-[var(--os-hair-2)] hover:border-[var(--os-accent)]"}`}>
                          {on && <Icon name="Check" size={11} className="text-[var(--os-ground)]" />}
                        </button>
                        <div className="min-w-0 flex-1">
                          <div className="flex flex-wrap items-baseline gap-2">
                            <span className="mono text-[10px] text-[var(--os-faint)]">{String(i + 1).padStart(2, "0")}</span>
                            <p className={`text-[13px] leading-[1.5] ${on ? "text-[var(--os-faint)] line-through" : "text-[var(--os-ink)]"}`}>{s.do}</p>
                          </div>
                          <p className="mt-1 text-[12px] leading-[1.5] text-[var(--os-muted)]">
                            <span className="mono text-[9.5px] uppercase tracking-[.12em] text-[var(--os-refuted)]">guards against</span>{" "}
                            {s.guards}
                          </p>
                          {s.where && (
                            <Link href={s.where} className="mono mt-1 inline-block text-[11px] text-[var(--os-accent)] hover:underline">{s.where}</Link>
                          )}
                        </div>
                      </li>
                    );
                  })}
                </ol>
                <div className="border-t border-[var(--os-hair)] px-5 py-4">
                  <span className="mono text-[9.5px] uppercase tracking-[.14em] text-[var(--os-confirmed)]">what you may conclude</span>
                  <p className="mt-1 max-w-[86ch] text-[13px] leading-[1.6] text-[var(--os-ink)]">{p.conclusion}</p>
                  {isBeginner && (
                    <div className="mt-3 flex flex-wrap items-center gap-1.5">
                      <span className="mono text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">concepts</span>
                      {p.teaches.map((k) => term(k) && <TermChip key={k} k={k} />)}
                    </div>
                  )}
                </div>
              </div>
            )}
          </Panel>
        );
      })}
    </div>
  );
}
