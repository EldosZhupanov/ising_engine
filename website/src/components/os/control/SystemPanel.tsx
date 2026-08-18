"use client";

/**
 * Host telemetry — the panel the browser could not fill on its own.
 *
 * GPU utilisation, total VRAM, CPU threads, RAM and load average require
 * shelling out (`nvidia-smi`, `/proc`), which a page cannot do. With the control
 * plane running these become real; without it the panel says so plainly and
 * shows nothing invented. If the host reports no GPU, the panel says that too
 * rather than implying one exists.
 */

import { useCallback, useEffect, useState } from "react";
import { getSystem, fmtKb, fmtMib, type SystemInfo } from "@/lib/control/client";
import { useControl } from "@/lib/control/useControl";
import { Panel, PanelHeader, StatusBadge, Icon } from "@/components/os/kit";

export default function SystemPanel() {
  const { online, url } = useControl();
  const [sys, setSys] = useState<SystemInfo | null>(null);
  const [err, setErr] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!online) { setSys(null); return; }
    try { setSys(await getSystem(url)); setErr(null); }
    catch (e) { setErr(e instanceof Error ? e.message : String(e)); setSys(null); }
  }, [online, url]);

  // setState happens in an async callback, never synchronously in the effect body.
  useEffect(() => {
    let alive = true;
    const tick = async () => { if (alive) await load(); };
    void tick();
    const id = setInterval(() => { void tick(); }, 5000);
    return () => { alive = false; clearInterval(id); };
  }, [load]);

  const memUsedPct = sys?.mem_total_kb && sys?.mem_available_kb
    ? ((sys.mem_total_kb - sys.mem_available_kb) / sys.mem_total_kb) * 100
    : null;

  return (
    <Panel>
      <PanelHeader
        title="Host telemetry"
        status={online ? "real" : "offline"}
        sub={online ? "control_api /api/system — measured on this machine" : "requires the control plane"}
        right={online ? (
          <button onClick={() => void load()} className="mono rounded-[7px] border border-[var(--os-hair-2)] px-2.5 py-1 text-[11px] text-[var(--os-muted)] hover:text-[var(--os-ink)]">refresh</button>
        ) : undefined}
      />
      <div className="p-5">
        {!online ? (
          <div className="space-y-2.5">
            <StatusBadge status="offline" />
            <p className="max-w-[64ch] text-[13px] leading-[1.6] text-[var(--os-muted)]">
              GPU utilisation, total VRAM, CPU and RAM cannot be read from a browser — they need a process that can
              shell out. Start the control plane and this becomes live:
            </p>
            <pre className="mono overflow-x-auto rounded-[8px] border border-[var(--os-hair)] bg-[var(--os-ground)] p-3 text-[11px] text-[var(--os-ink)]">cargo run --release --bin control_api</pre>
            <p className="mono text-[10.5px] text-[var(--os-faint)]">expected at {url} · nothing is estimated in the meantime</p>
          </div>
        ) : err ? (
          <p className="mono text-[12px] text-[var(--os-refuted)]">{err}</p>
        ) : !sys ? (
          <p className="mono text-[12px] text-[var(--os-faint)]">reading…</p>
        ) : (
          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
              <Stat label="CPU threads" value={String(sys.cpu_threads)} />
              <Stat label="RAM total" value={fmtKb(sys.mem_total_kb)} />
              <Stat label="RAM available" value={fmtKb(sys.mem_available_kb)} accent />
              <Stat label="Load (1m)" value={sys.load_avg_1m?.toFixed(2) ?? "—"} />
            </div>

            {memUsedPct !== null && (
              <div>
                <div className="mono mb-1 flex justify-between text-[10px] uppercase tracking-[.1em] text-[var(--os-faint)]">
                  <span>memory in use</span><span>{memUsedPct.toFixed(0)}%</span>
                </div>
                <div className="h-1.5 overflow-hidden rounded-full bg-[var(--os-raised)]">
                  <div className="h-full rounded-full bg-[var(--os-cyan)]" style={{ width: `${memUsedPct}%` }} />
                </div>
              </div>
            )}

            <div className="border-t border-[var(--os-hair)] pt-3.5">
              <div className="mono mb-2 flex items-center gap-2 text-[10px] uppercase tracking-[.14em] text-[var(--os-muted)]">
                <Icon name="Cpu" size={12} /> GPU
              </div>
              {sys.gpus.length === 0 ? (
                <p className="mono text-[11.5px] text-[var(--os-faint)]">
                  no GPU reported by nvidia-smi on this host — not assumed, not estimated
                </p>
              ) : sys.gpus.map((g, i) => {
                const pct = g.memory_total_mib ? (g.memory_used_mib / g.memory_total_mib) * 100 : 0;
                return (
                  <div key={i} className="mb-3 last:mb-0">
                    <div className="flex flex-wrap items-baseline justify-between gap-2">
                      <span className="mono text-[12.5px] text-[var(--os-ink)]">{g.name}</span>
                      <span className="mono text-[11px] text-[var(--os-muted)] tnum">
                        {fmtMib(g.memory_used_mib)} / {fmtMib(g.memory_total_mib)} · {g.utilization_pct}% util
                      </span>
                    </div>
                    <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-[var(--os-raised)]">
                      <div className="h-full rounded-full bg-[var(--os-accent)] transition-[width] duration-500" style={{ width: `${pct}%` }} />
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </Panel>
  );
}

function Stat({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div>
      <div className={`mono text-[1.25rem] font-semibold leading-none tnum ${accent ? "text-[var(--os-accent)]" : "text-[var(--os-ink)]"}`}>{value}</div>
      <div className="mono mt-1.5 text-[10px] uppercase tracking-[.08em] text-[var(--os-faint)]">{label}</div>
    </div>
  );
}
