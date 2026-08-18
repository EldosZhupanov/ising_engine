"use client";

/**
 * Settings — everything that is genuinely local to this browser.
 *
 * Appearance, operating mode, the control-plane address, and mission progress all
 * live in localStorage; nothing here is sent anywhere. The panel says so, and
 * offers an honest reset for each.
 */

import { useState } from "react";
import { THEMES, useTheme } from "@/components/os/Theme";
import { useMode, useProgress } from "@/lib/workflow/progress";
import { useControl } from "@/lib/control/useControl";
import { DEFAULT_CONTROL_URL } from "@/lib/control/client";
import { Panel, PanelHeader, Pill, StatusBadge, Icon } from "@/components/os/kit";

export default function Settings() {
  const { choice, setTheme } = useTheme();
  const { mode, setMode } = useMode();
  const { url, setUrl, status, health, recheck } = useControl();
  const { completedCount, totalCount, availableCompleted, availableCount, reset } = useProgress();
  const [draft, setDraft] = useState(url);
  const [confirmReset, setConfirmReset] = useState(false);

  return (
    <div className="space-y-4">
      <Panel>
        <PanelHeader title="Appearance" status="real" sub="handcrafted themes — print uses a dedicated ink-on-paper set" />
        <div className="grid gap-2 p-5 sm:grid-cols-2 lg:grid-cols-3">
          {THEMES.map((t) => (
            <button key={t.id} onClick={() => setTheme(t.id)} aria-pressed={choice === t.id}
              className={`flex items-start gap-2.5 rounded-[10px] border px-3.5 py-3 text-left transition-colors ${
                choice === t.id ? "border-[var(--os-accent)] bg-[var(--os-accent-soft)]" : "border-[var(--os-hair-2)] hover:border-[var(--os-faint)]"}`}>
              <Icon name={t.icon} size={15} className={choice === t.id ? "text-[var(--os-accent)]" : "text-[var(--os-muted)]"} />
              <span className="min-w-0">
                <span className="block text-[12.5px] text-[var(--os-ink)]">{t.label}</span>
                <span className="mono block text-[10px] leading-[1.4] text-[var(--os-faint)]">{t.hint}</span>
              </span>
            </button>
          ))}
        </div>
      </Panel>

      <Panel>
        <PanelHeader title="Operating mode" status="real" sub="progressive disclosure — neither mode hides data, only scaffolding" />
        <div className="grid gap-2 p-5 sm:grid-cols-2">
          {([
            ["beginner", "Beginner", "Guidance expanded, concepts explained inline, the 'why' shown on every mission."],
            ["professional", "Professional", "Guidance collapsed to one line, raw controls and provenance paths surfaced."],
          ] as const).map(([id, label, hint]) => (
            <button key={id} onClick={() => setMode(id)} aria-pressed={mode === id}
              className={`rounded-[10px] border px-4 py-3 text-left transition-colors ${
                mode === id ? "border-[var(--os-accent)] bg-[var(--os-accent-soft)]" : "border-[var(--os-hair-2)] hover:border-[var(--os-faint)]"}`}>
              <span className="block text-[13px] text-[var(--os-ink)]">{label}</span>
              <span className="mt-0.5 block text-[12px] leading-[1.5] text-[var(--os-muted)]">{hint}</span>
            </button>
          ))}
        </div>
      </Panel>

      <Panel>
        <PanelHeader title="Control plane" status={status === "online" ? "real" : "offline"}
          sub="unlocks replay, host telemetry and campaign control"
          right={<button onClick={recheck} className="mono rounded-[7px] border border-[var(--os-hair-2)] px-2.5 py-1 text-[11px] text-[var(--os-muted)] hover:text-[var(--os-ink)]">re-check</button>} />
        <div className="space-y-3 p-5">
          <div className="flex items-center gap-2">
            <StatusBadge status={status === "online" ? "real" : "offline"} />
            {health && <span className="mono text-[11px] text-[var(--os-faint)]">{health.service} · dir {health.dir}</span>}
          </div>
          <label className="block">
            <span className="mono text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">address</span>
            <span className="mt-1 flex gap-2">
              <input value={draft} onChange={(e) => setDraft(e.target.value)}
                className="mono flex-1 rounded-[8px] border border-[var(--os-hair-2)] bg-[var(--os-ground)] px-3 py-2 text-[12.5px] text-[var(--os-ink)] focus:border-[var(--os-accent)] focus:outline-none" />
              <button onClick={() => setUrl(draft.trim() || DEFAULT_CONTROL_URL)}
                className="mono rounded-[8px] border border-[var(--os-hair-2)] px-3 py-2 text-[12px] text-[var(--os-muted)] hover:text-[var(--os-ink)]">apply</button>
            </span>
          </label>
          {status !== "online" && (
            <pre className="mono overflow-x-auto rounded-[8px] border border-[var(--os-hair)] bg-[var(--os-ground)] p-3 text-[11px] text-[var(--os-ink)]">cargo run --release --bin control_api</pre>
          )}
          <p className="mono text-[10.5px] leading-[1.5] text-[var(--os-faint)]">
            Bound to localhost by the server itself. Stored in this browser only.
          </p>
        </div>
      </Panel>

      <Panel>
        <PanelHeader title="Research progress" status="real" sub="missions completed by real actions, stored locally" />
        <div className="p-5">
          <div className="flex flex-wrap items-baseline gap-x-4 gap-y-1">
            <span className="mono text-[1.6rem] font-semibold leading-none tnum text-[var(--os-accent)]">
              {availableCompleted}<span className="text-[0.55em] text-[var(--os-faint)]"> / {availableCount}</span>
            </span>
            <span className="mono text-[11px] uppercase tracking-[.1em] text-[var(--os-muted)]">completable today</span>
            <span className="mono text-[11px] text-[var(--os-faint)]">{completedCount} of {totalCount} total steps</span>
          </div>
          <div className="mt-4 flex items-center gap-2">
            {confirmReset ? (
              <>
                <span className="mono text-[11.5px] text-[var(--os-warn)]">This clears your mission history. Sure?</span>
                <button onClick={() => { reset(); setConfirmReset(false); }}
                  className="mono rounded-[7px] border border-[var(--os-refuted)] px-2.5 py-1 text-[11px] text-[var(--os-refuted)]">yes, reset</button>
                <button onClick={() => setConfirmReset(false)}
                  className="mono rounded-[7px] border border-[var(--os-hair-2)] px-2.5 py-1 text-[11px] text-[var(--os-muted)]">cancel</button>
              </>
            ) : (
              <button onClick={() => setConfirmReset(true)}
                className="mono rounded-[7px] border border-[var(--os-hair-2)] px-2.5 py-1 text-[11px] text-[var(--os-muted)] hover:text-[var(--os-ink)]">reset progress</button>
            )}
          </div>
        </div>
      </Panel>

      <Panel className="p-5">
        <div className="mb-2"><Pill tone="neutral">what is stored, and where</Pill></div>
        <ul className="space-y-1.5 text-[12.5px] text-[var(--os-muted)]">
          {[
            ["ising.theme", "appearance choice"],
            ["ising.mode", "beginner / professional"],
            ["ising.controlUrl", "control-plane address"],
            ["ising.progress.v1", "mission progress (actions you actually performed)"],
            ["ising.llm.key.*", "API keys you enter for remote LLM providers — never sent anywhere but that provider"],
            ["ising.llm.host.*", "per-provider host overrides"],
          ].map(([k, v]) => (
            <li key={k} className="flex flex-wrap items-baseline gap-2">
              <span className="mono text-[11.5px] text-[var(--os-ink)]">{k}</span>
              <span className="text-[12px]">— {v}</span>
            </li>
          ))}
        </ul>
        <p className="mono mt-3 border-t border-[var(--os-hair)] pt-2.5 text-[10.5px] leading-[1.5] text-[var(--os-faint)]">
          All of it is browser localStorage. This interface has no accounts, no analytics and no server-side session.
        </p>
      </Panel>
    </div>
  );
}
