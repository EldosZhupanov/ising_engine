/**
 * Typed client for the control plane (`src/bin/control_api.rs`).
 *
 * The laboratory works entirely without this: every workspace renders the
 * build-time snapshot. When the control plane IS running, surfaces that were
 * honestly marked OFFLINE become live — system telemetry, data freshness, and
 * bit-identical replay.
 *
 * Nothing here invents a fallback value. If the plane is unreachable the caller
 * gets `null`/throws and the UI says "offline", never a plausible-looking number.
 */

export const DEFAULT_CONTROL_URL = "http://127.0.0.1:7878";
const KEY = "ising.controlUrl";

export const loadControlUrl = (): string => {
  if (typeof window === "undefined") return DEFAULT_CONTROL_URL;
  try { return window.localStorage.getItem(KEY) || DEFAULT_CONTROL_URL; } catch { return DEFAULT_CONTROL_URL; }
};
export const saveControlUrl = (u: string) => {
  try { window.localStorage.setItem(KEY, u); } catch { /* private mode */ }
};

export type Health = {
  ok: boolean; service: string; dir: string; dir_exists: boolean;
  instances_dir: string; endpoints: string[];
};

export type Gpu = { name: string; memory_total_mib: number; memory_used_mib: number; utilization_pct: number };
export type SystemInfo = {
  cpu_threads: number;
  mem_total_kb: number | null;
  mem_available_kb: number | null;
  load_avg_1m: number | null;
  gpus: Gpu[];
};

export type DbStats = {
  dir: string; experiments: number; knowledge_facts: number;
  reports: number; proposals: number; db_mtime: number | null; max_id: number | null;
};

export type ReplayResult = {
  id: number; instance: string; instance_file: string;
  operators: string[]; sweeps: number[]; temp_hi: number; temp_lo: number;
  replicas: number; seed: number;
  recorded_score: number; replayed_score: number;
  identical: boolean; delta: number;
  backend_used: string; wall_ms: number; note: string;
};

async function req<T>(base: string, path: string, init?: RequestInit, timeoutMs = 10_000): Promise<T> {
  const ctl = new AbortController();
  const t = setTimeout(() => ctl.abort(), timeoutMs);
  try {
    const r = await fetch(`${base.replace(/\/$/, "")}${path}`, { ...init, signal: ctl.signal });
    if (!r.ok) {
      const body = await r.text().catch(() => "");
      throw new Error(`HTTP ${r.status}${body ? `: ${body.slice(0, 200)}` : ""}`);
    }
    return (await r.json()) as T;
  } finally { clearTimeout(t); }
}

export const getHealth = (base: string) => req<Health>(base, "/health", undefined, 3500);
export const getSystem = (base: string) => req<SystemInfo>(base, "/api/system", undefined, 6000);
export const getDbStats = (base: string) => req<DbStats>(base, "/api/db/stats", undefined, 8000);

/** Replay a recorded experiment. Slow by nature — the Runtime actually runs. */
export const postReplay = (base: string, id: number) =>
  req<ReplayResult>(base, "/api/replay", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ id }),
  }, 300_000);

export const fmtKb = (kb?: number | null) => {
  if (!kb) return "—";
  const gb = kb / 1024 / 1024;
  return gb >= 1 ? `${gb.toFixed(1)} GB` : `${(kb / 1024).toFixed(0)} MB`;
};
export const fmtMib = (mib?: number) => (mib == null ? "—" : mib >= 1024 ? `${(mib / 1024).toFixed(1)} GB` : `${mib} MiB`);

/* ───────────────────────── campaign lifecycle ───────────────────────── */

export type Phase = "idle" | "running" | "paused" | "finished" | "failed";

export type TickEvent = {
  tick: number; instance: string;
  experiments_before: number; experiments_after: number;
  best_score: number; best_schedule: string[]; baseline: number;
  theories_published: number; concepts: string[];
  health: string; recall: string; directives: string[]; wall_ms: number;
};

export type CampaignStatus = {
  phase: Phase; dir: string;
  ticks_done: number; max_ticks: number;
  instances: string[]; next_instance: string | null;
  planner_driven: boolean; executive_driven: boolean;
  started_unix: number | null; last_error: string | null;
  events: TickEvent[];
};

export type StartCampaign = {
  instances: string[];
  max_ticks?: number;
  generations?: number;
  planner_driven?: boolean;
  executive_driven?: boolean;
  shared_knowledge?: boolean;
  curiosity_lambda?: number;
  seed?: number;
  start_paused?: boolean;
};

export const getCampaign = (base: string) =>
  req<CampaignStatus>(base, "/api/campaign/status", undefined, 8000);

export const startCampaign = (base: string, body: StartCampaign) =>
  req<CampaignStatus>(base, "/api/campaign/start", {
    method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body),
  }, 20_000);

/** pause | resume | step | stop. `stop` joins the worker, so allow time. */
export const campaignAction = (base: string, action: "pause" | "resume" | "step" | "stop") =>
  req<CampaignStatus>(base, `/api/campaign/${action}`, { method: "POST" }, 300_000);

/* ───────────────────── solver (existing server_api) ───────────────────── */

export const DEFAULT_SOLVER_URL = "http://127.0.0.1:8080";

/** Matches `QuboResponse` in src/bin/server_api.rs exactly. */
export type SolveResponse = {
  status: string;
  energy: number;
  state: number[];          // Vec<i8> on the wire
  compute_time_ms: number;  // u128 on the wire
};

/** Matches `QuboRequest` — `num_vars` is required. */
export type SolveRequest = {
  num_vars: number;
  linear: number[];
  quadratic: [number, number, number][];
  sweeps?: number;
  replicas?: number;
  exchanges?: number;
  seed?: number;
};

/** POST a QUBO to the production solver (`src/bin/server_api.rs`). */
export async function postSolve(base: string, body: SolveRequest): Promise<SolveResponse> {
  return req<SolveResponse>(base, "/api/v1/solve", {
    method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body),
  }, 120_000);
}

export const solverHealth = (base: string) => req<unknown>(base, "/health", undefined, 3000);

/* ───────────────────────── scientific memory ───────────────────────── */

export type MemoryBucket = {
  label: string; experiments: number; instances: number;
  compactable: boolean; dominant_operator: string | null;
};
export type MemoryReport = {
  total_experiments: number; distinct_instances: number;
  buckets: MemoryBucket[]; note: string;
};
export type Recollection = {
  instance: string; radius: number;
  signature: { n: number; density: number; clustering: number; mean_degree: number; degree_cv: number };
  similar_instances: string[];
  best_operators: { operator: string; mean_improvement: number }[];
  applicable_facts: string[];
  narrative: string;
  computed_from: string;
};

export const getMemoryReport = (base: string) =>
  req<MemoryReport>(base, "/api/memory/report", undefined, 20_000);

export const getMemoryRecall = (base: string, instance: string, radius: number) =>
  req<Recollection>(base, `/api/memory/recall?instance=${encodeURIComponent(instance)}&radius=${radius}`, undefined, 30_000);
