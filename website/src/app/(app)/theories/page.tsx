import { THEORY_PIPELINE } from "@/data/curated";
import { Eyebrow, Panel, Pill, StatusBadge } from "@/components/os/kit";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";

export default function Theories() {
  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/theories" />
      <Spine route="/theories" activeId="theory" />
      <Guide route="/theories" />
      <div className="mb-6">
        <Eyebrow n="07">Theory Engine</Eyebrow>
        <h1 className="max-w-[26ch] text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">A theory is a mechanism that survived an attempt to break it.</h1>
        <p className="mt-2 max-w-[72ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">Every analyzer stops at a rule — a correlation. The Theory Engine promotes it to a causal mechanism, forces a falsifiable prediction, then runs the ablation on the runtime that would refute it. What survives is a theory; what doesn&rsquo;t is kept as a refutation.</p>
      </div>

      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-5">
        {THEORY_PIPELINE.map((s, i) => (
          <Panel key={s.stage} ticks className="p-4">
            <div className="mono text-[10px] text-[var(--os-faint)]">{String(i + 1).padStart(2, "0")}</div>
            <div className="mt-1 flex items-center gap-2"><span className="h-1.5 w-1.5 rounded-full bg-[var(--os-accent)]" /><h3 className="text-[14px] font-semibold text-[var(--os-ink)]">{s.stage}</h3></div>
            <p className="mt-2 text-[12px] leading-[1.5] text-[var(--os-muted)]">{s.desc}</p>
          </Panel>
        ))}
      </div>

      <div className="mt-4 grid gap-3 md:grid-cols-2">
        <Panel className="p-5"><div className="mb-2 flex items-center gap-2"><Pill tone="confirmed">survived</Pill><span className="mono text-[12px] text-[var(--os-muted)]">thermal exploration</span></div><p className="text-[13.5px] leading-[1.6] text-[var(--os-ink)]">Thermal exploration dominates across every problem class tested (n = 75, 6 families). Ablating temperature collapses performance — the mechanism held. metropolis_sweep is confirmed causal (removing it costs ~5%).</p></Panel>
        <Panel className="p-5"><div className="mb-2 flex items-center gap-2"><Pill tone="refuted">refuted · kept</Pill><span className="mono text-[12px] text-[var(--os-muted)]">self-proposed operator</span></div><p className="text-[13.5px] leading-[1.6] text-[var(--os-ink)]">The loop&rsquo;s own proposed operator extremal_metropolis is weak on G-Set — it appears in the worst solutions ~8×. Tested and refuted, honestly; the dead end is recorded so it is never explored twice.</p></Panel>
      </div>

      <div className="mt-4 os-panel os-ticks flex flex-col items-start gap-3 p-6">
        <StatusBadge status="offline" />
        <p className="max-w-[64ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">Launching a new ablation (<span className="mono">research_platform --investigate &lt;operator&gt;</span>) requires the control-plane API. Recorded verdicts above are real; live investigation lights up in the Cockpit phase.</p>
      </div>
    </div>
  );
}
