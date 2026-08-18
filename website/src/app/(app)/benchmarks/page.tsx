import { benchmarks, provenance } from "@/data";
import { Eyebrow, Panel, PanelHeader, Pill, ProvenanceTag, NotLive } from "@/components/os/kit";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";

export default function Benchmarks() {
  const src = provenance.sources?.find((s) => s.label.includes("benchmarks:index"));
  const runs = benchmarks.index?.runs ?? [];
  const specById = new Map(benchmarks.specs.map((s) => [s.id, s]));

  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/benchmarks" />
      <Spine route="/benchmarks" activeId="statistics" />
      <Guide route="/benchmarks" />
      <div className="mb-6 flex flex-wrap items-end justify-between gap-3">
        <div>
          <Eyebrow n="03">Benchmark Center</Eyebrow>
          <h1 className="text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">Honest head-to-heads — with the kill-criterion in view.</h1>
        </div>
        <ProvenanceTag file={src?.file} rows={src?.rows} mtime={src?.mtime} />
      </div>

      {runs.length === 0 && <NotLive status="planned" note="No results/index.json was ingested." />}

      <div className="space-y-4">
        {runs.map((run) => {
          const spec = specById.get(run.exp);
          const engines = Object.entries(run.per_engine);
          const h2h = engines.map(([, v]) => v.head_to_head).find(Boolean);
          return (
            <Panel key={run.exp}>
              <PanelHeader title={spec?.title ?? run.exp} status="real" sub={`${run.exp} · ${run.ts}`} right={spec && <Pill tone={spec.status === "active" ? "open" : "neutral"}>{spec.status}</Pill>} />
              <div className="grid gap-4 p-5 lg:grid-cols-[1.4fr_1fr]">
                <div>
                  {spec && (
                    <div className="mb-4 space-y-2">
                      <div><span className="mono text-[10px] uppercase tracking-[.1em] text-[var(--os-faint)]">Hypothesis</span><p className="mt-0.5 text-[13px] leading-[1.55] text-[var(--os-muted)]">{spec.hypothesis}</p></div>
                      <div><span className="mono text-[10px] uppercase tracking-[.1em] text-[var(--os-refuted)]">Kill-criterion</span><p className="mt-0.5 text-[13px] leading-[1.55] text-[var(--os-muted)]">{spec.kill_criterion}</p></div>
                    </div>
                  )}
                  <table className="w-full text-left">
                    <thead><tr className="mono border-b border-[var(--os-hair)] text-[10px] uppercase tracking-[.08em] text-[var(--os-faint)]"><th className="py-2 font-normal">engine</th><th className="py-2 font-normal">mean best</th><th className="py-2 font-normal">mean wall (ms)</th></tr></thead>
                    <tbody>
                      {engines.map(([name, v]) => (
                        <tr key={name} className="border-b border-[var(--os-hair)] text-[12.5px] last:border-0">
                          <td className="mono py-2.5 text-[var(--os-ink)]">{name}</td>
                          <td className="mono py-2.5 tnum text-[var(--os-ink)]">{v.mean_best.toLocaleString(undefined, { maximumFractionDigits: 1 })}</td>
                          <td className="mono py-2.5 tnum text-[var(--os-muted)]">{v.mean_wall_ms.toLocaleString(undefined, { maximumFractionDigits: 0 })}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
                {h2h && (
                  <div className="os-raised rounded-[12px] p-5">
                    <div className="mono text-[11px] uppercase tracking-[.14em] text-[var(--os-muted)]">Head-to-head vs {h2h.vs}</div>
                    <div className="mono mt-3 text-[3rem] font-semibold leading-none tnum text-[var(--os-accent)]">{Math.round(h2h.win_fraction * 100)}%</div>
                    <div className="mono mt-1 text-[11px] text-[var(--os-faint)]">win fraction over {h2h.instances} instances</div>
                    <div className="mono mt-4 space-y-1 text-[12px] text-[var(--os-muted)]">
                      <div>Wilcoxon p = {h2h.wilcoxon_p} · sign-test p = {h2h.sign_test_p}</div>
                      {h2h.instances < 6 && <div className="text-[var(--os-warn)]">underpowered (n = {h2h.instances}) — not a strong claim</div>}
                    </div>
                  </div>
                )}
              </div>
            </Panel>
          );
        })}
      </div>
    </div>
  );
}
