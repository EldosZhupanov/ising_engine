import { graph, provenance } from "@/data";
import { Eyebrow, ProvenanceTag } from "@/components/os/kit";
import GraphCanvas from "@/components/os/GraphCanvas";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";

export default function GraphPage() {
  const src = provenance.sources?.find((s) => s.label.endsWith(":graph"));
  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/graph" />
      <Spine route="/graph" activeId="knowledge" />
      <Guide route="/graph" />
      <div className="mb-6 flex flex-wrap items-end justify-between gap-3">
        <div>
          <Eyebrow n="02">Knowledge Graph</Eyebrow>
          <h1 className="max-w-[26ch] text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">Conditional, source-attributed facts — never a bare assertion.</h1>
          <p className="mt-2 max-w-[70ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">Typed triples <span className="mono">subject —relation→ object</span>, each carrying an evidence weight, a support count, and a confidence. Confidence accrues by Welford statistics over repeated observation.</p>
        </div>
        <ProvenanceTag file={src?.file} rows={src?.rows} mtime={src?.mtime} />
      </div>
      <GraphCanvas facts={graph.facts} relations={graph.relations} />
    </div>
  );
}
