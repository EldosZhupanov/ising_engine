import Link from "next/link";
import { papers, provenance } from "@/data";
import { Eyebrow, Panel, PanelHeader, ProvenanceTag, NotLive } from "@/components/os/kit";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";

export default async function Papers({ searchParams }: { searchParams: Promise<{ p?: string }> }) {
  const sp = await searchParams;
  const src = provenance.sources?.find((s) => s.label.includes("papers"));
  if (!papers.length) return <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/papers" />
      <Spine route="/papers" />
      <Guide route="/papers" /><Eyebrow n="06">Research Papers</Eyebrow><NotLive status="planned" note="No reports/ ingested." /></div>;
  const idx = Math.min(papers.length - 1, Math.max(0, sp.p ? +sp.p : papers.length - 1));
  const paper = papers[idx];

  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/papers" />
      <Spine route="/papers" />
      <Guide route="/papers" />
      <div className="mb-6 flex flex-wrap items-end justify-between gap-3">
        <div>
          <Eyebrow n="06">Research Papers</Eyebrow>
          <h1 className="text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">The platform&rsquo;s cumulative written memory.</h1>
          <p className="mono mt-2 text-[12px] text-[var(--os-faint)]">Numbered reports, never rewritten — reports.rs</p>
        </div>
        <ProvenanceTag file={src?.file} rows={src?.rows} mtime={src?.mtime} />
      </div>

      <div className="grid gap-4 lg:grid-cols-[260px_1fr]">
        <Panel className="h-max">
          <PanelHeader title="Reports" status="real" />
          <div className="divide-y divide-[var(--os-hair)]">
            {papers.map((p, i) => (
              <Link key={p.file} href={`/papers?p=${i}`} className={`block px-4 py-3 ${i === idx ? "bg-[var(--os-accent-soft)]" : "hover:bg-[var(--os-raised)]"}`}>
                <div className="mono text-[12px] text-[var(--os-ink)]">{p.file}</div>
                <div className="mono text-[11px] text-[var(--os-faint)]">{p.experiments?.toLocaleString()} experiments · {p.discoveries.length} findings</div>
              </Link>
            ))}
          </div>
        </Panel>

        <Panel>
          <PanelHeader title={`Report — ${paper.experiments?.toLocaleString()} experiments`} status="real" sub={paper.best ?? undefined} />
          <div className="p-5">
            <div className="mono mb-3 text-[11px] uppercase tracking-[.14em] text-[var(--os-muted)]">Discoveries</div>
            <div className="space-y-2">
              {paper.discoveries.map((d, i) => (
                <div key={i} className="flex items-start gap-3 rounded-[9px] border border-[var(--os-hair)] bg-[var(--os-ground-2)] px-3.5 py-2.5">
                  <span className="mono mt-0.5 text-[10px] text-[var(--os-faint)]">{String(i + 1).padStart(2, "0")}</span>
                  <p className="flex-1 text-[13px] leading-[1.5] text-[var(--os-ink)]">{d.text}</p>
                  <span className="mono shrink-0 text-[10.5px] text-[var(--os-faint)]">support {d.support} · conf {d.confidence.toFixed(2)}</span>
                </div>
              ))}
            </div>
          </div>
        </Panel>
      </div>
    </div>
  );
}
