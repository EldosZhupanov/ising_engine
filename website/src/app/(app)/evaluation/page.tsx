import { evaluation, provenance } from "@/data";
import { Eyebrow, Panel, PanelHeader, Pill, ProvenanceTag, NotLive } from "@/components/os/kit";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";

export default function Evaluation() {
  const src = provenance.sources?.find((s) => s.label.includes("evaluation"));
  const rows = evaluation.rows ?? [];
  const families = [...new Set(rows.map((r) => r.family))];
  const maxInst = Math.max(...rows.map((r) => r.instances), 1);

  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/evaluation" />
      <Spine route="/evaluation" />
      <Guide route="/evaluation" />
      <div className="mb-6 flex flex-wrap items-end justify-between gap-3">
        <div>
          <Eyebrow n="04">Evaluation</Eyebrow>
          <h1 className="text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">Does the platform reproduce its own rules?</h1>
          <p className="mono mt-2 text-[12px] text-[var(--os-faint)]">A rule counts as reproduced when its signature emerges independently from ≥ 2 instances.</p>
        </div>
        <ProvenanceTag file={src?.file} rows={src?.rows} mtime={src?.mtime} />
      </div>

      {rows.length === 0 ? <NotLive status="planned" note="No evaluation_report.md ingested." /> : (
        <Panel>
          <PanelHeader title="Rule reproducibility across instances" status="real" sub={`${rows.length} signatures · families: ${families.join(" · ")}`} />
          <div className="overflow-x-auto">
            <table className="w-full min-w-[720px] text-left">
              <thead><tr className="mono border-b border-[var(--os-hair)] text-[10.5px] uppercase tracking-[.08em] text-[var(--os-faint)]">
                <th className="px-4 py-2.5 font-normal">signature</th><th className="px-4 py-2.5 font-normal">instances</th><th className="px-4 py-2.5 font-normal">confidence</th><th className="px-4 py-2.5 font-normal">example</th></tr></thead>
              <tbody>
                {rows.map((r, i) => (
                  <tr key={i} className="border-b border-[var(--os-hair)] text-[12.5px] last:border-0 hover:bg-[var(--os-raised)]">
                    <td className="px-4 py-3"><span className="mono text-[var(--os-ink)]">{r.signature}</span></td>
                    <td className="px-4 py-3">
                      <div className="flex items-center gap-2">
                        <span className="h-1.5 w-16 overflow-hidden rounded-full bg-[var(--os-raised)]"><span className="block h-full rounded-full bg-[var(--os-cyan)]" style={{ width: `${(r.instances / maxInst) * 100}%` }} /></span>
                        <span className="mono tnum text-[var(--os-muted)]">{r.instances}</span>
                      </div>
                    </td>
                    <td className="px-4 py-3"><Pill tone={r.confidence >= 0.9 ? "confirmed" : r.confidence >= 0.6 ? "open" : "neutral"}>{r.confidence.toFixed(2)}</Pill></td>
                    <td className="px-4 py-3 text-[12px] text-[var(--os-muted)]">{r.example}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Panel>
      )}
    </div>
  );
}
