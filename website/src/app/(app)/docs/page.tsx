import { Eyebrow, Panel, PanelHeader, Pill, Icon } from "@/components/os/kit";
import Guide from "@/components/os/workflow/Guide";
import Spine from "@/components/os/workflow/Spine";
import { TrackVisit } from "@/components/os/workflow/Track";

/** Governance order — higher outranks lower. Real files in the repository. */
const GOVERNANCE = [
  { rank: 1, file: "research/ISING_ENGINE_CONSTITUTION.md", title: "Constitution",
    role: "The single source of truth for direction. Outranks every other document; direction changes only via its amendment procedure, backed by reproducible evidence." },
  { rank: 2, file: "SOUL.md", title: "Soul",
    role: "Why the project exists. When a rule and the mission conflict, the mission decides what the rule was protecting." },
  { rank: 3, file: "research/architecture/ADR/", title: "Architecture Decisions",
    role: "Queryable record of every structural decision and its rationale. Consult before architectural work; record new decisions as ADRs." },
  { rank: 4, file: "ROADMAP.md", title: "Roadmap",
    role: "Status: what exists, what is in flight, what is next — with per-stage completion and an honest confirmed/refuted ledger." },
  { rank: 5, file: "CLAUDE.md", title: "Operating rules",
    role: "The day-to-day rules: how work gets done, what may never be touched, and the verification gates." },
];

const ADRS = [
  ["ADR-0000", "ADR system and knowledge graph"],
  ["ADR-0001", "Operators, not algorithms"],
  ["ADR-0002", "Two state backends"],
  ["ADR-0003", "Cache locality outranks FLOPs"],
  ["ADR-0004", "Reproducibility mandatory"],
  ["ADR-0005", "Deterministic certified lowering"],
  ["ADR-0006", "Amendment: direction presumed fixed"],
  ["ADR-0007", "Four-track program"],
  ["ADR-0008", "Experiment infrastructure"],
];

const DESIGN = [
  ["website/PRODUCT_SPEC.md", "The product specification for this interface — personas, workflows, IA, per-screen design, and the REAL/OFFLINE/PLANNED contract."],
  ["website/ARCHITECTURE_REVIEW.md", "A brutal review of an earlier build of this interface, plus the verified environment findings that shaped it."],
  ["website/ARCHITECTURE_V2.md", "Architecture decisions AD-1…AD-4: in-process orchestrator, campaigns-as-directories, the epistemic firewall, evidence-tiered applications."],
  ["website/ARCHITECTURE_V3_ADDENDUM.md", "AD-5…AD-8: the Discovery Center from existing faculties, Git-for-science, publication, and the provider matrix."],
  ["website/references/analysis.md", "The design-reference study written before any interface code."],
];

export default function DocsPage() {
  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      <TrackVisit route="/docs" />
      <Spine route="/docs" activeId="publication" />
      <div className="mb-6">
        <Eyebrow n="§">Documentation</Eyebrow>
        <h1 className="max-w-[34ch] text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">
          A governed project — direction is fixed, details are earned by evidence.
        </h1>
        <p className="mt-2 max-w-[84ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
          These are real files in the repository, ordered by authority. The point of writing direction down and ranking it
          is that no single result — however exciting — can quietly redefine what the project is for.
        </p>
      </div>

      <Guide route="/docs" />

      <Panel className="mb-4">
        <PanelHeader title="Governance order" status="real" sub="higher outranks lower" />
        <div className="divide-y divide-[var(--os-hair)]">
          {GOVERNANCE.map((g) => (
            <div key={g.file} className="flex items-start gap-4 px-5 py-3.5">
              <span className="mono mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full border border-[var(--os-hair-2)] text-[11px] text-[var(--os-accent)]">{g.rank}</span>
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-baseline gap-2">
                  <span className="text-[13.5px] font-semibold text-[var(--os-ink)]">{g.title}</span>
                  <span className="mono text-[11px] text-[var(--os-faint)]">{g.file}</span>
                </div>
                <p className="mt-1 max-w-[86ch] text-[12.5px] leading-[1.55] text-[var(--os-muted)]">{g.role}</p>
              </div>
            </div>
          ))}
        </div>
      </Panel>

      <div className="grid gap-4 lg:grid-cols-2">
        <Panel>
          <PanelHeader title="Architecture decisions" status="real" sub="research/architecture/ADR/ — queryable" />
          <div className="divide-y divide-[var(--os-hair)]">
            {ADRS.map(([id, title]) => (
              <div key={id} className="flex items-center gap-3 px-5 py-2.5">
                <Icon name="FileText" size={14} className="shrink-0 text-[var(--os-faint)]" />
                <span className="mono text-[11.5px] text-[var(--os-accent)]">{id}</span>
                <span className="text-[12.5px] text-[var(--os-muted)]">{title}</span>
              </div>
            ))}
          </div>
          <div className="border-t border-[var(--os-hair)] px-5 py-3">
            <p className="mono text-[10.5px] text-[var(--os-faint)]">
              ADR-0004 (reproducibility mandatory) is the one that makes Replay possible at all.
            </p>
          </div>
        </Panel>

        <Panel>
          <PanelHeader title="This interface's own design record" status="real" sub="written before the code, kept after it" />
          <div className="divide-y divide-[var(--os-hair)]">
            {DESIGN.map(([file, role]) => (
              <div key={file} className="px-5 py-3">
                <div className="mono text-[11.5px] text-[var(--os-ink)]">{file}</div>
                <p className="mt-1 text-[12px] leading-[1.5] text-[var(--os-muted)]">{role}</p>
              </div>
            ))}
          </div>
        </Panel>
      </div>

      <Panel className="mt-4 p-5">
        <div className="mb-2"><Pill tone="open">how to run it</Pill></div>
        <div className="space-y-3">
          <div>
            <div className="mono text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">a campaign from the CLI</div>
            <pre className="mono mt-1 overflow-x-auto rounded-[8px] border border-[var(--os-hair)] bg-[var(--os-ground)] p-3 text-[11px] text-[var(--os-ink)]">cargo run --release --bin research_platform -- \
    --file benchmark_suite/data/gset/G11 --campaigns 2 --generations 4</pre>
          </div>
          <div>
            <div className="mono text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">the control plane (unlocks replay, telemetry, campaign control here)</div>
            <pre className="mono mt-1 overflow-x-auto rounded-[8px] border border-[var(--os-hair)] bg-[var(--os-ground)] p-3 text-[11px] text-[var(--os-ink)]">cargo run --release --bin control_api</pre>
          </div>
          <div>
            <div className="mono text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">verification gates (release — see the skill for why)</div>
            <pre className="mono mt-1 overflow-x-auto rounded-[8px] border border-[var(--os-hair)] bg-[var(--os-ground)] p-3 text-[11px] text-[var(--os-ink)]">cargo verify        # test --release
cargo lint          # clippy --release -- -D warnings
cargo fmt --check</pre>
          </div>
        </div>
      </Panel>
    </div>
  );
}
