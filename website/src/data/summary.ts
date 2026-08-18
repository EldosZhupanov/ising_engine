/**
 * Client-safe data surface.
 *
 * Client components must import from HERE, never from `@/data` — that barrel
 * pulls every generated JSON (including the ~1.2 MB experiments sample and the
 * papers markdown) into the client bundle. This module imports only the two
 * small files (overview + provenance, a few kB) that chrome actually needs.
 */
import overviewJson from "./generated/overview.json";
import provenanceJson from "./generated/provenance.json";

export type SummaryTotals = {
  runsPrimary: number; runsAll: number; instances: number;
  operators: number; backends: string[]; generations: number;
};

const ov = overviewJson as unknown as { primary: string; totals: SummaryTotals };
const pv = provenanceJson as unknown as { fixture?: boolean; primary?: string };

export const summary = {
  primary: ov.primary,
  totals: ov.totals,
};
export const dataAvailable = !(overviewJson as { fixture?: boolean }).fixture && !pv.fixture;
