"use client";

/**
 * Progress instrumentation.
 *
 * `<TrackVisit>` records that a workspace was opened; `<TrackButton>` wraps a
 * real control so that *doing the thing* completes the mission. Missions are
 * never completed by a "next" button — only by the action they describe.
 */

import { useEffect } from "react";
import { recordAction, recordVisit } from "@/lib/workflow/progress";
import type { ActionEvent } from "@/lib/workflow/curriculum";

export function TrackVisit({ route }: { route: string }) {
  useEffect(() => { recordVisit(route); }, [route]);
  return null;
}

/** Fire-and-forget action recorder for non-button interactions. */
export function useRecord() {
  return recordAction;
}

export function TrackButton({
  event, onClick, children, className, ariaLabel,
}: {
  event: ActionEvent; onClick?: () => void; children: React.ReactNode; className?: string; ariaLabel?: string;
}) {
  return (
    <button aria-label={ariaLabel} className={className}
      onClick={() => { recordAction(event); onClick?.(); }}>
      {children}
    </button>
  );
}

export { recordAction };
