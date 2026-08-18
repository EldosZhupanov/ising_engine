import { CONCEPT_PIPELINE } from "@/data/curated";
import { Eyebrow, Panel, Pill } from "@/components/os/kit";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";

export default function Concepts() {
  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/concepts" />
      <Spine route="/concepts" />
      <Guide route="/concepts" />
      <div className="mb-6">
        <Eyebrow n="08">Concept Evolution</Eyebrow>
        <h1 className="max-w-[26ch] text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">The scientist can invent its own concepts — not just sharpen old ones.</h1>
        <p className="mt-2 max-w-[72ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">Most systems only grow confidence inside a fixed vocabulary. Concept Discovery expands the vocabulary: propose a candidate structural concept, test whether it improves out-of-sample prediction, gate it against an Occam penalty, admit only what earns its place.</p>
      </div>

      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        {CONCEPT_PIPELINE.map((s, i) => (
          <Panel key={s.stage} ticks className="p-5">
            <div className="mono text-[10px] text-[var(--os-faint)]">{String(i + 1).padStart(2, "0")}</div>
            <div className="mt-1 flex items-center gap-2"><span className="h-1.5 w-1.5 rounded-full bg-[var(--os-accent)]" /><h3 className="text-[14.5px] font-semibold text-[var(--os-ink)]">{s.stage}</h3></div>
            <p className="mt-2 text-[12.5px] leading-[1.5] text-[var(--os-muted)]">{s.desc}</p>
          </Panel>
        ))}
      </div>

      <div className="mt-4 os-panel os-ticks p-6" style={{ borderColor: "rgba(229,98,74,0.28)" }}>
        <div className="mb-2"><Pill tone="refuted">the discipline</Pill></div>
        <p className="max-w-[74ch] text-[14px] leading-[1.6] text-[var(--os-ink)]">In-sample fit is never enough. The naive 11-feature descriptor fit the data and still failed to generalize — so it was rejected. A concept is admitted only if its <span className="font-medium">out-of-sample</span> gain (leave-one-instance-out ridge RMSE) exceeds an Occam complexity penalty. Truth over optimism, enforced in code.</p>
      </div>
    </div>
  );
}
