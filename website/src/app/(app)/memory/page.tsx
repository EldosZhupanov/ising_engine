import { experiments } from "@/data";
import Memory from "@/components/os/control/Memory";
import { Eyebrow } from "@/components/os/kit";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";

export default function MemoryPage() {
  const instances = experiments.rollups.byInstance
    .map((i) => i.key)
    .sort((a, b) => a.localeCompare(b, undefined, { numeric: true }));

  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/memory" />
      <Spine route="/memory" activeId="knowledge" />
      <div className="mb-5">
        <Eyebrow n="◉">Scientific Memory</Eyebrow>
        <h1 className="max-w-[34ch] text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">
          &ldquo;Have I seen a structure like this before?&rdquo;
        </h1>
        <p className="mt-2 max-w-[84ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
          This is the first stage of every research tick. Before the platform plans anything it searches the entire
          append-only record for structurally similar instances, ranks what actually worked on them, and gathers the
          conditional facts whose condition applies here. Recall is computed on demand from the live store — it is the one
          workspace that cannot be served from a snapshot.
        </p>
      </div>
      <Guide route="/memory" />
      <Memory instances={instances} />
    </div>
  );
}
