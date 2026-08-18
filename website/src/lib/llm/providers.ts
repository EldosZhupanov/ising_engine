/**
 * Provider registry (AD-8). Two protocol adapters + Ollama native cover every
 * provider in the brief. API keys are held in browser memory/localStorage ONLY —
 * never sent anywhere but the provider, never persisted server-side, never logged.
 */

export type Protocol = "ollama" | "openai-compatible" | "anthropic";

export type Provider = {
  id: string;
  label: string;
  protocol: Protocol;
  /** Default base URL; local providers are reachable without a key. */
  baseUrl: string;
  local: boolean;
  needsKey: boolean;
  /** What the engine itself uses this tier for, if anything (from the Rust side). */
  engineRole?: string;
  docs?: string;
};

export const PROVIDERS: Provider[] = [
  {
    id: "ollama", label: "Ollama", protocol: "ollama", baseUrl: "http://localhost:11434",
    local: true, needsKey: false,
    engineRole: "llm.rs — the local Ideator tier (POST /api/generate)",
  },
  {
    id: "lmstudio", label: "LM Studio", protocol: "openai-compatible", baseUrl: "http://localhost:1234/v1",
    local: true, needsKey: false,
  },
  {
    id: "vllm", label: "vLLM", protocol: "openai-compatible", baseUrl: "http://localhost:8000/v1",
    local: true, needsKey: false,
  },
  {
    id: "anthropic", label: "Anthropic", protocol: "anthropic", baseUrl: "https://api.anthropic.com/v1",
    local: false, needsKey: true,
    engineRole: "cloud.rs — the deep-analysis tier (Messages API, ANTHROPIC_API_KEY)",
  },
  { id: "openai", label: "OpenAI", protocol: "openai-compatible", baseUrl: "https://api.openai.com/v1", local: false, needsKey: true },
  { id: "openrouter", label: "OpenRouter", protocol: "openai-compatible", baseUrl: "https://openrouter.ai/api/v1", local: false, needsKey: true },
  { id: "deepseek", label: "DeepSeek", protocol: "openai-compatible", baseUrl: "https://api.deepseek.com/v1", local: false, needsKey: true },
  { id: "gemini", label: "Gemini", protocol: "openai-compatible", baseUrl: "https://generativelanguage.googleapis.com/v1beta/openai", local: false, needsKey: true },
];

export const byId = (id: string) => PROVIDERS.find((p) => p.id === id);

const KEY_PREFIX = "ising.llm.key.";
const HOST_PREFIX = "ising.llm.host.";

export const loadKey = (id: string): string => {
  if (typeof window === "undefined") return "";
  return window.localStorage.getItem(KEY_PREFIX + id) ?? "";
};
export const saveKey = (id: string, key: string) => {
  if (typeof window === "undefined") return;
  if (key) window.localStorage.setItem(KEY_PREFIX + id, key);
  else window.localStorage.removeItem(KEY_PREFIX + id);
};
export const loadHost = (id: string, fallback: string): string => {
  if (typeof window === "undefined") return fallback;
  return window.localStorage.getItem(HOST_PREFIX + id) || fallback;
};
export const saveHost = (id: string, host: string) => {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(HOST_PREFIX + id, host);
};

/** List models for a non-Ollama provider (OpenAI-compatible `/models`). */
export async function listOpenAiModels(baseUrl: string, key: string): Promise<string[]> {
  const r = await fetch(`${baseUrl.replace(/\/$/, "")}/models`, {
    headers: key ? { Authorization: `Bearer ${key}` } : {},
  });
  if (!r.ok) throw new Error(`HTTP ${r.status}`);
  const d = (await r.json()) as { data?: { id: string }[] };
  return (d.data ?? []).map((m) => m.id).sort();
}
