import Solver from "@/components/os/control/Solver";
import { Eyebrow } from "@/components/os/kit";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";

export default function SolvePage() {
  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/solve" />
      <Spine route="/solve" activeId="experiment" />
      <div className="mb-6">
        <Eyebrow n="▷">Solver Playground</Eyebrow>
        <h1 className="max-w-[32ch] text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">
          Send a problem to the production solver and read the real energy back.
        </h1>
        <p className="mt-2 max-w-[84ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
          This is not a demo path: it posts to <span className="mono">server_api</span>&rsquo;s{" "}
          <span className="mono">/api/v1/solve</span>, the same semaphore-guarded production solver the platform ships.
          Small hand-checkable instances are provided so you can verify the answer yourself.
        </p>
      </div>
      <Guide route="/solve" />
      <Solver />
    </div>
  );
}
