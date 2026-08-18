"use client";

/**
 * Solver Playground — genuinely live against the production solver.
 *
 * This talks to `src/bin/server_api.rs` (`POST /api/v1/solve`), which is the
 * production `ParallelTemperingSolver` path, semaphore-guarded and core-pinned.
 * The request/response shapes here mirror `QuboRequest` / `QuboResponse` exactly.
 *
 * Two honest notes the UI states out loud:
 *  • server_api binds 0.0.0.0:8080 and has no CORS layer of its own, so a browser
 *    request may be blocked by the browser even though the server is healthy.
 *    We detect that and say so rather than reporting a fake failure.
 *  • the solver returns a real energy; nothing here is simulated.
 */

import { useCallback, useEffect, useState } from "react";
import { postSolve, DEFAULT_SOLVER_URL, type SolveRequest, type SolveResponse } from "@/lib/control/client";
import { Panel, PanelHeader, Pill, StatusBadge, Icon } from "@/components/os/kit";

/** A tiny, checkable instance: minimise x'Qx with an obvious optimum. */
const PRESETS: { id: string; label: string; note: string; build: () => SolveRequest }[] = [
  {
    id: "triangle", label: "3-var frustrated triangle",
    note: "All couplings positive, so no assignment satisfies every pair — the classic frustration case.",
    build: () => ({ num_vars: 3, linear: [0, 0, 0], quadratic: [[0, 1, 1], [1, 2, 1], [0, 2, 1]], sweeps: 200, seed: 7 }),
  },
  {
    id: "maxcut4", label: "4-node MaxCut (square)",
    note: "A 4-cycle: the optimal cut alternates around the ring.",
    build: () => ({ num_vars: 4, linear: [0, 0, 0, 0], quadratic: [[0, 1, -1], [1, 2, -1], [2, 3, -1], [0, 3, -1]], sweeps: 400, seed: 11 }),
  },
  {
    id: "biased", label: "8-var with linear bias",
    note: "Linear terms pull individual variables; quadratic terms couple neighbours.",
    build: () => ({
      num_vars: 8,
      linear: [-1, 0.5, -0.5, 1, -1, 0.25, 0, -0.75],
      quadratic: Array.from({ length: 7 }, (_, i) => [i, i + 1, i % 2 === 0 ? 1 : -1] as [number, number, number]),
      sweeps: 600, replicas: 16, seed: 42,
    }),
  },
];

