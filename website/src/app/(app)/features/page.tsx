import { operators } from "@/data";
import { CONCEPT_PIPELINE } from "@/data/curated";
import { Eyebrow, Panel, PanelHeader, Pill, Icon } from "@/components/os/kit";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";

/** The structural signature every faculty reads (InstanceSignature in the engine). */
const SIGNATURE = [
  { name: "n", meaning: "problem size — number of spins/variables" },
  { name: "density", meaning: "edge density of the coupling graph" },
  { name: "clustering", meaning: "local clustering coefficient — how tightly neighbourhoods close" },
  { name: "mean_degree", meaning: "average node degree" },
  { name: "degree_cv", meaning: "coefficient of variation of degree — heterogeneity of the structure" },
];

export default function FeaturesPage() {
  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/features" />
      <Spine route="/features" activeId="knowledge" />
      <div className="mb-6">
        <Eyebrow n="◇">Feature Registry</Eyebrow>
        <h1 className="max-w-[32ch] text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">
          One shared language of thought, versioned so it can grow safely.
        </h1>
        <p className="mt-2 max-w-[84ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
          Predictor, World, Policy and Dynamics all read the <em>same</em> vocabulary. Before the registry existed each
          copied its own encoding, so a change in one silently diverged from the others. Now the vocabulary is a single
          versioned artifact: <span className="mono text-[var(--os-ink)]">v0</span> encodes exactly as it did the day it
          was frozen, and growth is additive.
        </p>
      </div>

      <Guide route="/features" />

      <div className="grid gap-4 lg:grid-cols-[1.1fr_1fr]">
        <Panel>
          <PanelHeader title="The structural signature" status="real" sub="what every model sees about a problem" />
          <div className="divide-y divide-[var(--os-hair)]">
            {SIGNATURE.map((f) => (
              <div key={f.name} className="flex items-baseline gap-3 px-5 py-2.5">
                <span className="mono w-[104px] shrink-0 text-[12.5px] text-[var(--os-accent)]">{f.name}</span>
                <span className="text-[12.5px] leading-[1.5] text-[var(--os-muted)]">{f.meaning}</span>
              </div>
            ))}
          </div>
          <div className="border-t border-[var(--os-hair)] px-5 py-3.5">
            <p className="mono text-[10.5px] leading-[1.5] text-[var(--os-faint)]">
              These five scalars are the base vocabulary. Concept Discovery may add derived features (e.g. an
              interaction the linear vocabulary cannot express) — but only if they earn their place.
            </p>
          </div>
        </Panel>

        <Panel>
          <PanelHeader title="Guarantees" status="real" sub="why the vocabulary can grow without breaking models" />
          <ul className="space-y-2.5 p-5">
            {[
              ["Versioned", "v0 encodes bit-identically forever, so a model trained then still means the same thing now."],
              ["Shared", "one vocabulary across Predictor, World, Policy and Dynamics — no divergent private copies."],
              ["Additive", "a new feature appends; it never renumbers or redefines an existing one."],
              ["Gated", "a candidate is admitted only if it improves out-of-sample prediction past an Occam penalty."],
              ["Replay-safe", "the same registry version produces the same encoding, so runs stay reproducible."],
            ].map(([t, d]) => (
              <li key={t} className="flex gap-2.5">
                <Icon name="Check" size={15} className="mt-0.5 shrink-0 text-[var(--os-accent)]" />
                <span className="text-[13px] leading-[1.55] text-[var(--os-muted)]">
                  <span className="text-[var(--os-ink)]">{t}.</span> {d}
                </span>
              </li>
            ))}
          </ul>
        </Panel>
      </div>

      <Panel className="mt-4">
        <PanelHeader title="How a concept enters the vocabulary" status="real" sub="ai_scientist/concept.rs — propose → test → gate → admit" />
        <div className="grid gap-3 p-5 md:grid-cols-2 xl:grid-cols-4">
          {CONCEPT_PIPELINE.map((s, i) => (
            <div key={s.stage} className="os-raised rounded-[12px] p-4">
              <div className="mono text-[10px] text-[var(--os-faint)]">{String(i + 1).padStart(2, "0")}</div>
              <div className="mt-1 text-[13.5px] font-semibold text-[var(--os-ink)]">{s.stage}</div>
              <p className="mt-1.5 text-[12px] leading-[1.5] text-[var(--os-muted)]">{s.desc}</p>
            </div>
          ))}
        </div>
      </Panel>

      <Panel className="mt-4">
        <PanelHeader title="Operator vocabulary" status="real" sub={`the ${operators.operators.length} tokens a Policy can emit`} />
        <div className="flex flex-wrap gap-1.5 p-5">
          {operators.operators.map((o) => (
            <span key={o} className="mono rounded-[7px] border border-[var(--os-hair-2)] bg-[var(--os-ground-2)] px-2 py-1 text-[11.5px] text-[var(--os-muted)]">{o}</span>
          ))}
        </div>
      </Panel>

      <Panel className="mt-4 p-5">
        <div className="mb-2"><Pill tone="neutral">not yet observable</Pill></div>
        <p className="max-w-[86ch] text-[13px] leading-[1.6] text-[var(--os-muted)]">
          The live registry contents (which concepts have actually been admitted, and each one&rsquo;s out-of-sample gain)
          are held in memory by <span className="mono text-[var(--os-ink)]">feature_registry.rs</span> and never
          serialised. Until that export exists this page describes the mechanism and the base vocabulary — it does not
          claim to list admitted concepts.
        </p>
      </Panel>
    </div>
  );
}
