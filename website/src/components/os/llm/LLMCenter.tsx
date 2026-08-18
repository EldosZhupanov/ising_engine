"use client";

/**
 * LLM Control Center — the laboratory's AI subsystem.
 *
 * This is REAL and LIVE: the browser talks directly to Ollama's HTTP API
 * (CORS-verified on this machine). Model install/pull/load/unload/delete,
 * VRAM, context, tokens/sec, TTFT and streaming reasoning are all measured from
 * the API — nothing here is simulated.
 *
 * The Reasoning Console does not host a chatbot. It sends the *engine's own*
 * prompt shape — the knowledge-graph digest that `llm.rs` feeds its Ideator —
 * so you are watching the platform's scientist reason over real evidence.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { motion } from "motion/react";
import {
  Ollama, derive, fmtBytes,
  type OllamaModel, type LoadedModel, type DerivedMetrics,
} from "@/lib/llm/ollama";
import { PROVIDERS, byId, loadKey, saveKey, loadHost, saveHost, listOpenAiModels } from "@/lib/llm/providers";
import { Panel, PanelHeader, Pill, StatusBadge, EmptyState } from "@/components/os/kit";

type Fact = { subject: string; relation: string; object: string; weight: number; support: number; confidence: number | null };

type Bench = { model: string; tokPerSec?: number; ttftMs?: number; loadMs?: number; promptRate?: number; at: number };

const PROMPT_PRESETS = [
  { id: "ideate", label: "Ideate operator sequences", build: (d: string) => `You are the scientist inside an autonomous algorithm-discovery platform (Ising Engine). It searches for sequences of physics OPERATORS that minimise the energy of Ising/QUBO problems.\n\nHere is the platform's current knowledge graph (subject --relation--> object, with evidence weight and support count):\n\n${d}\n\nPropose 5 operator sequences worth testing next. For EACH: give the sequence, then a one-sentence CAUSAL argument grounded in the facts above. Be concise. Do not invent facts that are not in the graph.` },
  { id: "critique", label: "Critique the evidence", build: (d: string) => `Below is the knowledge graph of an autonomous optimisation-research platform:\n\n${d}\n\nAct as a skeptical reviewer. Identify the WEAKEST claims — low support, likely confounds, or conclusions that do not follow. Say explicitly what further experiment would falsify each. Be terse and rigorous.` },
  { id: "explain", label: "Explain to a newcomer", build: (d: string) => `Given these mined facts from an Ising/QUBO optimisation research platform:\n\n${d}\n\nExplain in plain language what this system appears to have learned about which operators work and when. Flag anything that looks like an artifact rather than a real effect.` },
];

/** Format facts exactly like llm.rs does for its Ideator prompt. */
function digest(facts: Fact[], limit = 40): string {
  return facts
    .slice()
    .sort((a, b) => b.support - a.support)
    .slice(0, limit)
    .map((f) => `  ${f.subject} --${f.relation}--> ${f.object || "?"} (w=${f.weight >= 0 ? "+" : ""}${f.weight.toFixed(1)}, support=${f.support})`)
    .join("\n");
}

