"use client";

/**
 * Beginner / Professional — progressive disclosure.
 *
 * Beginner expands guidance, shows concept chips and explains conventions.
 * Professional collapses all of it and surfaces raw controls (seeds, sweeps,
 * temperatures, provenance paths). Neither mode hides *data* — only scaffolding.
 */

import { useMode } from "@/lib/workflow/progress";
import { Icon } from "@/components/os/kit";

export default function ModeToggle() {
  const { mode, setMode } = useMode();
  return (
    <div role="radiogroup" aria-label="Operating mode"
      className="hidden items-center rounded-[9px] border border-[var(--os-hair-2)] bg-[var(--os-panel)] p-0.5 md:inline-flex">
      {([["beginner", "GraduationCap", "Beginner"], ["professional", "Wrench", "Pro"]] as const).map(([id, icon, label]) => (
        <button key={id} role="radio" aria-checked={mode === id} onClick={() => setMode(id)}
          title={id === "beginner" ? "Guidance expanded, concepts explained" : "Guidance collapsed, raw controls surfaced"}
          className={`mono flex items-center gap-1.5 rounded-[7px] px-2 py-1 text-[11px] transition-colors ${
            mode === id ? "bg-[var(--os-accent-soft)] text-[var(--os-ink)]" : "text-[var(--os-muted)] hover:text-[var(--os-ink)]"}`}>
          <Icon name={icon} size={12} /> <span className="hidden lg:inline">{label}</span>
        </button>
      ))}
    </div>
  );
}
