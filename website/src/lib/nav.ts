/** The Ising Engine OS sitemap — grouped rail, with honest per-item status. */

export type Status = "real" | "offline" | "planned";
export type NavItem = { label: string; href: string; icon: string; status: Status; note?: string };
export type NavGroup = { group: string; items: NavItem[] };

export const NAV: NavGroup[] = [
  {
    group: "Overview",
    items: [
      { label: "Mission Control", href: "/", icon: "LayoutDashboard", status: "real" },
      { label: "Research Path", href: "/start", icon: "Compass", status: "real", note: "The arc from opening the platform to discovering an algorithm." },
    ],
  },
  {
    group: "Research",
    items: [
      { label: "Discovery Center", href: "/discover", icon: "Sparkles", status: "real", note: "Gaps the engine mined from its own record." },
      { label: "Experiment Designer", href: "/design", icon: "FlaskConical", status: "real", note: "Hypothesis + kill-criterion + a verified reproduce command." },
      { label: "Publication", href: "/publish", icon: "ScrollText", status: "real", note: "Assemble a finding with its evidence, confidence and unknowns." },
      { label: "Applications", href: "/applications", icon: "Waypoints", status: "real", note: "Reduction atlas, evidence-gated." },
      { label: "Playbooks", href: "/playbooks", icon: "BookOpen", status: "real", note: "Method, encoded as procedures." },
    ],
  },
  {
    group: "Evidence",
    items: [
      { label: "Experiments", href: "/experiments", icon: "Activity", status: "real" },
      { label: "Benchmarks", href: "/benchmarks", icon: "GitCompare", status: "real" },
      { label: "Dataset", href: "/dataset", icon: "Database", status: "real" },
      { label: "Evaluation", href: "/evaluation", icon: "ClipboardCheck", status: "real" },
    ],
  },
  {
    group: "Knowledge",
    items: [
      { label: "Knowledge Graph", href: "/graph", icon: "Waypoints", status: "real" },
      { label: "Theory Engine", href: "/theories", icon: "Atom", status: "real" },
      { label: "Concept Evolution", href: "/concepts", icon: "Sparkles", status: "real" },
      { label: "Research Papers", href: "/papers", icon: "FileText", status: "real" },
      { label: "Scientific Memory", href: "/memory", icon: "Brain", status: "real", note: "Live recall computed from the append-only record via the control plane." },
    ],
  },
  {
    group: "Machinery",
    items: [
      { label: "Operators", href: "/operators", icon: "Boxes", status: "real" },
      { label: "Runtime & Backends", href: "/runtime", icon: "Cpu", status: "real" },
      { label: "Model Registry", href: "/models", icon: "Layers", status: "real" },
      { label: "Feature Registry", href: "/features", icon: "Binary", status: "real" },
    ],
  },
  {
    group: "Operate",
    items: [
      { label: "LLM Control Center", href: "/llm", icon: "BrainCircuit", status: "real", note: "Live: talks directly to Ollama's HTTP API (CORS-verified)." },
      { label: "Campaign Manager", href: "/campaigns", icon: "Radar", status: "real", note: "Live: start/pause/step/stop the orchestrator via the control plane." },
      { label: "Solver Playground", href: "/solve", icon: "Play", status: "real", note: "Posts to the production solver (server_api /api/v1/solve)." },
    ],
  },
  {
    group: "System",
    items: [
      { label: "Documentation", href: "/docs", icon: "BookOpen", status: "real" },
      { label: "Settings", href: "/settings", icon: "Settings", status: "real" },
    ],
  },
];

export const ALL_ITEMS = NAV.flatMap((g) => g.items);
export const STATUS_LABEL: Record<Status, string> = { real: "REAL", offline: "OFFLINE", planned: "PLANNED" };
