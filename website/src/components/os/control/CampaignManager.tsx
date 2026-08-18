"use client";

/**
 * Autonomous Scientist — mission control.
 *
 * One button starts the loop; from then on you watch it think. Because the
 * control plane owns the orchestrator in-process (AD-1) rather than shelling out
 * to the CLI, three things are possible that a spawned process could not offer:
 * **pause** between ticks, **step** exactly one tick, and a structured report
 * after each one.
 *
 * Every figure shown is from a completed tick. Nothing is projected, smoothed or
 * estimated while a tick is in flight.
 */

import { useCallback, useEffect, useState } from "react";
import {
  getCampaign, startCampaign, campaignAction,
  type CampaignStatus, type Phase,
} from "@/lib/control/client";
import { useControl } from "@/lib/control/useControl";
import { Icon, Panel, PanelHeader, Pill, StatusBadge } from "@/components/os/kit";

const PHASE_TONE: Record<Phase, "confirmed" | "refuted" | "open" | "neutral"> = {
  idle: "neutral", running: "confirmed", paused: "open", finished: "neutral", failed: "refuted",
};
const PHASE_TEXT: Record<Phase, string> = {
  idle: "idle — nothing started",
  running: "running — ticking autonomously",
  paused: "paused — between ticks",
  finished: "finished — budget reached or stopped",
  failed: "failed — a tick returned an error",
};