export default function Solver() {
  const [url, setUrl] = useState(DEFAULT_SOLVER_URL);
  const [preset, setPreset] = useState(PRESETS[0].id);
  const [body, setBody] = useState(() => JSON.stringify(PRESETS[0].build(), null, 2));
  const [res, setRes] = useState<SolveResponse | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [reach, setReach] = useState<"unknown" | "ok" | "blocked">("unknown");

  const pick = (id: string) => {
    const p = PRESETS.find((x) => x.id === id);
    if (!p) return;
    setPreset(id); setBody(JSON.stringify(p.build(), null, 2)); setRes(null); setErr(null);
  };

  // A HEAD/GET probe distinguishes "not running" from "running but CORS-blocked".
  const probe = useCallback(async () => {
    try {
      await fetch(`${url.replace(/\/$/, "")}/health`, { mode: "cors" });
      setReach("ok");
    } catch { setReach("blocked"); }
  }, [url]);

  useEffect(() => { let a = true; const t = async () => { if (a) await probe(); }; void t(); return () => { a = false; }; }, [probe]);

  const solve = async () => {
    setBusy(true); setErr(null); setRes(null);
    let parsed: SolveRequest;
    try { parsed = JSON.parse(body) as SolveRequest; }
    catch (e) { setErr(`invalid JSON: ${e instanceof Error ? e.message : e}`); setBusy(false); return; }
    try { setRes(await postSolve(url, parsed)); setReach("ok"); }
    catch (e) {
      const m = e instanceof Error ? e.message : String(e);
      setErr(m.includes("Failed to fetch")
        ? "The browser blocked the request. server_api has no CORS layer, so a page cannot call it directly — this is a server limitation, not a solver failure. Verify from a terminal with curl, or add a CORS layer to server_api."
        : m);
      setBusy(false); return;
    }
    setBusy(false);
  };

  const current = PRESETS.find((p) => p.id === preset);

  return (
    <div className="space-y-4">
      <Panel>
        <PanelHeader title="Production solver" status={reach === "ok" ? "real" : "offline"}
          sub="src/bin/server_api.rs · POST /api/v1/solve · ParallelTemperingSolver"
          right={<button onClick={() => void probe()} className="mono rounded-[7px] border border-[var(--os-hair-2)] px-2.5 py-1 text-[11px] text-[var(--os-muted)] hover:text-[var(--os-ink)]">re-check</button>} />
        <div className="space-y-3 p-5">
          <div className="flex flex-wrap items-center gap-2">
            <StatusBadge status={reach === "ok" ? "real" : "offline"} />
            <span className="mono text-[11px] text-[var(--os-faint)]">
              {reach === "ok" ? "reachable from this page" : reach === "blocked" ? "not reachable from the browser" : "checking…"}
            </span>
          </div>
          <label className="block">
            <span className="mono text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">solver address</span>
            <input value={url} onChange={(e) => setUrl(e.target.value)}
              className="mono mt-1 w-full rounded-[8px] border border-[var(--os-hair-2)] bg-[var(--os-ground)] px-3 py-2 text-[12.5px] text-[var(--os-ink)] focus:border-[var(--os-accent)] focus:outline-none" />
          </label>
          {reach !== "ok" && (
            <div className="space-y-2">
              <pre className="mono overflow-x-auto rounded-[8px] border border-[var(--os-hair)] bg-[var(--os-ground)] p-3 text-[11px] text-[var(--os-ink)]">cargo run --release --bin server_api</pre>
              <p className="mono text-[10.5px] leading-[1.5] text-[var(--os-warn)]">
                Note: <span className="text-[var(--os-ink)]">server_api sends no CORS headers</span>, so even when it is
                running the browser may refuse the call. That is a server-side gap (the control plane adds CORS; this
                older binary does not) — not a solver problem. From a terminal it works:
              </p>
              <pre className="mono overflow-x-auto rounded-[8px] border border-[var(--os-hair)] bg-[var(--os-ground)] p-3 text-[10.5px] text-[var(--os-muted)]">{`curl -s ${url}/api/v1/solve -H 'Content-Type: application/json' \\
  -d '${JSON.stringify(PRESETS[0].build())}'`}</pre>
            </div>
          )}
        </div>
      </Panel>

      <div className="grid gap-4 lg:grid-cols-2">
        <Panel>
          <PanelHeader title="Problem" status="real" sub="QuboRequest — num_vars, linear, quadratic (i, j, w)" />
          <div className="space-y-3 p-5">
            <div className="flex flex-wrap gap-1.5">
              {PRESETS.map((p) => (
                <button key={p.id} onClick={() => pick(p.id)}
                  className={`mono rounded-full border px-2.5 py-1 text-[11px] ${preset === p.id ? "border-[var(--os-accent)] bg-[var(--os-accent-soft)] text-[var(--os-ink)]" : "border-[var(--os-hair-2)] text-[var(--os-muted)] hover:text-[var(--os-ink)]"}`}>
                  {p.label}
                </button>
              ))}
            </div>
            {current && <p className="text-[12px] leading-[1.5] text-[var(--os-muted)]">{current.note}</p>}
            <textarea value={body} onChange={(e) => setBody(e.target.value)} rows={14} spellCheck={false}
              className="mono w-full rounded-[8px] border border-[var(--os-hair-2)] bg-[var(--os-ground)] p-3 text-[11.5px] leading-[1.5] text-[var(--os-ink)] focus:border-[var(--os-accent)] focus:outline-none" />
            <button onClick={() => void solve()} disabled={busy}
              className="mono flex w-full items-center justify-center gap-2 rounded-[9px] bg-[var(--os-accent)] px-4 py-2.5 text-[13px] font-medium text-[var(--os-on-accent)] disabled:opacity-40">
              <Icon name="Play" size={14} /> {busy ? "solving…" : "solve"}
            </button>
          </div>
        </Panel>

        <Panel>
          <PanelHeader title="Result" status="real" sub="energy and spin state from the production solver" />
          <div className="p-5">
            {err && (
              <div className="rounded-[8px] border border-[color:rgba(240,113,90,.3)] bg-[rgba(240,113,90,.06)] px-3.5 py-3">
                <div className="mono mb-1 text-[10px] uppercase tracking-[.12em] text-[var(--os-refuted)]">not solved</div>
                <p className="text-[12.5px] leading-[1.55] text-[var(--os-ink)]">{err}</p>
              </div>
            )}
            {!err && !res && (
              <p className="text-[13px] leading-[1.6] text-[var(--os-muted)]">
                No result yet. This calls the same production solver the platform uses, so the energy returned is real —
                and, being a stochastic solver, may differ between runs unless you pin <span className="mono">seed</span>.
              </p>
            )}
            {res && (
              <div className="space-y-4">
                <div className="flex flex-wrap items-center gap-2">
                  <Pill tone={res.status === "ok" || res.status === "success" ? "confirmed" : "neutral"}>{res.status}</Pill>
                  <span className="mono text-[11px] text-[var(--os-faint)] tnum">{res.compute_time_ms} ms</span>
                </div>
                <div>
                  <div className="mono text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">energy</div>
                  <div className="mono text-[2rem] font-semibold leading-none tnum text-[var(--os-accent)]">
                    {res.energy.toLocaleString(undefined, { maximumFractionDigits: 4 })}
                  </div>
                </div>
                <div>
                  <div className="mono mb-1.5 text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">
                    spin state ({res.state.length} vars)
                  </div>
                  <div className="flex flex-wrap gap-1">
                    {res.state.map((s, i) => (
                      <span key={i} title={`x${i} = ${s}`}
                        className={`mono flex h-6 w-6 items-center justify-center rounded-[5px] text-[10px] ${
                          s > 0 ? "bg-[var(--os-accent)] text-[var(--os-on-accent)]" : "bg-[var(--os-raised)] text-[var(--os-muted)]"}`}>
                        {s > 0 ? "+" : "−"}
                      </span>
                    ))}
                  </div>
                </div>
              </div>
            )}
          </div>
        </Panel>
      </div>
    </div>
  );
}