export default function LLMCenter({ facts }: { facts: Fact[] }) {
  const [providerId, setProviderId] = useState("ollama");
  const provider = byId(providerId)!;

  const [host, setHost] = useState(provider.baseUrl);
  const [apiKey, setApiKey] = useState("");
  const [conn, setConn] = useState<"checking" | "online" | "offline">("checking");
  const [version, setVersion] = useState<string | null>(null);
  const [models, setModels] = useState<OllamaModel[]>([]);
  const [loaded, setLoaded] = useState<LoadedModel[]>([]);
  const [remoteModels, setRemoteModels] = useState<string[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);

  // pull
  const [pullName, setPullName] = useState("");
  const [pull, setPull] = useState<{ status: string; completed?: number; total?: number } | null>(null);

  // console
  const [selected, setSelected] = useState<string>("");
  const [preset, setPreset] = useState(PROMPT_PRESETS[0].id);
  const [temp, setTemp] = useState(0.7);
  const [output, setOutput] = useState("");
  const [streaming, setStreaming] = useState(false);
  const [metrics, setMetrics] = useState<DerivedMetrics & { ttftMs?: number } | null>(null);
  const abortRef = useRef<AbortController | null>(null);
  const outRef = useRef<HTMLPreElement>(null);

  const [bench, setBench] = useState<Bench[]>([]);

  const client = useCallback(() => new Ollama(host), [host]);

  // hydrate persisted host/key when the provider changes (event-driven, not in an effect body)
  const pickProvider = (id: string) => {
    const p = byId(id)!;
    setProviderId(id);
    const h = loadHost(id, p.baseUrl);
    setHost(h);
    setApiKey(loadKey(id));
    setConn("checking");
    setModels([]); setLoaded([]); setRemoteModels([]); setVersion(null); setErr(null);
  };

  const refresh = useCallback(async () => {
    if (providerId !== "ollama") {
      // OpenAI-compatible / Anthropic: list models if a key is present
      if (provider.needsKey && !apiKey) { setConn("offline"); setRemoteModels([]); return; }
      try {
        const list = provider.protocol === "openai-compatible" ? await listOpenAiModels(host, apiKey) : [];
        setRemoteModels(list); setConn("online"); setErr(null);
      } catch (e) { setConn("offline"); setErr(e instanceof Error ? e.message : String(e)); }
      return;
    }
    const o = client();
    try {
      const [v, t, p] = await Promise.all([o.version(), o.tags(), o.ps()]);
      setVersion(v.version); setModels(t); setLoaded(p); setConn("online"); setErr(null);
      setSelected((s) => s || t[0]?.name || "");
    } catch (e) {
      setConn("offline"); setErr(e instanceof Error ? e.message : String(e));
    }
  }, [client, providerId, provider, apiKey, host]);

  // initial detect + poll /api/ps (setState happens in async callbacks, never synchronously)
  useEffect(() => {
    let alive = true;
    const tick = async () => { if (alive) await refresh(); };
    void tick();
    const id = setInterval(() => { if (providerId === "ollama") void tick(); }, 4000);
    return () => { alive = false; clearInterval(id); };
  }, [refresh, providerId]);

  const vramTotal = loaded.reduce((a, m) => a + (m.size_vram || 0), 0);

  const act = async (label: string, fn: () => Promise<unknown>) => {
    setBusy(label); setErr(null);
    try { await fn(); await refresh(); }
    catch (e) { setErr(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(null); }
  };

  const doPull = async () => {
    const name = pullName.trim();
    if (!name) return;
    setBusy(`pull ${name}`); setErr(null); setPull({ status: "starting" });
    try {
      await client().pull(name, (p) => setPull(p));
      setPullName("");
      await refresh();
    } catch (e) { setErr(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(null); setTimeout(() => setPull(null), 1500); }
  };

  const run = async () => {
    if (!selected) return;
    const p = PROMPT_PRESETS.find((x) => x.id === preset)!;
    const prompt = p.build(digest(facts));
    setOutput(""); setMetrics(null); setStreaming(true); setErr(null);
    const ctl = new AbortController();
    abortRef.current = ctl;
    try {
      const t = await client().generateStream(
        { model: selected, prompt, options: { temperature: temp } },
        (chunk) => {
          setOutput((o) => o + chunk);
          if (outRef.current) outRef.current.scrollTop = outRef.current.scrollHeight;
        },
        ctl.signal,
      );
      setMetrics({ ...derive(t), ttftMs: t.ttftMs });
    } catch (e) {
      if (!(e instanceof DOMException && e.name === "AbortError")) setErr(e instanceof Error ? e.message : String(e));
    } finally { setStreaming(false); abortRef.current = null; void refresh(); }
  };

  const benchmark = async (model: string) => {
    setBusy(`benchmark ${model}`); setErr(null);
    try {

      const t = await client().generateStream(
        { model, prompt: "Count from 1 to 40, comma separated. Nothing else.", options: { temperature: 0, num_predict: 96 } },
        () => {},
      );
      const d = derive(t);
      setBench((b) => [{ model, tokPerSec: d.tokensPerSec, ttftMs: t.ttftMs, loadMs: d.loadMs, promptRate: d.promptTokensPerSec, at: Date.now() }, ...b.filter((x) => x.model !== model)].slice(0, 8));
      await refresh();
    } catch (e) { setErr(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(null); }
  };

  const isLoaded = (name: string) => loaded.some((l) => l.name === name);

  return (
    <div className="mx-auto max-w-[1240px] px-5 py-8 md:px-8">
      {/* header */}
      <div className="mb-6 flex flex-wrap items-end justify-between gap-3">
        <div>
          <div className="mono mb-3 flex items-center gap-3 text-[11px] uppercase tracking-[.2em] text-[var(--os-muted)]">
            <span className="text-[var(--os-accent)]">AI</span><span className="h-px w-7 bg-[var(--os-hair-2)]" /><span>LLM Control Center</span>
          </div>
          <h1 className="text-[clamp(1.6rem,2.6vw,2.3rem)] font-semibold tracking-[-.025em]">The laboratory&rsquo;s reasoning subsystem.</h1>
          <p className="mt-2 max-w-[74ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
            Live control of local and remote models. Every metric below is measured from the provider&rsquo;s API — VRAM, context, tokens/sec and time-to-first-token are read or derived, never estimated.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <StatusBadge status={conn === "online" ? "real" : conn === "checking" ? "planned" : "offline"} />
          {version && <span className="mono rounded-md border border-[var(--os-hair)] bg-[var(--os-ground-2)] px-2 py-1 text-[10.5px] text-[var(--os-faint)]">ollama {version}</span>}
        </div>
      </div>

      {/* providers */}
      <div className="mb-4 flex flex-wrap items-center gap-1.5">
        {PROVIDERS.map((p) => (
          <button key={p.id} onClick={() => pickProvider(p.id)}
            className={`mono rounded-full border px-3 py-1.5 text-[11.5px] transition-colors ${providerId === p.id ? "border-[var(--os-accent)] bg-[var(--os-accent-soft)] text-[var(--os-ink)]" : "border-[var(--os-hair-2)] text-[var(--os-muted)] hover:text-[var(--os-ink)]"}`}>
            {p.label}{p.local && <span className="ml-1.5 text-[var(--os-confirmed)]">local</span>}
          </button>
        ))}
      </div>

      {/* connection / config */}
      <Panel className="mb-4">
        <PanelHeader title={`${provider.label} — connection`} status={conn === "online" ? "real" : "offline"}
          sub={provider.engineRole ?? `${provider.protocol} protocol`} />
        <div className="grid gap-4 p-5 md:grid-cols-[1.4fr_1fr]">
          <div className="space-y-3">
            <label className="block">
              <span className="mono text-[10.5px] uppercase tracking-[.1em] text-[var(--os-faint)]">Host / base URL</span>
              <input value={host} onChange={(e) => { setHost(e.target.value); saveHost(providerId, e.target.value); }}
                className="mono mt-1 w-full rounded-[8px] border border-[var(--os-hair-2)] bg-[var(--os-ground)] px-3 py-2 text-[12.5px] text-[var(--os-ink)] focus:border-[var(--os-accent)] focus:outline-none" />
            </label>
            {provider.needsKey && (
              <label className="block">
                <span className="mono text-[10.5px] uppercase tracking-[.1em] text-[var(--os-faint)]">API key — stored in this browser only</span>
                <input type="password" value={apiKey} placeholder="sk-…"
                  onChange={(e) => { setApiKey(e.target.value); saveKey(providerId, e.target.value); }}
                  className="mono mt-1 w-full rounded-[8px] border border-[var(--os-hair-2)] bg-[var(--os-ground)] px-3 py-2 text-[12.5px] text-[var(--os-ink)] focus:border-[var(--os-accent)] focus:outline-none" />
              </label>
            )}
            <div className="flex items-center gap-2">
              <button onClick={() => void refresh()} className="mono rounded-[8px] border border-[var(--os-hair-2)] px-3 py-1.5 text-[11.5px] text-[var(--os-muted)] hover:text-[var(--os-ink)]">re-detect</button>
              {err && <span className="mono truncate text-[11px] text-[var(--os-refuted)]">{err}</span>}
            </div>
          </div>

          {/* live telemetry */}
          <div className="os-raised rounded-[12px] p-4">
            <div className="mono mb-3 text-[10.5px] uppercase tracking-[.12em] text-[var(--os-muted)]">Live telemetry</div>
            <div className="space-y-2.5">
              <Metric label="Models resident" value={String(loaded.length)} />
              <Metric label="VRAM held by models" value={fmtBytes(vramTotal)} accent />
              <Metric label="Installed" value={providerId === "ollama" ? String(models.length) : String(remoteModels.length)} />
              {loaded[0]?.context_length !== undefined && <Metric label="Active context" value={`${loaded[0].context_length!.toLocaleString()} tok`} />}
            </div>
            <p className="mono mt-3 border-t border-[var(--os-hair)] pt-2.5 text-[10px] leading-[1.5] text-[var(--os-faint)]">
              GPU utilisation, total VRAM, CPU and RAM require the local bridge (not yet built) — the browser cannot read <span className="text-[var(--os-muted)]">nvidia-smi</span>. VRAM above is what the provider reports.
            </p>
          </div>
        </div>
      </Panel>

      {providerId === "ollama" ? (
        <>
          {/* models + pull */}
          <div className="grid gap-4 lg:grid-cols-[1.45fr_1fr]">
            <Panel>
              <PanelHeader title="Installed models" status="real" sub="/api/tags · /api/ps"
                right={<span className="mono text-[10.5px] text-[var(--os-faint)]">{models.length} installed · {loaded.length} resident</span>} />
              {models.length === 0 ? <EmptyState title={conn === "online" ? "No models installed" : "Ollama not reachable"} hint={conn === "online" ? "pull one below" : `checked ${host}`} /> : (
                <div className="divide-y divide-[var(--os-hair)]">
                  {models.map((m) => {
                    const on = isLoaded(m.name);
                    const lm = loaded.find((l) => l.name === m.name);
                    const d = m.details ?? {};
                    return (
                      <div key={m.name} className="px-5 py-3.5">
                        <div className="flex flex-wrap items-center gap-2">
                          <button onClick={() => setSelected(m.name)}
                            className={`mono text-[13px] ${selected === m.name ? "text-[var(--os-accent)]" : "text-[var(--os-ink)] hover:text-[var(--os-accent)]"}`}>
                            {m.name}
                          </button>
                          {on && <Pill tone="confirmed">resident · {fmtBytes(lm?.size_vram)}</Pill>}
                          <span className="mono ml-auto text-[11px] text-[var(--os-faint)]">{fmtBytes(m.size)}</span>
                        </div>
                        <div className="mono mt-1.5 flex flex-wrap gap-x-3 gap-y-1 text-[10.5px] text-[var(--os-faint)]">
                          {d.parameter_size && <span>{d.parameter_size}</span>}
                          {d.quantization_level && <span>{d.quantization_level}</span>}
                          {d.family && <span>{d.family}</span>}
                          {d.context_length && <span>ctx {d.context_length.toLocaleString()}</span>}
                          {m.capabilities?.length ? <span>{m.capabilities.join(" · ")}</span> : null}
                        </div>
                        <div className="mt-2.5 flex flex-wrap gap-1.5">
                          {on
                            ? <Act label="Unload" busy={busy === `unload ${m.name}`} onClick={() => act(`unload ${m.name}`, () => client().unload(m.name))} />
                            : <Act label="Load" busy={busy === `load ${m.name}`} onClick={() => act(`load ${m.name}`, () => client().load(m.name))} primary />}
                          <Act label="Benchmark" busy={busy === `benchmark ${m.name}`} onClick={() => benchmark(m.name)} />
                          <Act label="Delete" danger busy={busy === `delete ${m.name}`} onClick={() => act(`delete ${m.name}`, () => client().del(m.name))} />
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </Panel>

            <div className="space-y-4">
              <Panel>
                <PanelHeader title="Install a model" status="real" sub="/api/pull — streamed progress" />
                <div className="p-5">
                  <div className="flex gap-2">
                    <input value={pullName} onChange={(e) => setPullName(e.target.value)} placeholder="e.g. qwen2.5-coder:7b"
                      onKeyDown={(e) => { if (e.key === "Enter") void doPull(); }}
                      className="mono flex-1 rounded-[8px] border border-[var(--os-hair-2)] bg-[var(--os-ground)] px-3 py-2 text-[12.5px] text-[var(--os-ink)] focus:border-[var(--os-accent)] focus:outline-none" />
                    <button onClick={() => void doPull()} disabled={!pullName.trim() || !!busy}
                      className="mono rounded-[8px] bg-[var(--os-accent)] px-3.5 py-2 text-[12px] font-medium text-black disabled:opacity-40">pull</button>
                  </div>
                  {pull && (
                    <div className="mt-3">
                      <div className="mono mb-1 flex justify-between text-[10.5px] text-[var(--os-muted)]">
                        <span className="truncate">{pull.status}</span>
                        {pull.total ? <span>{fmtBytes(pull.completed)} / {fmtBytes(pull.total)}</span> : null}
                      </div>
                      <div className="h-1.5 overflow-hidden rounded-full bg-[var(--os-raised)]">
                        <motion.div className="h-full rounded-full bg-[var(--os-accent)]"
                          animate={{ width: pull.total ? `${Math.round(((pull.completed ?? 0) / pull.total) * 100)}%` : "12%" }} transition={{ duration: 0.3 }} />
                      </div>
                    </div>
                  )}
                  <p className="mono mt-3 text-[10px] leading-[1.5] text-[var(--os-faint)]">
                    Downloads run on the Ollama server, streamed here layer by layer.
                  </p>
                </div>
              </Panel>

              <Panel>
                <PanelHeader title="Benchmark" status="real" sub="measured: tok/s · TTFT · load" />
                {bench.length === 0 ? <div className="px-5 py-6 text-[12.5px] text-[var(--os-faint)]">Run a benchmark on any model to compare.</div> : (
                  <div className="overflow-x-auto">
                    <table className="w-full text-left">
                      <thead><tr className="mono border-b border-[var(--os-hair)] text-[10px] uppercase tracking-[.08em] text-[var(--os-faint)]">
                        <th className="px-4 py-2 font-normal">model</th><th className="px-4 py-2 font-normal">tok/s</th><th className="px-4 py-2 font-normal">TTFT</th><th className="px-4 py-2 font-normal">load</th></tr></thead>
                      <tbody>
                        {bench.map((b) => (
                          <tr key={b.model} className="mono border-b border-[var(--os-hair)] text-[11.5px] last:border-0">
                            <td className="px-4 py-2 text-[var(--os-ink)]">{b.model}</td>
                            <td className="px-4 py-2 tnum text-[var(--os-accent)]">{b.tokPerSec?.toFixed(1) ?? "—"}</td>
                            <td className="px-4 py-2 tnum text-[var(--os-muted)]">{b.ttftMs ? `${Math.round(b.ttftMs)} ms` : "—"}</td>
                            <td className="px-4 py-2 tnum text-[var(--os-muted)]">{b.loadMs ? `${(b.loadMs / 1000).toFixed(1)} s` : "—"}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                )}
              </Panel>
            </div>
          </div>

          {/* reasoning console */}
          <Panel className="mt-4">
            <PanelHeader title="Reasoning console" status="real"
              sub="the engine's own Ideator prompt — real knowledge-graph digest, streamed"
              right={<span className="mono text-[10.5px] text-[var(--os-faint)]">{facts.length} facts in prompt</span>} />
            <div className="grid gap-4 p-5 lg:grid-cols-[300px_1fr]">
              <div className="space-y-3">
                <label className="block">
                  <span className="mono text-[10.5px] uppercase tracking-[.1em] text-[var(--os-faint)]">Model</span>
                  <select value={selected} onChange={(e) => setSelected(e.target.value)}
                    className="mono mt-1 w-full rounded-[8px] border border-[var(--os-hair-2)] bg-[var(--os-ground)] px-2.5 py-2 text-[12px] text-[var(--os-ink)] focus:border-[var(--os-accent)] focus:outline-none">
                    {models.map((m) => <option key={m.name} value={m.name}>{m.name}</option>)}
                  </select>
                </label>
                <label className="block">
                  <span className="mono text-[10.5px] uppercase tracking-[.1em] text-[var(--os-faint)]">Task</span>
                  <select value={preset} onChange={(e) => setPreset(e.target.value)}
                    className="mono mt-1 w-full rounded-[8px] border border-[var(--os-hair-2)] bg-[var(--os-ground)] px-2.5 py-2 text-[12px] text-[var(--os-ink)] focus:border-[var(--os-accent)] focus:outline-none">
                    {PROMPT_PRESETS.map((p) => <option key={p.id} value={p.id}>{p.label}</option>)}
                  </select>
                </label>
                <label className="block">
                  <span className="mono flex justify-between text-[10.5px] uppercase tracking-[.1em] text-[var(--os-faint)]"><span>Temperature</span><span className="text-[var(--os-muted)]">{temp.toFixed(2)}</span></span>
                  <input type="range" min={0} max={1.5} step={0.05} value={temp} onChange={(e) => setTemp(+e.target.value)} className="mt-2 w-full accent-[var(--os-accent)]" />
                </label>
                <div className="flex gap-2">
                  {streaming
                    ? <button onClick={() => abortRef.current?.abort()} className="mono flex-1 rounded-[8px] border border-[var(--os-refuted)] px-3 py-2 text-[12px] text-[var(--os-refuted)]">stop</button>
                    : <button onClick={() => void run()} disabled={!selected || conn !== "online"}
                        className="mono flex-1 rounded-[8px] bg-[var(--os-accent)] px-3 py-2 text-[12px] font-medium text-black disabled:opacity-40">reason</button>}
                </div>
                {metrics && (
                  <div className="os-raised rounded-[10px] p-3">
                    <div className="mono mb-2 text-[10px] uppercase tracking-[.12em] text-[var(--os-muted)]">measured</div>
                    <div className="space-y-1.5">
                      <Metric label="tokens/sec" value={metrics.tokensPerSec?.toFixed(1) ?? "—"} accent />
                      <Metric label="TTFT" value={metrics.ttftMs ? `${Math.round(metrics.ttftMs)} ms` : "—"} />
                      <Metric label="output tokens" value={metrics.evalTokens?.toString() ?? "—"} />
                      <Metric label="prompt tokens" value={metrics.promptTokens?.toString() ?? "—"} />
                      <Metric label="load" value={metrics.loadMs ? `${(metrics.loadMs / 1000).toFixed(1)} s` : "—"} />
                    </div>
                  </div>
                )}
              </div>

              <div>
                <pre ref={outRef} className="mono h-[340px] overflow-auto whitespace-pre-wrap rounded-[10px] border border-[var(--os-hair)] bg-[var(--os-ground)] p-4 text-[12px] leading-[1.6] text-[var(--os-ink)]">
{output || (streaming ? "…" : "The prompt is built from the platform's real knowledge graph — the same digest llm.rs feeds its Ideator. Press “reason” to watch the model think over actual evidence.")}
                  {streaming && <span className="ml-0.5 inline-block h-3.5 w-[7px] animate-pulse bg-[var(--os-accent)] align-middle" />}
                </pre>
                <p className="mono mt-2 text-[10px] text-[var(--os-faint)]">
                  Output is the model&rsquo;s, not the platform&rsquo;s: proposals here are <span className="text-[var(--os-muted)]">unverified</span> until the engine runs and records them. Nothing written here enters the knowledge graph.
                </p>
              </div>
            </div>
          </Panel>
        </>
      ) : (
        <Panel className="p-6">
          <div className="mb-3"><StatusBadge status={conn === "online" ? "real" : "offline"} /></div>
          {remoteModels.length > 0 ? (
            <>
              <div className="mono mb-2 text-[11px] uppercase tracking-[.12em] text-[var(--os-muted)]">{remoteModels.length} models available</div>
              <div className="flex flex-wrap gap-1.5">
                {remoteModels.slice(0, 60).map((m) => <span key={m} className="mono rounded-md border border-[var(--os-hair-2)] px-2 py-1 text-[11px] text-[var(--os-muted)]">{m}</span>)}
              </div>
            </>
          ) : (
            <p className="max-w-[62ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
              {provider.needsKey && !apiKey
                ? `Enter an API key above to list ${provider.label} models. The key stays in this browser and is sent only to ${provider.label}.`
                : `No models listed. Check that ${provider.label} is running at ${host}.`}
              {provider.engineRole && <span className="mono mt-3 block text-[11.5px] text-[var(--os-faint)]">Engine tier: {provider.engineRole}</span>}
            </p>
          )}
        </Panel>
      )}
    </div>
  );
}

function Metric({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div className="flex items-baseline justify-between gap-3">
      <span className="mono text-[10.5px] text-[var(--os-faint)]">{label}</span>
      <span className={`mono text-[13px] tnum ${accent ? "text-[var(--os-accent)]" : "text-[var(--os-ink)]"}`}>{value}</span>
    </div>
  );
}

function Act({ label, onClick, busy, primary, danger }: { label: string; onClick: () => void; busy?: boolean; primary?: boolean; danger?: boolean }) {
  return (
    <button onClick={onClick} disabled={busy}
      className={`mono rounded-[7px] border px-2.5 py-1 text-[11px] transition-colors disabled:opacity-50 ${
        primary ? "border-[var(--os-accent)] text-[var(--os-accent)] hover:bg-[var(--os-accent-soft)]"
        : danger ? "border-[var(--os-hair-2)] text-[var(--os-faint)] hover:border-[var(--os-refuted)] hover:text-[var(--os-refuted)]"
        : "border-[var(--os-hair-2)] text-[var(--os-muted)] hover:text-[var(--os-ink)]"}`}>
      {busy ? "…" : label}
    </button>
  );
}
