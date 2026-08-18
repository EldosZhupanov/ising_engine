"use client";

/**
 * Data freshness — is what you are looking at current?
 *
 * The workspaces render a build-time snapshot. When the control plane is up we
 * can ask the DB what it contains *now* and say plainly whether the snapshot is
 * still accurate. This is the honest fix for "stale by construction": rather than
 * silently showing old numbers, the UI states the drift.
 */

import { useCallback, useEffect, useState } from "react";
import { getDbStats, type DbStats } from "@/lib/control/client";
import { useControl } from "@/lib/control/useControl";
import { Icon } from "@/components/os/kit";

export default function Freshness({ snapshotRows, compact = false }: { snapshotRows: number; compact?: boolean }) {
  const { online, url } = useControl();
  const [live, setLive] = useState<DbStats | null>(null);

  const load = useCallback(async () => {
    if (!online) { setLive(null); return; }
    try { setLive(await getDbStats(url)); } catch { setLive(null); }
  }, [online, url]);

  useEffect(() => {
    let alive = true;
    const tick = async () => { if (alive) await load(); };
    void tick();
    const id = setInterval(() => { void tick(); }, 10_000);
    return () => { alive = false; clearInterval(id); };
  }, [load]);

  if (!online || !live) {
    return (
      <span className="mono inline-flex items-center gap-1.5 text-[10.5px] text-[var(--os-faint)]"
        title="Numbers come from the build-time snapshot; start control_api to compare against the live record">
        <Icon name="Database" size={10} /> snapshot
      </span>
    );
  }

  const drift = live.experiments - snapshotRows;
  const fresh = drift === 0;
  const when = live.db_mtime ? new Date(live.db_mtime * 1000).toISOString().slice(0, 16).replace("T", " ") : null;

  return (
    <span className={`mono inline-flex items-center gap-1.5 text-[10.5px] ${fresh ? "text-[var(--os-confirmed)]" : "text-[var(--os-warn)]"}`}
      title={`live: ${live.experiments.toLocaleString()} rows in ${live.dir}${when ? ` (written ${when})` : ""}`}>
      <Icon name="Database" size={10} />
      {fresh ? "live · in sync" : `live · ${drift > 0 ? "+" : ""}${drift.toLocaleString()} vs snapshot`}
      {!compact && !fresh && (
        <span className="text-[var(--os-faint)]">— re-run npm run ingest to refresh</span>
      )}
    </span>
  );
}
