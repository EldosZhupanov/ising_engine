import { operators, behaviour, provenance } from "@/data";
import { BACKENDS } from "@/data/curated";
import { Eyebrow, Panel, PanelHeader, Pill, ProvenanceTag, Bar } from "@/components/os/kit";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";

/** Family → how to read it, so a signature is never just a label. */
const FAMILY_MEANING: Record<string, string> = {
  Ordering: "position-dependent — behaves differently depending on what ran before it",
  Budget: "budget-dependent — its value changes with the sweep allowance",
  Antipattern: "over-represented in the WORST solutions — a weak choice under these conditions",
  Temperature: "temperature-dependent — the thermal regime changes its effect",
};
const FAMILY_TONE: Record<string, "confirmed" | "refuted" | "open" | "neutral"> = {
  Ordering: "open", Budget: "confirmed", Antipattern: "refuted", Temperature: "open",
};

export default function OperatorsPage() {
  const src = provenance.sources?.find((s) => s.label.endsWith(":db"));
  const evalSrc = provenance.sources?.find((s) => s.label.includes("evaluation"));
  const rollup = operators.rollup;
  const maxRuns = Math.max(...rollup.map((r) => r.runs), 1);

  // Order by how much the record actually knows about each operator.
  const ordered = [...operators.operators].sort((a, b) => {
    const score = (k: string) => {
      const x = behaviour[k];
      return x ? x.signatures.length * 3 + x.facts.length + x.discoveries.length : 0;
    };
    return score(b) - score(a);
  });

  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/operators" />
      <Spine route="/operators" activeId="knowledge" />
      <div className="mb-6 flex flex-wrap items-end justify-between gap-3">
        <div>
          <Eyebrow n="⚙">Operator Registry</Eyebrow>
          <h1 className="max-w-[32ch] text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">
            Fourteen physical laws — and what the record learned about each.
          </h1>
          <p className="mt-2 max-w-[84ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
            Operators are selected by <span className="text-[var(--os-ink)]">capability passport</span>, never by name, so a
            new one is usable the moment it is registered. Everything below each operator is <em>discovered</em>, not
            declared: usage from the append-only record, mined rule signatures from cross-instance evaluation, typed graph
            facts, and the report lines that named it.
          </p>
        </div>
        <span className="flex flex-wrap items-center gap-2">
          <ProvenanceTag file={src?.file} rows={src?.rows} mtime={src?.mtime} />
          <ProvenanceTag file={evalSrc?.file} rows={evalSrc?.rows} />
        </span>
      </div>

      <Guide route="/operators" />

      {/* usage across the record */}
      <Panel className="mb-4">
        <PanelHeader title="Usage across the record" status="real"
          sub={`${rollup.length} operators · runs and mean Δ (score − baseline; negative is better)`} />
        <div className="px-5 py-4">
          {rollup.map((r) => (
            <Bar key={r.key} label={r.key} frac={r.runs / maxRuns} value={r.runs.toLocaleString()}
              tone={r.meanImpr < 0 ? "cyan" : "accent"} />
          ))}
          <p className="mono mt-3 border-t border-[var(--os-hair)] pt-2.5 text-[10.5px] leading-[1.5] text-[var(--os-faint)]">
            Run count is exposure, not quality — the evolution engine tries promising operators more often, so a long bar
            reflects the search&rsquo;s attention. Read the signatures below for whether it actually helps.
          </p>
        </div>
      </Panel>

      {/* per-operator profiles */}
      <div className="space-y-3">
        {ordered.map((op) => {
          const b = behaviour[op];
          const r = b?.rollup;
          const families = [...new Set((b?.signatures ?? []).map((s) => s.family))];
          const isAntipattern = families.includes("Antipattern");
          const known = (b?.signatures.length ?? 0) + (b?.facts.length ?? 0) + (b?.discoveries.length ?? 0);
          return (
            <Panel key={op} ticks>
              <PanelHeader
                title={op}
                status="real"
                sub={r ? `${r.runs.toLocaleString()} runs · mean Δ ${r.meanImpr > 0 ? "+" : ""}${r.meanImpr} · best ${r.best?.toLocaleString() ?? "—"}` : "no recorded runs"}
                right={
                  <span className="flex flex-wrap items-center gap-1.5">
                    {isAntipattern && <Pill tone="refuted">antipattern</Pill>}
                    {families.filter((f) => f !== "Antipattern").map((f) => (
                      <Pill key={f} tone={FAMILY_TONE[f] ?? "neutral"}>{f.toLowerCase()}</Pill>
                    ))}
                    {known === 0 && <Pill tone="neutral">no mined rules</Pill>}
                  </span>
                }
              />
              <div className="grid gap-5 p-5 lg:grid-cols-2">
                {/* mined signatures */}
                <div>
                  <div className="mono mb-2 text-[10px] uppercase tracking-[.14em] text-[var(--os-accent)]">
                    mined rule signatures
                  </div>
                  {b?.signatures.length ? (
                    <div className="space-y-2">
                      {b.signatures.map((s, i) => (
                        <div key={i} className="rounded-[8px] border border-[var(--os-hair)] bg-[var(--os-ground-2)] px-3 py-2">
                          <div className="flex flex-wrap items-center gap-2">
                            <span className="mono text-[11.5px] text-[var(--os-ink)]">{s.signature}</span>
                            <span className="mono text-[10px] tnum text-[var(--os-faint)]">
                              {s.instances} inst · conf {s.confidence.toFixed(2)}
                            </span>
                          </div>
                          <p className="mt-1 text-[12px] leading-[1.5] text-[var(--os-muted)]">{s.example}</p>
                          {FAMILY_MEANING[s.family] && (
                            <p className="mono mt-1 text-[10px] leading-[1.45] text-[var(--os-faint)]">
                              {FAMILY_MEANING[s.family]}
                            </p>
                          )}
                        </div>
                      ))}
                    </div>
                  ) : (
                    <p className="mono text-[11.5px] leading-[1.5] text-[var(--os-faint)]">
                      No rule reproduced across instances for this operator. That is a gap, not a verdict — it may simply
                      be under-sampled.
                    </p>
                  )}
                </div>

                {/* graph facts + report lines */}
                <div className="space-y-4">
                  <div>
                    <div className="mono mb-2 text-[10px] uppercase tracking-[.14em] text-[var(--os-muted)]">
                      typed graph facts
                    </div>
                    {b?.facts.length ? (
                      <ul className="space-y-1">
                        {b.facts.map((f, i) => (
                          <li key={i} className="mono flex flex-wrap items-baseline gap-x-2 text-[11.5px]">
                            <span style={{ color: f.relation === "fails-on" ? "var(--os-refuted)" : f.relation === "effective-on" ? "var(--os-confirmed)" : "var(--os-cyan)" }}>
                              {f.relation}
                            </span>
                            <span className="text-[var(--os-ink)]">{f.object || "—"}</span>
                            <span className="tnum text-[var(--os-faint)]">
                              w {f.weight >= 0 ? "+" : ""}{f.weight.toFixed(1)} · s{f.support}
                              {f.confidence !== null ? ` · conf ${f.confidence.toFixed(2)}` : ""}
                            </span>
                          </li>
                        ))}
                      </ul>
                    ) : (
                      <p className="mono text-[11.5px] text-[var(--os-faint)]">no facts with this operator as subject</p>
                    )}
                  </div>

                  {b?.discoveries.length ? (
                    <div>
                      <div className="mono mb-2 text-[10px] uppercase tracking-[.14em] text-[var(--os-muted)]">
                        named in the newest report
                      </div>
                      <ul className="space-y-1.5">
                        {b.discoveries.map((d, i) => (
                          <li key={i} className="text-[12px] leading-[1.5] text-[var(--os-muted)]">
                            · {d.text}{" "}
                            <span className="mono text-[10px] text-[var(--os-faint)]">
                              (s{d.support} · {d.confidence.toFixed(2)})
                            </span>
                          </li>
                        ))}
                      </ul>
                    </div>
                  ) : null}
                </div>
              </div>
            </Panel>
          );
        })}
      </div>

      {/* backends the operators run on */}
      <Panel className="mt-4">
        <PanelHeader title="Backends these operators run on" status="real" sub="engine_v2/state.rs — selected by the Decision Engine" />
        <div className="grid gap-3 p-5 md:grid-cols-3">
          {BACKENDS.map((b) => (
            <div key={b.name} className="os-raised rounded-[12px] p-4">
              <div className="flex items-center justify-between">
                <span className="mono text-[13px] text-[var(--os-ink)]">{b.name}</span>
                <Pill tone="neutral">{b.tag}</Pill>
              </div>
              <p className="mt-2 text-[12px] leading-[1.5] text-[var(--os-muted)]">{b.role}</p>
            </div>
          ))}
        </div>
      </Panel>
    </div>
  );
}
