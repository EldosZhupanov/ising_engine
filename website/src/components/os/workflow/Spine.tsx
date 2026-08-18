"use client";

/**
 * The workflow spine — where you are in the scientific process.
 *
 * Rendered on every workspace. It is the answer to "why does this screen exist":
 * each screen is one link in Problem → … → Applications, and the spine shows
 * which link you occupy and what the next act is.
 */

import Link from "next/link";
import { CHAIN } from "@/lib/workflow/chain";
import { Icon } from "@/components/os/kit";

export default function Spine({ route, activeId }: { route: string; activeId?: string }) {
  const activeIdx = activeId
    ? CHAIN.findIndex((l) => l.id === activeId)
    : CHAIN.findIndex((l) => l.route === route);
  const active = activeIdx >= 0 ? CHAIN[activeIdx] : null;
  const next = activeIdx >= 0 && activeIdx < CHAIN.length - 1 ? CHAIN[activeIdx + 1] : null;

  return (
    <nav aria-label="Scientific workflow position" className="mb-4">
      <ol className="flex flex-wrap items-center gap-x-1 gap-y-1.5">
        {CHAIN.map((l, i) => {
          const on = i === activeIdx;
          const past = activeIdx >= 0 && i < activeIdx;
          return (
            <li key={l.id} className="flex items-center gap-1">
              <Link href={l.route} title={l.act}
                aria-current={on ? "step" : undefined}
                className={`mono rounded-full border px-2 py-[3px] text-[10.5px] tracking-[.02em] transition-colors ${
                  on ? "border-[var(--os-accent)] bg-[var(--os-accent-soft)] text-[var(--os-ink)]"
                  : past ? "border-transparent text-[var(--os-muted)] hover:text-[var(--os-ink)]"
                  : "border-transparent text-[var(--os-faint)] hover:text-[var(--os-ink)]"}`}>
                {l.label}
              </Link>
              {i < CHAIN.length - 1 && <span className="text-[9px] text-[var(--os-hair-2)]">→</span>}
            </li>
          );
        })}
      </ol>
      {active && (
        <p className="mt-2 flex flex-wrap items-baseline gap-x-2 text-[12.5px] leading-[1.5] text-[var(--os-muted)]">
          <span className="mono text-[9.5px] uppercase tracking-[.14em] text-[var(--os-accent)]">this step</span>
          <span className="text-[var(--os-ink)]">{active.act}</span>
          {next && (
            <>
              <span className="mono text-[9.5px] uppercase tracking-[.14em] text-[var(--os-faint)]">then</span>
              <Link href={next.route} className="inline-flex items-center gap-1 text-[var(--os-accent)] hover:underline">
                {next.label} <Icon name="ArrowRight" size={10} />
              </Link>
            </>
          )}
        </p>
      )}
    </nav>
  );
}