export default function CampaignManager({ instances }: { instances: string[] }) {
  const { online, url } = useControl();
  const [st, setSt] = useState<CampaignStatus | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);

  // form
  const [picked, setPicked] = useState<string[]>(instances.slice(0, 1));
  const [maxTicks, setMaxTicks] = useState(3);
  const [generations, setGenerations] = useState(1);
  const [planner, setPlanner] = useState(false);
  const [executive, setExecutive] = useState(false);
  const [shared, setShared] = useState(false);
  const [curiosity, setCuriosity] = useState(0);
  const [startPaused, setStartPaused] = useState(true);

  const refresh = useCallback(async () => {
    if (!online) { setSt(null); return; }
    try { setSt(await getCampaign(url)); } catch { setSt(null); }
  }, [online, url]);

  // Poll faster while a tick is in flight so the log feels live.
  useEffect(() => {
    let alive = true;
    const tick = async () => { if (alive) await refresh(); };
    void tick();
    const id = setInterval(() => { void tick(); }, 2000);
    return () => { alive = false; clearInterval(id); };
  }, [refresh]);

  const act = async (label: string, fn: () => Promise<CampaignStatus>) => {
    setBusy(label); setErr(null);
    try { setSt(await fn()); }
    catch (e) { setErr(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(null); void refresh(); }
  };

  const phase = st?.phase ?? "idle";
  const active = phase === "running" || phase === "paused";

  if (!online) {
    return (
      <Panel className="p-6">
        <div className="mb-3"><StatusBadge status="offline" /></div>
        <p className="max-w-[70ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
          The autonomous scientist runs inside the control plane, which owns the orchestrator and drives its
          <span className="mono text-[var(--os-ink)]"> tick()</span> loop under supervision. Start it and this becomes a
          real mission control — start, pause, <em>step</em>, resume, stop:
        </p>
        <pre className="mono mt-3 overflow-x-auto rounded-[8px] border border-[var(--os-hair)] bg-[var(--os-ground)] p-3 text-[11.5px] text-[var(--os-ink)]">cargo run --release --bin control_api</pre>
        <p className="mono mt-2 text-[10.5px] text-[var(--os-faint)]">expected at {url}</p>
      </Panel>
    );
  }

  return (
    <div className="space-y-4">
      {/* ── status ── */}
      <Panel ticks>
        <PanelHeader title="Autonomous Scientist" status="real"
          sub={st ? `${st.dir}` : "control plane connected"}
          right={<Pill tone={PHASE_TONE[phase]}>{phase}</Pill>} />
        <div className="grid gap-5 p-5 lg:grid-cols-[1fr_auto]">
          <div>
            <p className="text-[13px] text-[var(--os-muted)]">{PHASE_TEXT[phase]}</p>
            {st && active && (
              <div className="mt-3 flex flex-wrap items-baseline gap-x-5 gap-y-1.5">
                <span className="mono text-[2rem] font-semibold leading-none tnum text-[var(--os-accent)]">
                  {st.ticks_done}
                  <span className="text-[0.5em] text-[var(--os-faint)]">{st.max_ticks ? ` / ${st.max_ticks}` : " / ∞"}</span>
                </span>
                <span className="mono text-[11px] uppercase tracking-[.1em] text-[var(--os-muted)]">ticks completed</span>
                {st.next_instance && (
                  <span className="mono text-[11.5px] text-[var(--os-faint)]">next: <span className="text-[var(--os-ink)]">{st.next_instance}</span></span>
                )}
                {(st.planner_driven || st.executive_driven) && (
                  <span className="mono text-[10.5px] text-[var(--os-faint)]">
                    {st.planner_driven && "planner-driven "}{st.executive_driven && "executive-driven"}
                  </span>
                )}
              </div>
            )}
            {st?.last_error && (
              <p className="mono mt-3 rounded-[8px] border border-[color:rgba(240,113,90,.35)] bg-[rgba(240,113,90,.06)] px-3 py-2 text-[11.5px] text-[var(--os-refuted)]">
                {st.last_error}
              </p>
            )}
            {err && <p className="mono mt-3 text-[11.5px] text-[var(--os-warn)]">{err}</p>}
          </div>

          {/* ── controls ── */}
          <div className="flex flex-wrap items-start gap-2">
            {phase === "running" && (
              <Btn label="pause" icon="Pause" busy={busy === "pause"} onClick={() => act("pause", () => campaignAction(url, "pause"))} />
            )}
            {phase === "paused" && (
              <>
                <Btn label="resume" icon="Play" primary busy={busy === "resume"} onClick={() => act("resume", () => campaignAction(url, "resume"))} />
                <Btn label="step 1 tick" icon="StepForward" busy={busy === "step"} onClick={() => act("step", () => campaignAction(url, "step"))} />
              </>
            )}
            {active && (
              <Btn label="stop" icon="Square" danger busy={busy === "stop"} onClick={() => act("stop", () => campaignAction(url, "stop"))} />
            )}
          </div>
        </div>
      </Panel>

      {/* ── launcher ── */}
      {!active && (
        <Panel>
          <PanelHeader title="Start a campaign" status="real"
            sub="the loop appends to the record — one campaign per directory" />
          <div className="grid gap-5 p-5 lg:grid-cols-[1.1fr_1fr]">
            <div className="space-y-3.5">
              <div>
                <span className="mono text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">
                  instances <span className="text-[var(--os-hair-2)]">({picked.length} selected)</span>
                </span>
                <div className="mt-1.5 flex max-h-[132px] flex-wrap gap-1.5 overflow-y-auto">
                  {instances.map((i) => {
                    const on = picked.includes(i);
                    return (
                      <button key={i} onClick={() => setPicked((p) => on ? p.filter((x) => x !== i) : [...p, i])}
                        aria-pressed={on}
                        className={`mono rounded-[6px] border px-2 py-1 text-[11px] ${on ? "border-[var(--os-accent)] bg-[var(--os-accent-soft)] text-[var(--os-ink)]" : "border-[var(--os-hair-2)] text-[var(--os-muted)] hover:text-[var(--os-ink)]"}`}>
                        {i}
                      </button>
                    );
                  })}
                </div>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <Num label="max ticks" hint="0 = until stopped" value={maxTicks} set={setMaxTicks} min={0} max={500} />
                <Num label="generations / tick" value={generations} set={setGenerations} min={1} max={20} />
                <Num label="curiosity λ" hint="0 = pure optimiser" value={curiosity} set={setCuriosity} min={0} max={1} step={0.05} />
              </div>
            </div>

            <div className="space-y-2.5">
              <Check on={startPaused} set={setStartPaused} label="start paused"
                hint="lets you step tick-by-tick — a debugger for science" />
              <Check on={planner} set={setPlanner} label="planner-driven"
                hint="the loop chooses its own next instance by expected new knowledge" />
              <Check on={executive} set={setExecutive} label="executive-driven"
                hint="obey the Chief Scientist: retrain stale models, investigate flagged theories" />
              <Check on={shared} set={setShared} label="shared knowledge"
                hint="Meta-Learning Layer: cross-model consensus biases ideation" />
              <button
                disabled={picked.length === 0 || !!busy}
                onClick={() => act("start", () => startCampaign(url, {
                  instances: picked, max_ticks: maxTicks, generations,
                  planner_driven: planner, executive_driven: executive,
                  shared_knowledge: shared, curiosity_lambda: curiosity,
                  start_paused: startPaused,
                }))}
                className="mono mt-1 flex w-full items-center justify-center gap-2 rounded-[9px] bg-[var(--os-accent)] px-4 py-2.5 text-[13px] font-medium text-[var(--os-on-accent)] disabled:opacity-40">
                <Icon name="Play" size={14} />
                {busy === "start" ? "starting…" : "START AUTONOMOUS SCIENTIST"}
              </button>
              <p className="mono text-[10px] leading-[1.5] text-[var(--os-faint)]">
                Each tick runs observe → learn → discover → theorize → plan → run → refute → record and appends
                provenance-stamped experiments. Budget caps are what make leaving it running safe.
              </p>
            </div>
          </div>
        </Panel>
      )}

      {/* ── tick log ── */}
      <Panel>
        <PanelHeader title="Tick log" status="real"
          sub={st?.events.length ? `${st.events.length} completed tick(s), newest first` : "no ticks yet"} />
        {!st?.events.length ? (
          <div className="px-5 py-8 text-center text-[13px] text-[var(--os-faint)]">
            Nothing recorded yet. Each completed tick appears here with what it actually did.
          </div>
        ) : (
          <div className="divide-y divide-[var(--os-hair)]">
            {st.events.map((e) => {
              const gained = e.experiments_after - e.experiments_before;
              const beatBaseline = e.best_score < e.baseline;
              return (
                <div key={e.tick} className="px-5 py-3.5">
                  <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
                    <span className="mono text-[11px] text-[var(--os-accent)]">tick {e.tick}</span>
                    <span className="mono text-[12.5px] text-[var(--os-ink)]">{e.instance}</span>
                    <span className="mono text-[11px] text-[var(--os-muted)] tnum">
                      +{gained.toLocaleString()} experiments
                    </span>
                    <span className={`mono text-[11px] tnum ${beatBaseline ? "text-[var(--os-confirmed)]" : "text-[var(--os-muted)]"}`}>
                      best {e.best_score.toLocaleString()} vs baseline {e.baseline.toLocaleString()}
                    </span>
                    {e.theories_published > 0 && <Pill tone="confirmed">+{e.theories_published} theories</Pill>}
                    {e.concepts.length > 0 && <Pill tone="open">+{e.concepts.length} concepts</Pill>}
                    <span className="mono ml-auto text-[10.5px] text-[var(--os-faint)] tnum">{(e.wall_ms / 1000).toFixed(1)}s</span>
                  </div>
                  {e.best_schedule.length > 0 && (
                    <div className="mono mt-1.5 text-[11.5px] text-[var(--os-muted)]">
                      [{e.best_schedule.join(" → ")}]
                    </div>
                  )}
                  <div className="mono mt-1 flex flex-wrap gap-x-4 gap-y-0.5 text-[10.5px] text-[var(--os-faint)]">
                    {e.health && <span className={e.health.startsWith("HEALTH OK") ? "text-[var(--os-confirmed)]" : "text-[var(--os-warn)]"}>{e.health}</span>}
                    {e.directives.slice(0, 2).map((d, i) => <span key={i}>· {d}</span>)}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </Panel>
    </div>
  );
}

function Btn({ label, icon, onClick, busy, primary, danger }:
  { label: string; icon: string; onClick: () => void; busy?: boolean; primary?: boolean; danger?: boolean }) {
  return (
    <button onClick={onClick} disabled={busy}
      className={`mono inline-flex items-center gap-1.5 rounded-[8px] border px-3 py-2 text-[12px] transition-colors disabled:opacity-50 ${
        primary ? "border-[var(--os-accent)] bg-[var(--os-accent)] text-[var(--os-on-accent)]"
        : danger ? "border-[var(--os-hair-2)] text-[var(--os-muted)] hover:border-[var(--os-refuted)] hover:text-[var(--os-refuted)]"
        : "border-[var(--os-hair-2)] text-[var(--os-muted)] hover:text-[var(--os-ink)]"}`}>
      <Icon name={icon} size={13} /> {busy ? "…" : label}
    </button>
  );
}

function Num({ label, hint, value, set, min, max, step = 1 }:
  { label: string; hint?: string; value: number; set: (n: number) => void; min: number; max: number; step?: number }) {
  return (
    <label className="block">
      <span className="mono text-[10px] uppercase tracking-[.1em] text-[var(--os-faint)]">{label}</span>
      {hint && <span className="mono ml-1.5 text-[9.5px] text-[var(--os-hair-2)]">{hint}</span>}
      <input type="number" value={value} min={min} max={max} step={step}
        onChange={(e) => set(Number(e.target.value))}
        className="mono mt-1 w-full rounded-[8px] border border-[var(--os-hair-2)] bg-[var(--os-ground)] px-2.5 py-1.5 text-[12px] tnum text-[var(--os-ink)] focus:border-[var(--os-accent)] focus:outline-none" />
    </label>
  );
}

function Check({ on, set, label, hint }: { on: boolean; set: (b: boolean) => void; label: string; hint: string }) {
  return (
    <button onClick={() => set(!on)} aria-pressed={on} className="flex w-full items-start gap-2.5 text-left">
      <span className={`mt-[2px] flex h-4 w-4 shrink-0 items-center justify-center rounded-[4px] border ${on ? "border-[var(--os-confirmed)] bg-[var(--os-confirmed)]" : "border-[var(--os-hair-2)]"}`}>
        {on && <Icon name="Check" size={11} className="text-[var(--os-ground)]" />}
      </span>
      <span className="min-w-0">
        <span className="mono block text-[12px] text-[var(--os-ink)]">{label}</span>
        <span className="block text-[11px] leading-[1.45] text-[var(--os-faint)]">{hint}</span>
      </span>
    </button>
  );
}
