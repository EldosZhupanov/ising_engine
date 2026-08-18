import { dataset, provenance } from "@/data";
import { Eyebrow, Panel, PanelHeader, Readout, ProvenanceTag, NotLive } from "@/components/os/kit";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";

export default function Dataset() {
  const src = provenance.sources?.find((s) => s.label.includes("dataset"));
  if (!dataset || (dataset as { missing?: boolean }).missing) {
    return <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/dataset" />
      <Spine route="/dataset" />
      <Guide route="/dataset" /><Eyebrow n="05">Dataset</Eyebrow><NotLive status="planned" note="No foundation_manifest.md ingested." /></div>;
  }
  const ms = dataset.milestones ?? [];
  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/dataset" />
      <Spine route="/dataset" />
      <Guide route="/dataset" />
      <div className="mb-6 flex flex-wrap items-end justify-between gap-3">
        <div>
          <Eyebrow n="05">Foundation Dataset</Eyebrow>
          <h1 className="text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">A training corpus — honestly, not yet a foundation model.</h1>
        </div>
        <ProvenanceTag file={src?.file} rows={src?.rows} mtime={src?.mtime} />
      </div>

      <Panel ticks className="mb-4 p-6">
        <div className="grid grid-cols-3 gap-6">
          <Readout label="Examples" accent value={dataset.examples?.toLocaleString()} sub="structural features → algorithm → outcome" />
          <Readout label="Distinct instances" value={dataset.instances} />
          <Readout label="Operator vocabulary" value={dataset.operators} />
        </div>
      </Panel>

      <Panel>
        <PanelHeader title="Distance to foundation scale" status="real" sub="honest scale-gap toward a Research Foundation Model" />
        <div className="p-5">
          {ms.map((m) => (
            <div key={m.milestone} className="grid grid-cols-[120px_1fr_90px] items-center gap-4 py-2">
              <span className="mono text-[13px] text-[var(--os-ink)] tnum">{m.milestone.toLocaleString()}</span>
              <span className="h-1.5 overflow-hidden rounded-full bg-[var(--os-raised)]"><span className="block h-full rounded-full bg-[var(--os-accent)]" style={{ width: `${Math.min(100, (m.examples / m.milestone) * 100)}%` }} /></span>
              <span className="mono text-right text-[12px] text-[var(--os-warn)]">{m.multiplier}× to go</span>
            </div>
          ))}
          <p className="mt-4 border-t border-[var(--os-hair)] pt-4 text-[13px] leading-[1.6] text-[var(--os-muted)]">
            With {dataset.examples?.toLocaleString()} examples this is a training corpus, not a foundation model. The honest path is to grow it — Curiosity for informative data, Scientific Memory to never lose it — until the milestones are met.
          </p>
        </div>
      </Panel>
    </div>
  );
}
