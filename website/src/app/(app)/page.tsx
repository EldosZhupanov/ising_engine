import Link from "next/link";
import { overview, provenance, dataAvailable } from "@/data";
import { LOOP_STAGES, LEDGER } from "@/data/curated";
import { Icon, Eyebrow, Panel, PanelHeader, Readout, CountUp, Pill, ProvenanceTag } from "@/components/os/kit";
import { LoopRing } from "@/components/os/islands";
import Freshness from "@/components/os/control/Freshness";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";

export default function MissionControl() {
  const t = overview.totals;
  const best = overview.best;
  const src = provenance.sources ?? [];
  const dbRows = src.filter((s) => s.label.endsWith(":db")).reduce((a, s) => a + s.rows, 0) || t.runsAll;

  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/" />
      <Spine route="/" activeId="problem" />
      <Guide route="/" />
      {/* header */}
      <div className="mb-8 flex flex-wrap items-end justify-between gap-4">
        <div>
          <Eyebrow n="00">Mission Control</Eyebrow>
          <h1 className="max-w-[24ch] text-[clamp(1.9rem,3.4vw,2.9rem)] font-semibold leading-[1.05] tracking-[-.03em] text-balance">
            An autonomous scientist you can <span className="text-[var(--os-accent)]">watch, interrogate, and reproduce</span>.
          </h1>
        </div>
        <span className="flex flex-wrap items-center gap-2">
          <Freshness snapshotRows={t.runsPrimary} />
          <ProvenanceTag file="experiments/…/ai_experiments.txt" rows={dbRows} mtime={src[0]?.mtime} />
        </span>
      </div>

      {/* hero: telemetry + loop */}
      <div className="grid gap-4 lg:grid-cols-[1.3fr_1fr]">
        <Panel ticks className="p-6">
          <div className="grid grid-cols-2 gap-6 sm:grid-cols-3">
            <Readout label="Experiments recorded" accent value={<CountUp to={t.runsAll} />} sub={`all ${overview.perCampaign.length} campaigns combined`} />
            <Readout label="Distinct instances" value={<CountUp to={t.instances} />} sub={`G-Set MaxCut · in ${overview.primary}`} />
            <Readout label="Physics operators" value={<CountUp to={t.operators} />} sub="by capability, not name" />
            <Readout label="Backends" value={t.backends.length} sub={t.backends.join(" · ")} />
            <Readout label="Generations" value={<CountUp to={t.generations} />} sub="across campaigns" />
            <Readout label="Graph facts" value={<CountUp to={overview.graphFactCount} />} sub={`typed & weighted · ${overview.primary}`} />
          </div>
          {best && (
            <div className="mt-6 flex flex-wrap items-center gap-x-4 gap-y-2 border-t border-[var(--os-hair)] pt-5">
              <span className="mono text-[11px] uppercase tracking-[.14em] text-[var(--os-muted)]">Best result</span>
              <span className="mono text-[15px] text-[var(--os-accent)] tnum">{best.score.toLocaleString()}</span>
              <span className="mono text-[12px] text-[var(--os-faint)]">baseline {best.baseline.toLocaleString()} · {best.instance}</span>
              <span className="mono text-[12px] text-[var(--os-muted)]">[{best.seq.join(" → ")}]</span>
            </div>
          )}
        </Panel>

        <Panel ticks className="flex flex-col p-6">
          <div className="mono mb-1 flex items-center justify-between text-[11px] uppercase tracking-[.16em] text-[var(--os-muted)]">
            <span>Research loop</span><span className="text-[var(--os-faint)]">orchestrator.rs::tick</span>
          </div>
          <div className="flex flex-1 items-center justify-center"><LoopRing stages={LOOP_STAGES} /></div>
        </Panel>
      </div>

      {/* latest discoveries + ledger */}
      <div className="mt-4 grid gap-4 lg:grid-cols-2">
        <Panel>
          <PanelHeader title="Latest discoveries" status="real" sub={overview.latestPaper ? `${overview.latestPaper.file} · ${overview.latestPaper.experiments?.toLocaleString()} experiments` : undefined} right={<Link href="/papers" className="mono text-[11px] text-[var(--os-muted)] hover:text-[var(--os-ink)]">papers →</Link>} />
          <div className="divide-y divide-[var(--os-hair)]">
            {(overview.latestPaper?.discoveries ?? []).map((d, i) => (
              <div key={i} className="flex items-start gap-3 px-5 py-3">
                <span className="mono mt-0.5 text-[10px] text-[var(--os-faint)]">{String(i + 1).padStart(2, "0")}</span>
                <p className="flex-1 text-[13px] leading-[1.5] text-[var(--os-ink)]">{d.text}</p>
                <span className="mono shrink-0 text-[10.5px] text-[var(--os-faint)]">s{d.support} · {d.confidence.toFixed(2)}</span>
              </div>
            ))}
            {!overview.latestPaper && <div className="px-5 py-6 text-[13px] text-[var(--os-faint)]">No reports ingested.</div>}
          </div>
        </Panel>

        <Panel>
          <PanelHeader title="Confirmed vs. refuted" status="real" sub="research honesty — refutations are kept" />
          <div className="divide-y divide-[var(--os-hair)]">
            {LEDGER.map((l, i) => (
              <div key={i} className="flex items-start gap-3 px-5 py-3">
                <Pill tone={l.status === "confirmed" ? "confirmed" : "refuted"}>{l.status}</Pill>
                <div className="min-w-0 flex-1">
                  <div className="text-[13px] font-medium text-[var(--os-ink)]">{l.claim}</div>
                  <div className="mt-0.5 text-[12px] leading-[1.5] text-[var(--os-muted)]">{l.detail}</div>
                  <div className="mono mt-1 text-[10px] text-[var(--os-faint)]">{l.source}</div>
                </div>
              </div>
            ))}
          </div>
        </Panel>
      </div>

      {/* campaigns + jump */}
      <div className="mt-4 grid gap-4 lg:grid-cols-[1fr_1.4fr]">
        <Panel>
          <PanelHeader title="Campaigns" status="real" sub="recorded, append-only" right={<Link href="/campaigns" className="mono text-[11px] text-[var(--os-muted)] hover:text-[var(--os-ink)]">manage →</Link>} />
          <div className="divide-y divide-[var(--os-hair)]">
            {overview.perCampaign.map((c) => (
              <div key={c.label} className="flex items-center justify-between px-5 py-3">
                <span className="mono text-[13px] text-[var(--os-ink)]">{c.label}</span>
                <span className="mono text-[12px] text-[var(--os-faint)]">{c.runs.toLocaleString()} runs · {c.instances} inst · {c.generations} gen</span>
              </div>
            ))}
          </div>
        </Panel>

        <Panel className="p-5">
          <div className="mono mb-3 text-[11px] uppercase tracking-[.16em] text-[var(--os-muted)]">Jump into the lab</div>
          <div className="grid grid-cols-2 gap-2.5 sm:grid-cols-3">
            {[
              { href: "/experiments", icon: "Activity", label: "Experiments" },
              { href: "/graph", icon: "Waypoints", label: "Knowledge Graph" },
              { href: "/benchmarks", icon: "GitCompare", label: "Benchmarks" },
              { href: "/evaluation", icon: "ClipboardCheck", label: "Evaluation" },
              { href: "/dataset", icon: "Database", label: "Dataset" },
              { href: "/operators", icon: "Boxes", label: "Operators" },
            ].map((j) => (
              <Link key={j.href} href={j.href} className="os-raised group flex items-center gap-2.5 rounded-[11px] px-3.5 py-3 transition-colors hover:border-[color:rgba(240,132,46,0.4)]">
                <Icon name={j.icon} size={16} className="text-[var(--os-muted)] group-hover:text-[var(--os-accent)]" />
                <span className="text-[12.5px] text-[var(--os-ink)]">{j.label}</span>
              </Link>
            ))}
          </div>
        </Panel>
      </div>

      {!dataAvailable && (
        <div className="mono mt-4 rounded-[10px] border border-[var(--os-warn)]/30 bg-[var(--os-warn)]/5 px-4 py-3 text-[12px] text-[var(--os-warn)]">
          Showing bundled fixture data — the experiments/ directory was not found at build time.
        </div>
      )}
    </div>
  );
}
