/**
 * Ollama client — browser-direct.
 *
 * Verified on this machine: Ollama 0.31.2 serves CORS headers for
 * `http://localhost:3000` and `:3100`, so the browser talks to it with no proxy
 * and no Rust involvement. Every number this module reports is measured from the
 * API response — nothing is estimated.
 *
 * Endpoints used (all verified live):
 *   GET  /api/version           → server version
 *   GET  /api/tags              → installed models + metadata
 *   GET  /api/ps                → LOADED models + size_vram + expires_at
 *   POST /api/show              → per-model detail (details, capabilities)
 *   POST /api/pull   (NDJSON)   → download with progress
 *   DELETE /api/delete          → remove a model
 *   POST /api/generate (NDJSON) → generate / load / unload, with timings
 */

export const DEFAULT_OLLAMA_HOST = "http://localhost:11434";

export type OllamaDetails = {
  parent_model?: string;
  format?: string;
  family?: string;
  families?: string[];
  parameter_size?: string;
  quantization_level?: string;
  context_length?: number;
  embedding_length?: number;
};

export type OllamaModel = {
  name: string;
  model: string;
  size: number;
  digest: string;
  modified_at: string;
  details?: OllamaDetails;
  capabilities?: string[];
};

export type LoadedModel = {
  name: string;
  size: number;
  size_vram: number;
  expires_at?: string;
  context_length?: number;
};

/** Timing fields Ollama returns; all durations are nanoseconds. */
export type GenTimings = {
  eval_count?: number;
  eval_duration?: number;
  prompt_eval_count?: number;
  prompt_eval_duration?: number;
  load_duration?: number;
  total_duration?: number;
  done_reason?: string;
  /** Measured client-side: time to first streamed token (ms). Not from Ollama. */
  ttftMs?: number;
};

/** Derived, honest metrics — undefined when the source field is absent. */
export type DerivedMetrics = {
  tokensPerSec?: number;
  promptTokensPerSec?: number;
  loadMs?: number;
  totalMs?: number;
  evalTokens?: number;
  promptTokens?: number;
};

export function derive(t: GenTimings): DerivedMetrics {
  const ns = (v?: number) => (typeof v === "number" ? v / 1e6 : undefined);
  const rate = (c?: number, d?: number) => (c && d ? c / (d / 1e9) : undefined);
  return {
    tokensPerSec: rate(t.eval_count, t.eval_duration),
    promptTokensPerSec: rate(t.prompt_eval_count, t.prompt_eval_duration),
    loadMs: ns(t.load_duration),
    totalMs: ns(t.total_duration),
    evalTokens: t.eval_count,
    promptTokens: t.prompt_eval_count,
  };
}

export const fmtBytes = (b?: number) => {
  if (!b) return "—";
  const u = ["B", "KB", "MB", "GB", "TB"];
  let i = 0, v = b;
  while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
  return `${v.toFixed(v < 10 && i > 1 ? 1 : 0)} ${u[i]}`;
};

async function jsonFetch<T>(url: string, init?: RequestInit, timeoutMs = 8000): Promise<T> {
  const ctl = new AbortController();
  const t = setTimeout(() => ctl.abort(), timeoutMs);
  try {
    const r = await fetch(url, { ...init, signal: ctl.signal });
    if (!r.ok) throw new Error(`HTTP ${r.status} ${r.statusText}`);
    return (await r.json()) as T;
  } finally {
    clearTimeout(t);
  }
}

export class Ollama {
  constructor(public host: string = DEFAULT_OLLAMA_HOST) {}

  private u(p: string) { return `${this.host.replace(/\/$/, "")}${p}`; }

  version() { return jsonFetch<{ version: string }>(this.u("/api/version")); }

  async tags(): Promise<OllamaModel[]> {
    const d = await jsonFetch<{ models?: OllamaModel[] }>(this.u("/api/tags"));
    return d.models ?? [];
  }

  async ps(): Promise<LoadedModel[]> {
    const d = await jsonFetch<{ models?: LoadedModel[] }>(this.u("/api/ps"), undefined, 5000);
    return d.models ?? [];
  }

  show(model: string) {
    return jsonFetch<{ details?: OllamaDetails; capabilities?: string[]; model_info?: Record<string, unknown> }>(
      this.u("/api/show"),
      { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ model }) },
      15000,
    );
  }

  del(model: string) {
    return jsonFetch<unknown>(this.u("/api/delete"), {
      method: "DELETE", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ model }),
    }, 30000);
  }

  /** Load a model into memory (empty prompt) with a keep-alive TTL. */
  load(model: string, keepAlive = "10m") {
    return jsonFetch<GenTimings>(this.u("/api/generate"), {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ model, prompt: "", stream: false, keep_alive: keepAlive }),
    }, 600_000); // a cold 4.7 GB load measured 75 s here; allow generous headroom
  }

  /** Unload immediately (keep_alive: 0). */
  unload(model: string) {
    return jsonFetch<GenTimings>(this.u("/api/generate"), {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ model, prompt: "", stream: false, keep_alive: 0 }),
    }, 60_000);
  }

  /** Stream a pull, reporting NDJSON progress lines. */
  async pull(
    model: string,
    onProgress: (p: { status: string; completed?: number; total?: number; digest?: string }) => void,
    signal?: AbortSignal,
  ) {
    const r = await fetch(this.u("/api/pull"), {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ model, stream: true }), signal,
    });
    if (!r.ok || !r.body) throw new Error(`HTTP ${r.status}`);
    await readNdjson(r.body, (o) => onProgress(o as { status: string; completed?: number; total?: number }));
  }

  /**
   * Stream a generation. Calls `onToken` per chunk and resolves with the final
   * timing object (Ollama sends timings on the `done` line).
   */
  async generateStream(
    req: { model: string; prompt: string; system?: string; options?: Record<string, unknown>; keepAlive?: string },
    onToken: (chunk: string) => void,
    signal?: AbortSignal,
  ): Promise<GenTimings> {
    const started = performance.now();
    let firstTokenAt: number | undefined;
    let timings: GenTimings = {};
    const r = await fetch(this.u("/api/generate"), {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: req.model, prompt: req.prompt, system: req.system,
        options: req.options, keep_alive: req.keepAlive ?? "10m", stream: true,
      }),
      signal,
    });
    if (!r.ok || !r.body) throw new Error(`HTTP ${r.status} ${r.statusText}`);
    await readNdjson(r.body, (o) => {
      const line = o as { response?: string; done?: boolean } & GenTimings;
      if (line.response) {
        if (firstTokenAt === undefined) firstTokenAt = performance.now();
        onToken(line.response);
      }
      if (line.done) timings = line;
    });
    if (firstTokenAt !== undefined) timings.ttftMs = firstTokenAt - started;
    return timings;
  }
}

/** Read an NDJSON stream, invoking `onLine` per parsed object. */
async function readNdjson(body: ReadableStream<Uint8Array>, onLine: (o: unknown) => void) {
  const reader = body.getReader();
  const dec = new TextDecoder();
  let buf = "";
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    buf += dec.decode(value, { stream: true });
    let nl: number;
    while ((nl = buf.indexOf("\n")) >= 0) {
      const line = buf.slice(0, nl).trim();
      buf = buf.slice(nl + 1);
      if (!line) continue;
      try { onLine(JSON.parse(line)); } catch { /* partial/non-JSON line */ }
    }
  }
  const tail = buf.trim();
  if (tail) { try { onLine(JSON.parse(tail)); } catch { /* ignore */ } }
}
