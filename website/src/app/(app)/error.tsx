"use client";

import Link from "next/link";

/** Workspace-level error boundary: never a white screen, always actionable. */
export default function WorkspaceError({ error, reset }: { error: Error & { digest?: string }; reset: () => void }) {
  return (
    <div className="mx-auto max-w-[720px] px-5 py-20 md:px-8">
      <div className="os-panel os-ticks p-7">
        <div className="mono mb-3 text-[11px] uppercase tracking-[.18em] text-[var(--os-refuted)]">workspace error</div>
        <h1 className="text-[1.5rem] font-semibold tracking-[-.02em]">This workspace failed to render.</h1>
        <p className="mt-3 text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
          The rest of the operating system is unaffected. If this persists, the generated data bundle may be
          stale — re-run <span className="mono text-[var(--os-ink)]">npm run ingest</span> to rebuild it from{" "}
          <span className="mono text-[var(--os-ink)]">experiments/</span>.
        </p>
        <pre className="mono mt-4 max-h-[180px] overflow-auto rounded-[8px] border border-[var(--os-hair)] bg-[var(--os-ground)] p-3 text-[11.5px] text-[var(--os-faint)]">
{error.message}{error.digest ? `\n\ndigest: ${error.digest}` : ""}
        </pre>
        <div className="mt-5 flex gap-2">
          <button onClick={reset} className="mono rounded-[8px] bg-[var(--os-accent)] px-3.5 py-2 text-[12px] font-medium text-[var(--os-on-accent)]">retry</button>
          <Link href="/" className="mono rounded-[8px] border border-[var(--os-hair-2)] px-3.5 py-2 text-[12px] text-[var(--os-muted)]">mission control</Link>
        </div>
      </div>
    </div>
  );
}
