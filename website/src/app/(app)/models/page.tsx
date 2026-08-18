import { models, provenance } from "@/data";
import { Eyebrow, Panel, PanelHeader, Pill, ProvenanceTag, Readout } from "@/components/os/kit";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";

const KIND_ROLE: Record<string, string> = {
  predictor: "Ranks operators for an unseen instance before any run — a filter, not an oracle.",
  world: "Predicts the reduced observable trajectory an operator would produce — it imagines outcomes.",
  policy: "Autoregressive operator-token model, supervised on the record and self-play.",
  dynamics: "Predicts remaining improvement from a partial trajectory — drives early-stop.",
};

export default function ModelsPage() {
  const srcs = (provenance.sources ?? []).filter((s) => s.label.startsWith("models:"));
  const kinds = [...new Set(models.map((m) => m.kind))].sort();
  const latest = kinds.map((k) => {
    const of = models.filter((m) => m.kind === k);
    return of.reduce((a, b) => (b.version > a.version ? b : a), of[0]);
  });
  const fmtTs = (t: number) => new Date(t * 1000).toISOString().slice(0, 16).replace("T", " ");

  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/models" />
      <Spine route="/models" activeId="knowledge" />
      <div className="mb-6 flex flex-wrap items-end justify-between gap-3">
        <div>
          <Eyebrow n="◫">Model Registry</Eyebrow>
          <h1 className="max-w-[32ch] text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">
            Every model version, with the parent it came from.
          </h1>
          <p className="mt-2 max-w-[84ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
            The registry is append-only, like the experiment DB: a snapshot per campaign/generation, each pointing at the
            version it superseded. That lineage is what lets a result be attributed to a specific model state rather than
            to &ldquo;the model&rdquo;. Weight payloads stay on disk — only metadata is shipped here.
          </p>
        </div>
        <span className="flex flex-wrap items-center gap-2">
          {srcs.map((s) => <ProvenanceTag key={s.file} file={s.file} rows={s.rows} mtime={s.mtime} />)}
        </span>
      </div>

      <Guide route="/models" />

      {models.length === 0 ? (
        <Panel className="p-6">
          <Pill tone="neutral">no snapshots ingested</Pill>
          <p className="mt-2 max-w-[70ch] text-[13px] leading-[1.6] text-[var(--os-muted)]">
            No <span className="mono">model_registry.txt</span> was found in the ingested campaigns. It is written as the
            Registry snapshots each model per generation.
          </p>
        </Panel>
      ) : (
        <>
          <Panel ticks className="mb-4 p-6">
            <div className="grid grid-cols-2 gap-6 sm:grid-cols-4">
              <Readout label="Snapshots" accent value={models.length.toLocaleString()} sub="append-only, never overwritten" />
              <Readout label="Model kinds" value={kinds.length} sub={kinds.join(" · ")} />
              <Readout label="Campaigns" value={new Set(models.map((m) => m.campaign)).size} />
              <Readout label="Largest corpus" value={Math.max(...models.map((m) => m.trained_on)).toLocaleString()} sub="records at training time" />
            </div>
          </Panel>

          <div className="mb-4 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {latest.map((m) => (
              <Panel key={m.kind} className="p-5">
                <div className="flex items-center justify-between">
                  <span className="mono text-[14px] font-semibold text-[var(--os-ink)]">{m.kind}</span>
                  <Pill tone="confirmed">v{m.version}</Pill>
                </div>
                {KIND_ROLE[m.kind] && <p className="mt-2 text-[12.5px] leading-[1.5] text-[var(--os-muted)]">{KIND_ROLE[m.kind]}</p>}
                <div className="mono mt-3 space-y-1 border-t border-[var(--os-hair)] pt-2.5 text-[10.5px] text-[var(--os-faint)]">
                  <div>trained on <span className="text-[var(--os-ink)] tnum">{m.trained_on.toLocaleString()}</span> records</div>
                  <div>parent: {m.parent === null ? "none (first version)" : `v${m.parent}`}</div>
                  <div>instance {m.instance} · campaign {m.campaign_id} · gen {m.generation_id}</div>
                  <div>weights {(m.payload_bytes / 1024).toFixed(1)} kB (kept on disk)</div>
                </div>
              </Panel>
            ))}
          </div>

          <Panel className="overflow-hidden">
            <PanelHeader title="Lineage" status="real"
              sub={`newest 60 of ${models.length.toLocaleString()} snapshots · each row names the version it superseded`} />
            <div className="overflow-x-auto">
              <table className="w-full min-w-[820px] text-left">
                <caption className="sr-only">Model snapshots with kind, version, parent version, training corpus size and timestamp</caption>
                <thead>
                  <tr className="mono border-b border-[var(--os-hair)] text-[10.5px] uppercase tracking-[.08em] text-[var(--os-faint)]">
                    <th scope="col" className="px-4 py-2.5 font-normal">kind</th>
                    <th scope="col" className="px-4 py-2.5 font-normal">version</th>
                    <th scope="col" className="px-4 py-2.5 font-normal">parent</th>
                    <th scope="col" className="px-4 py-2.5 font-normal">trained on</th>
                    <th scope="col" className="px-4 py-2.5 font-normal">instance</th>
                    <th scope="col" className="px-4 py-2.5 font-normal">campaign/gen</th>
                    <th scope="col" className="px-4 py-2.5 font-normal">when</th>
                  </tr>
                </thead>
                <tbody>
                  {[...models].sort((a, b) => b.timestamp - a.timestamp || b.version - a.version).slice(0, 60).map((m, i) => (
                    <tr key={i} className="border-b border-[var(--os-hair)] text-[12px] last:border-0 hover:bg-[var(--os-raised)]">
                      <td className="mono px-4 py-2 text-[var(--os-ink)]">{m.kind}</td>
                      <td className="mono px-4 py-2 tnum text-[var(--os-accent)]">v{m.version}</td>
                      <td className="mono px-4 py-2 tnum text-[var(--os-faint)]">{m.parent === null ? "—" : `v${m.parent}`}</td>
                      <td className="mono px-4 py-2 tnum text-[var(--os-muted)]">{m.trained_on.toLocaleString()}</td>
                      <td className="mono px-4 py-2 text-[var(--os-muted)]">{m.instance}</td>
                      <td className="mono px-4 py-2 tnum text-[var(--os-faint)]">{m.campaign_id}/{m.generation_id}</td>
                      <td className="mono px-4 py-2 text-[var(--os-faint)]">{fmtTs(m.timestamp)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Panel>
        </>
      )}
    </div>
  );
}
