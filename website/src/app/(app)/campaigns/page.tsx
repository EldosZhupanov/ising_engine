import { experiments } from "@/data";
import CampaignManager from "@/components/os/control/CampaignManager";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";
import { Eyebrow } from "@/components/os/kit";

export default function CampaignsPage() {
  // Real instance names from the record, so the launcher can only pick things
  // that actually exist on disk.
  const instances = experiments.rollups.byInstance
    .map((i) => i.key)
    .sort((a, b) => a.localeCompare(b, undefined, { numeric: true }));

  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/campaigns" />
      <Spine route="/campaigns" activeId="experiment" />
      <div className="mb-5">
        <Eyebrow n="▶">Campaign Manager</Eyebrow>
        <h1 className="max-w-[32ch] text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">
          Start the autonomous scientist — then watch it think.
        </h1>
        <p className="mt-2 max-w-[84ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
          The control plane owns the orchestrator and drives its <span className="mono">tick()</span> loop under
          supervision, so the loop can be paused between ticks and <em>stepped</em> one tick at a time — neither of which
          a spawned CLI process could offer. Every figure in the log below comes from a completed tick.
        </p>
      </div>
      <Guide route="/campaigns" />
      <CampaignManager instances={instances} />
    </div>
  );
}
