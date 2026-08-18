import Settings from "@/components/os/control/Settings";
import { Eyebrow } from "@/components/os/kit";
import Guide from "@/components/os/workflow/Guide";
import { TrackVisit } from "@/components/os/workflow/Track";

export default function SettingsPage() {
  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/settings" />
      <div className="mb-6">
        <Eyebrow n="⚙">Settings</Eyebrow>
        <h1 className="max-w-[32ch] text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">
          Local preferences — nothing leaves this browser.
        </h1>
      </div>
      <Guide route="/settings" />
      <Settings />
    </div>
  );
}
