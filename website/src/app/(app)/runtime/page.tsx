import { overview, operators, experiments } from "@/data";
import { BACKENDS } from "@/data/curated";
import { Eyebrow, Panel, PanelHeader, Pill, Readout } from "@/components/os/kit";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";
import SystemPanel from "@/components/os/control/SystemPanel";

export default function RuntimePage() {
  const byBackend = experiments.rollups.byBackend;
  const total = byBackend.reduce((a, b) => a + b.runs, 0) || 1;

  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/runtime" />
      <Spine route="/runtime" activeId="experiment" />
      <div className="mb-6">
        <Eyebrow n="▤">Runtime &amp; Backends</Eyebrow>
        <h1 className="max-w-[32ch] text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">
          One deterministic substrate. Same seed, same trajectory, same bit.
        </h1>
        <p className="mt-2 max-w-[84ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
          The Runtime is the only thing that computes; every faculty above it merely generates work and analyses results.
          It is read-only by constitution (ADR-0004): you extend the platform by adding operators and agents above it,
          never by editing it to suit a caller. That discipline is what makes replay meaningful.
        </p>
      </div>

      <Guide route="/runtime" />

      <Panel ticks className="mb-4 p-6">
        <div className="grid grid-cols-2 gap-6 sm:grid-cols-4">
          <Readout label="Backends" accent value={BACKENDS.length} sub="cross-checked bit-identical" />
          <Readout label="Operators" value={operators.operators.length} sub="selected by capability" />
          <Readout label="Runs executed" value={overview.totals.runsAll.toLocaleString()} sub="all campaigns" />
          <Readout label="Instances" value={overview.totals.instances} sub="G-Set MaxCut" />
        </div>
      </Panel>

      <Panel className="mb-4">
        <PanelHeader title="Backends" status="real" sub="engine_v2/state.rs — chosen per problem by the Decision Engine" />
        <div className="divide-y divide-[var(--os-hair)]">
          {BACKENDS.map((b) => {
            const use = byBackend.find((x) => x.key === b.name);
            const pct = use ? (use.runs / total) * 100 : 0;
            return (
              <div key={b.name} className="px-5 py-4">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="mono text-[13.5px] text-[var(--os-ink)]">{b.name}</span>
                  <Pill tone="neutral">{b.tag}</Pill>
                  {use && <span className="mono text-[11px] tnum text-[var(--os-faint)]">{use.runs.toLocaleString()} runs ({pct.toFixed(0)}% of the record)</span>}
                  {!use && <span className="mono text-[11px] text-[var(--os-faint)]">not used in the ingested campaigns</span>}
                </div>
                <p className="mt-1.5 max-w-[80ch] text-[12.5px] leading-[1.55] text-[var(--os-muted)]">{b.role}</p>
                {use && (
                  <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-[var(--os-raised)]">
                    <div className="h-full rounded-full bg-[var(--os-cyan)]" style={{ width: `${pct}%` }} />
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </Panel>

      <Panel className="mb-4">
        <PanelHeader title="The determinism firewall" status="real" sub="why this substrate is trustworthy" />
        <div className="grid gap-4 p-5 md:grid-cols-3">
          {[
            { t: "Same seed → same trajectory", d: "Every recorded run carries its seed, so any result can be re-executed and compared. Replay any row from the Experiments workspace to check it yourself." },
            { t: "Cross-backend bit-identity", d: "A fast production backend is validated against an exact oracle past 100k spins — performance without giving up correctness." },
            { t: "Adaptive features are opt-in", d: "Anything that could change a trajectory (early-stop, adaptation) is opt-in and replay-safe, so the default path stays reproducible." },
          ].map((x) => (
            <div key={x.t} className="os-raised rounded-[12px] p-4">
              <div className="mono text-[12px] text-[var(--os-ink)]">{x.t}</div>
              <p className="mt-1.5 text-[12px] leading-[1.5] text-[var(--os-muted)]">{x.d}</p>
            </div>
          ))}
        </div>
      </Panel>

      {/* live host telemetry, when the control plane is up */}
      <SystemPanel />

      <Panel className="mt-4 p-5">
        <div className="mono mb-2 text-[11px] uppercase tracking-[.16em] text-[var(--os-muted)]">Not yet observable here</div>
        <p className="max-w-[86ch] text-[13px] leading-[1.6] text-[var(--os-muted)]">
          The Runtime emits a per-step event log (<span className="mono text-[var(--os-ink)]">StepEvent</span>,{" "}
          <span className="mono text-[var(--os-ink)]">RunRecord</span>,{" "}
          <span className="mono text-[var(--os-ink)]">QualityMetrics</span>) that the Dynamics and World models consume —
          but it is never serialised, so live per-step telemetry, the energy landscape and a trajectory scrubber cannot be
          drawn honestly yet. That export is the single highest-value change remaining on the engine side.
        </p>
      </Panel>
    </div>
  );
}
