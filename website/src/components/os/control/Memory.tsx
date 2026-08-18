"use client";

/**
 * Scientific Memory — the first stage of the research loop, made observable.
 *
 * Before the platform decides anything it asks: *have I seen a structure like
 * this before?* `memory_os.rs` answers that by scanning the entire append-only
 * record for instances within a feature-distance radius, ranking what worked on
 * them, and collecting the conditional facts whose condition applies here.
 *
 * That computation is live — it runs against the DB as it is now, not against
 * the build-time snapshot the rest of the Observatory renders. It therefore
 * requires the control plane, and says so plainly when it is absent.
 */

import { useCallback, useEffect, useState } from "react";
import {
  getMemoryReport, getMemoryRecall,
  type MemoryReport, type Recollection,
} from "@/lib/control/client";
import { useControl } from "@/lib/control/useControl";
import { Panel, PanelHeader, Pill, StatusBadge, Icon } from "@/components/os/kit";

export default function Memory({ instances }: { instances: string[] }) {
  const { online, url } = useControl();
  const [report, setReport] = useState<MemoryReport | null>(null);
  const [rec, setRec] = useState<Recollection | null>(null);
  const [instance, setInstance] = useState(instances[0] ?? "G22");
  const [radius, setRadius] = useState(0.35);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const loadReport = useCallback(async () => {
    if (!online) { setReport(null); return; }
    try { setReport(await getMemoryReport(url)); } catch { setReport(null); }
  }, [online, url]);

  useEffect(() => {
    let alive = true;
    const tick = async () => { if (alive) await loadReport(); };
    void tick();
    return () => { alive = false; };
  }, [loadReport]);

  const doRecall = async () => {
    setBusy(true); setErr(null); setRec(null);
    try { setRec(await getMemoryRecall(url, instance, radius)); }
    catch (e) { setErr(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };

  if (!online) {
    return (
      <Panel className="p-6">
        <div className="mb-3"><StatusBadge status="offline" /></div>
        <p className="max-w-[74ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
          Recall is <em>computed</em>, not stored: <span className="mono text-[var(--os-ink)]">memory_os.rs</span> scans the
          whole append-only record on demand and never serialises the result. There is therefore nothing honest to show
          from a snapshot — this workspace needs the engine running:
        </p>
        <pre className="mono mt-3 overflow-x-auto rounded-[8px] border border-[var(--os-hair)] bg-[var(--os-ground)] p-3 text-[11.5px] text-[var(--os-ink)]">cargo run --release --bin control_api</pre>
        <p className="mono mt-2 text-[10.5px] text-[var(--os-faint)]">expected at {url} · no cached recall is shown in the meantime</p>
      </Panel>
    );
  }

  const maxImpr = Math.max(...(rec?.best_operators ?? []).map((o) => Math.abs(o.mean_improvement)), 1e-9);

  return (
    <div className="space-y-4">
      {/* ── recall ── */}
      <Panel ticks>
        <PanelHeader title="Recall a structure" status="real"
          sub="memory_os::recall — live scan of the append-only record + knowledge graph" />
        <div className="grid gap-5 p-5 lg:grid-cols-[300px_1fr]">
          <div className="space-y-3.5">
            <label className="block">
              <span className="mono text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">instance</span>
              <select value={instance} onChange={(e) => setInstance(e.target.value)}
                className="mono mt-1 w-full rounded-[8px] border border-[var(--os-hair-2)] bg-[var(--os-ground)] px-2.5 py-2 text-[12.5px] text-[var(--os-ink)] focus:border-[var(--os-accent)] focus:outline-none">
                {instances.map((i) => <option key={i} value={i}>{i}</option>)}
              </select>
            </label>
            <label className="block">
              <span className="mono flex justify-between text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">
                <span>similarity radius</span><span className="text-[var(--os-muted)]">{radius.toFixed(2)}</span>
              </span>
              <input type="range" min={0.05} max={2} step={0.05} value={radius}
                onChange={(e) => setRadius(Number(e.target.value))}
                className="mt-2 w-full accent-[var(--os-accent)]" />
              <span className="mono mt-1 block text-[10px] leading-[1.45] text-[var(--os-faint)]">
                feature-distance over (n, density, clustering, mean degree, degree CV). Wider radius = more instances
                count as &ldquo;similar&rdquo;, and the recall gets less specific.
              </span>
            </label>
            <button onClick={() => void doRecall()} disabled={busy}
              className="mono flex w-full items-center justify-center gap-2 rounded-[9px] bg-[var(--os-accent)] px-4 py-2.5 text-[13px] font-medium text-[var(--os-on-accent)] disabled:opacity-40">
              <Icon name="Brain" size={14} /> {busy ? "scanning the record…" : "recall"}
            </button>
            {err && <p className="mono text-[11.5px] text-[var(--os-refuted)]">{err}</p>}
          </div>

          <div>
            {!rec && !err && (
              <p className="text-[13px] leading-[1.6] text-[var(--os-muted)]">
                This is the <span className="text-[var(--os-ink)]">observe</span> stage of the research loop — the first
                thing the orchestrator does on every tick. Pick an instance and ask the platform what it already knows
                about structures like it.
              </p>
            )}
            {rec && (
              <div className="space-y-4">
                <div className="rounded-[10px] border border-[color:rgba(240,132,46,.28)] bg-[var(--os-accent-soft)] px-4 py-3">
                  <div className="mono mb-1 text-[10px] uppercase tracking-[.14em] text-[var(--os-accent)]">narrative</div>
                  <p className="text-[13.5px] leading-[1.6] text-[var(--os-ink)]">{rec.narrative}</p>
                </div>

                <div className="grid gap-3 sm:grid-cols-2">
                  <div>
                    <div className="mono mb-1.5 text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">
                      structural signature
                    </div>
                    <div className="mono space-y-0.5 text-[11.5px] text-[var(--os-muted)]">
                      <div>n <span className="text-[var(--os-ink)] tnum">{rec.signature.n.toLocaleString()}</span></div>
                      <div>density <span className="text-[var(--os-ink)] tnum">{rec.signature.density.toFixed(4)}</span></div>
                      <div>clustering <span className="text-[var(--os-ink)] tnum">{rec.signature.clustering.toFixed(4)}</span></div>
                      <div>mean degree <span className="text-[var(--os-ink)] tnum">{rec.signature.mean_degree.toFixed(2)}</span></div>
                      <div>degree CV <span className="text-[var(--os-ink)] tnum">{rec.signature.degree_cv.toFixed(4)}</span></div>
                    </div>
                  </div>
                  <div>
                    <div className="mono mb-1.5 text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">
                      similar instances ({rec.similar_instances.length})
                    </div>
                    <div className="flex flex-wrap gap-1.5">
                      {rec.similar_instances.map((i) => (
                        <span key={i} className={`mono rounded-[6px] border px-2 py-0.5 text-[11px] ${i === rec.instance ? "border-[var(--os-accent)] text-[var(--os-accent)]" : "border-[var(--os-hair-2)] text-[var(--os-muted)]"}`}>{i}</span>
                      ))}
                      {rec.similar_instances.length === 0 && (
                        <span className="mono text-[11px] text-[var(--os-faint)]">none within this radius — try widening it</span>
                      )}
                    </div>
                  </div>
                </div>

                {rec.best_operators.length > 0 && (
                  <div>
                    <div className="mono mb-2 text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">
                      what worked there · mean relative improvement (negative is better)
                    </div>
                    <div className="space-y-1">
                      {rec.best_operators.slice(0, 8).map((o) => (
                        <div key={o.operator} className="grid grid-cols-[170px_1fr_74px] items-center gap-3">
                          <span className="mono truncate text-[11.5px] text-[var(--os-muted)]">{o.operator}</span>
                          <span className="h-1.5 overflow-hidden rounded-full bg-[var(--os-raised)]">
                            <span className="block h-full rounded-full bg-[var(--os-cyan)]"
                              style={{ width: `${Math.max(2, (Math.abs(o.mean_improvement) / maxImpr) * 100)}%` }} />
                          </span>
                          <span className="mono text-right text-[11px] tnum text-[var(--os-ink)]">
                            {o.mean_improvement >= 0 ? "+" : ""}{(o.mean_improvement * 100).toFixed(2)}%
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {rec.applicable_facts.length > 0 && (
                  <div>
                    <div className="mono mb-1.5 text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">
                      conditional facts that apply here ({rec.applicable_facts.length})
                    </div>
                    <ul className="space-y-1">
                      {rec.applicable_facts.map((f, i) => (
                        <li key={i} className="mono text-[11.5px] leading-[1.5] text-[var(--os-muted)]">· {f}</li>
                      ))}
                    </ul>
                  </div>
                )}

                <p className="mono border-t border-[var(--os-hair)] pt-2.5 text-[10px] leading-[1.5] text-[var(--os-faint)]">
                  {rec.computed_from} — computed now, not read from the build-time snapshot.
                </p>
              </div>
            )}
          </div>
        </div>
      </Panel>

      {/* ── retention report ── */}
      <Panel>
        <PanelHeader title="Retention report" status="real"
          sub="MemoryManager::analyze — structural regimes and how well characterised each is"
          right={<button onClick={() => void loadReport()} className="mono rounded-[7px] border border-[var(--os-hair-2)] px-2.5 py-1 text-[11px] text-[var(--os-muted)] hover:text-[var(--os-ink)]">refresh</button>} />
        {!report ? (
          <div className="px-5 py-6 text-[13px] text-[var(--os-faint)]">reading the store…</div>
        ) : (
          <>
            <div className="flex flex-wrap items-baseline gap-x-6 gap-y-2 border-b border-[var(--os-hair)] px-5 py-4">
              <span className="mono text-[1.6rem] font-semibold leading-none tnum text-[var(--os-accent)]">
                {report.total_experiments.toLocaleString()}
              </span>
              <span className="mono text-[11px] uppercase tracking-[.1em] text-[var(--os-muted)]">experiments retained</span>
              <span className="mono text-[11.5px] text-[var(--os-faint)]">{report.distinct_instances} distinct instances · {report.buckets.length} structural regimes</span>
            </div>
            <div className="divide-y divide-[var(--os-hair)]">
              {report.buckets.map((b) => (
                <div key={b.label} className="flex flex-wrap items-center gap-x-4 gap-y-1.5 px-5 py-3">
                  <span className="mono w-[110px] text-[12.5px] text-[var(--os-ink)]">{b.label}</span>
                  <span className="mono text-[11.5px] tnum text-[var(--os-muted)]">{b.experiments.toLocaleString()} exp · {b.instances} inst</span>
                  {b.dominant_operator && (
                    <span className="mono text-[11px] text-[var(--os-faint)]">dominant: <span className="text-[var(--os-muted)]">{b.dominant_operator}</span></span>
                  )}
                  <span className="ml-auto">
                    {b.compactable
                      ? <Pill tone="confirmed">well-characterised</Pill>
                      : <Pill tone="open">still learning</Pill>}
                  </span>
                </div>
              ))}
            </div>
            <div className="border-t border-[var(--os-hair)] px-5 py-3.5">
              <p className="max-w-[92ch] text-[12.5px] leading-[1.55] text-[var(--os-muted)]">{report.note}</p>
              <p className="mono mt-2 text-[10.5px] leading-[1.5] text-[var(--os-faint)]">
                &ldquo;Well-characterised&rdquo; means a regime has enough data and a stable dominant operator that it
                <em> could</em> be summarised into a rule for bounded context. The store is append-only: the raw stream is
                always kept, and the Memory Manager never proposes deletion.
              </p>
            </div>
          </>
        )}
      </Panel>
    </div>
  );
}
