import { graph, overview, benchmarks, provenance } from "@/data";
import { APPLICATIONS } from "@/data/applications";
import { LEDGER } from "@/data/curated";
import Composer, { type Claim, type Bench } from "@/components/os/publish/Composer";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";
import { Eyebrow } from "@/components/os/kit";

export default function PublishPage() {
  // Claims come only from the recorded ledger — nothing invented.
  const claims: Claim[] = LEDGER.map((l, i) => ({
    id: `c${i}`, statement: l.claim, status: l.status, detail: l.detail, source: l.source,
  }));

  const benches: Bench[] = [];
  const seen = new Set<string>();
  for (const run of benchmarks.index?.runs ?? []) {
    for (const [engine, v] of Object.entries(run.per_engine)) {
      const h = v.head_to_head;
      if (!h) continue;
      const k = `${run.exp}|${engine}|${h.vs}`;
      if (seen.has(k)) continue;
      seen.add(k);
      benches.push({ exp: run.exp, engine, vs: h.vs, instances: h.instances, win: h.win_fraction, p: h.wilcoxon_p });
    }
  }

  const t = overview.totals;
  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/publish" />
      <Spine route="/publish" activeId="publication" />
      <div className="mb-5">
        <Eyebrow n="§">Publication Composer</Eyebrow>
        <h1 className="max-w-[32ch] text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">
          State the uncertainty, or it isn&rsquo;t a result.
        </h1>
        <p className="mt-2 max-w-[82ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
          Assemble a finding for an outside audience from artifacts that already exist. Every figure is transcluded from
          the append-only record and labelled with its source; the statistical power caveat is added automatically; the
          unknowns are a required field.
        </p>
      </div>
      <Guide route="/publish" />
      <Composer
        claims={claims}
        facts={graph.facts}
        ev={{ runsAll: t.runsAll, runsPrimary: t.runsPrimary, instances: t.instances, operators: t.operators, best: overview.best, primary: overview.primary }}
        benches={benches}
        applications={APPLICATIONS}
        reproSources={(provenance.sources ?? []).filter((s) => s.exists).map((s) => s.file)}
      />
    </div>
  );
}
