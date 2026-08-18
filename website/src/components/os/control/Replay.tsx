"use client";

/**
 * Replay — the capability determinism buys.
 *
 * Any recorded experiment can be re-executed from its seed and the score
 * compared to what was written down. ADR-0004 says it must come back identical;
 * this makes that checkable in one click instead of taken on trust.
 *
 * A mismatch is presented as a FINDING, not an error: it would mean the record
 * and the engine disagree, which is exactly the kind of thing a research
 * platform must surface loudly rather than round away.
 */

import { useState } from "react";
import { postReplay, type ReplayResult } from "@/lib/control/client";
import { useControl } from "@/lib/control/useControl";
import { recordAction } from "@/lib/workflow/progress";
import { Icon, Pill } from "@/components/os/kit";

export default function Replay({ id, recorded }: { id: number; recorded: number }) {
  const { online, url } = useControl();
  const [busy, setBusy] = useState(false);
  const [res, setRes] = useState<ReplayResult | null>(null);
  const [err, setErr] = useState<string | null>(null);

  const run = async () => {
    setBusy(true); setErr(null); setRes(null);
    try {
      const r = await postReplay(url, id);
      setRes(r);
      recordAction("experiments.replayed");
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    } finally { setBusy(false); }
  };

  if (!online) {
    return (
      <span className="mono text-[10.5px] text-[var(--os-faint)]" title={`Start the control plane to replay: cargo run --release --bin control_api`}>
        replay ·&nbsp;offline
      </span>
    );
  }

  return (
    <span className="inline-flex items-center gap-2">
      <button onClick={run} disabled={busy}
        aria-label={`Replay experiment ${id} and verify it reproduces bit-identically`}
        className="mono inline-flex items-center gap-1 rounded-[6px] border border-[var(--os-hair-2)] px-2 py-0.5 text-[10.5px] text-[var(--os-muted)] transition-colors hover:border-[var(--os-accent)] hover:text-[var(--os-accent)] disabled:opacity-50">
        {busy ? <><Icon name="Activity" size={10} className="animate-pulse" /> running…</> : <><Icon name="Play" size={10} /> replay</>}
      </button>

      {res && (
        <span className="inline-flex flex-wrap items-center gap-1.5" role="status">
          {res.identical ? (
            <Pill tone="confirmed">bit-identical</Pill>
          ) : (
            <Pill tone="refuted">MISMATCH Δ{res.delta}</Pill>
          )}
          {/* show the comparison explicitly — recorded → replayed */}
          <span className="mono text-[10px] tnum text-[var(--os-faint)]">
            {recorded.toLocaleString()} → <span className={res.identical ? "text-[var(--os-confirmed)]" : "text-[var(--os-refuted)]"}>{res.replayed_score.toLocaleString()}</span>
            {" · "}{res.backend_used} · {res.wall_ms.toFixed(0)} ms
          </span>
        </span>
      )}

      {err && (
        <span className="mono text-[10px] text-[var(--os-refuted)]" role="alert" title={err}>
          {err.slice(0, 60)}
        </span>
      )}
    </span>
  );
}

/** Compact banner explaining what replay proves — shown once above the table. */
export function ReplayBanner({ total }: { total: number }) {
  const { online, url, status } = useControl();
  return (
    <div className={`mono mb-3 flex flex-wrap items-center gap-x-3 gap-y-1 rounded-[8px] border px-3 py-2 text-[11px] ${
      online ? "border-[color:rgba(63,201,138,.3)] bg-[rgba(63,201,138,.05)]" : "border-[var(--os-hair)] bg-[var(--os-ground-2)]"}`}>
      <span className={online ? "text-[var(--os-confirmed)]" : "text-[var(--os-faint)]"}>
        {online ? "control plane online" : status === "checking" ? "checking control plane…" : "control plane offline"}
      </span>
      {online ? (
        <span className="text-[var(--os-muted)]">
          Any of these {total.toLocaleString()} records can be re-executed from its seed and checked against what was
          written down — determinism (ADR-0004) requires the score to come back identical.
        </span>
      ) : (
        <span className="text-[var(--os-faint)]">
          Replay needs it: <span className="text-[var(--os-muted)]">cargo run --release --bin control_api</span> — then
          rows become one-click verifiable. Expected at {url}.
        </span>
      )}
    </div>
  );
}
