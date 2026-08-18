import Playbooks from "@/components/os/workflow/Playbooks";
import { TrackVisit } from "@/components/os/workflow/Track";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";

export default function PlaybooksPage() {
  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/playbooks" />
      <Spine route="/playbooks" />
      <Guide route="/playbooks" />
      <Playbooks />
    </div>
  );
}
