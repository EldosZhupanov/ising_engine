"use client";

/** Inline glossary term — contextual education where the concept is used. */

import { useState } from "react";
import { term } from "@/lib/workflow/glossary";
import { Icon } from "@/components/os/kit";

export default function TermChip({ k }: { k: string }) {
  const t = term(k);
  const [open, setOpen] = useState(false);
  if (!t) return null;
  return (
    <span className="relative inline-block">
      <button onClick={() => setOpen((v) => !v)} aria-expanded={open}
        className="mono rounded-full border border-[var(--os-hair-2)] bg-[var(--os-ground-2)] px-2 py-0.5 text-[10.5px] text-[var(--os-muted)] transition-colors hover:border-[var(--os-accent)] hover:text-[var(--os-ink)]">
        {t.term}
      </button>
      {open && (
        <>
          <button className="fixed inset-0 z-40 cursor-default" aria-hidden tabIndex={-1} onClick={() => setOpen(false)} />
          <span role="tooltip" className="os-glass absolute left-0 top-[calc(100%+6px)] z-50 block w-[300px] rounded-[10px] p-3.5 text-left">
            <span className="block text-[12.5px] font-semibold text-[var(--os-ink)]">{t.term}</span>
            <span className="mt-1 block text-[12px] leading-[1.55] text-[var(--os-muted)]">{t.short}</span>
            {t.why && (
              <span className="mt-2 block border-t border-[var(--os-hair)] pt-2 text-[11.5px] leading-[1.5] text-[var(--os-muted)]">
                <span className="mono text-[9.5px] uppercase tracking-[.14em] text-[var(--os-accent)]">why it matters</span>
                <span className="mt-0.5 block">{t.why}</span>
              </span>
            )}
            {t.where && (
              <span className="mono mt-2 flex items-center gap-1.5 text-[10px] text-[var(--os-faint)]">
                <Icon name="FileCode2" size={10} /> {t.where}
              </span>
            )}
          </span>
        </>
      )}
    </span>
  );
}
