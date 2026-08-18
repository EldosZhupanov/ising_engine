import Applications from "@/components/os/workflow/Applications";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";
import { Eyebrow } from "@/components/os/kit";

export default function ApplicationsPage() {
  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/applications" />
      <div className="mb-6">
        <Eyebrow n="◈">Application Explorer</Eyebrow>
        <h1 className="max-w-[30ch] text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">
          A result travels exactly as far as its reduction and its evidence.
        </h1>
        <p className="mt-2 max-w-[80ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
          Every domain here carries a tier. A domain appears only if the engine actually ran it, or if a published
          reduction to QUBO/Ising exists — and in the second case the card states plainly what may <em>not</em> be
          concluded. Nothing is listed on plausibility alone.
        </p>
      </div>
      <Spine route="/applications" activeId="applications" />
      <Guide route="/applications" />
      <Applications />
    </div>
  );
}
