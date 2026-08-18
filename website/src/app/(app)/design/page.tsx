import { experiments, operators } from "@/data";
import Designer from "@/components/os/design/Designer";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";
import { Eyebrow } from "@/components/os/kit";

export default async function DesignPage({ searchParams }: { searchParams: Promise<{ seed?: string }> }) {
  const sp = await searchParams;
  // instance sizes come from the real sample rows
  const nByInstance = new Map<string, number>();
  for (const r of experiments.sample) if (!nByInstance.has(r.instance)) nByInstance.set(r.instance, r.n);
  const instances = experiments.rollups.byInstance
    .map((i) => ({ key: i.key, runs: i.runs, best: i.best, n: nByInstance.get(i.key) }))
    .sort((a, b) => a.key.localeCompare(b.key, undefined, { numeric: true }));

  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/design" />
      <Spine route="/design" activeId="hypothesis" />
      <div className="mb-5">
        <Eyebrow n="✦">Experiment Designer</Eyebrow>
        <h1 className="max-w-[32ch] text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">
          A hypothesis that cannot fail is not science.
        </h1>
        <p className="mt-2 max-w-[82ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
          State a falsifiable claim, fix the kill-criterion <em>before</em> the run, then specify the experiment down to
          the seed. The output is a spec in the repository&rsquo;s own schema and a command whose every flag was verified
          against <span className="mono">src/bin/research_platform.rs</span>.
        </p>
      </div>
      <Guide route="/design" />
      <Designer instances={instances} operators={[...operators.operators]} gapSeed={sp.seed} />
    </div>
  );
}
