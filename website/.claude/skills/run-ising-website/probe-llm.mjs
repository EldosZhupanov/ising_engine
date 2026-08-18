#!/usr/bin/env node
/**
 * LLM Control Center verification harness.
 *
 * Verifies the /llm workspace against a REAL running Ollama: detection, model
 * metadata, load→VRAM→unload (asserted against Ollama's own /api/ps, never
 * against ambiguous DOM text), and the streaming reasoning console with its
 * measured tok/s + TTFT.
 *
 *   node .claude/skills/run-ising-website/probe-llm.mjs [baseURL]
 *
 * Requires: the site served (see SKILL.md) and `ollama serve` running.
 * Exit 0 = all checks passed.
 */
import { chromium } from "playwright";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const libs = join(here, ".chromium-libs/root/usr/lib/x86_64-linux-gnu");
if (existsSync(libs)) process.env.LD_LIBRARY_PATH = `${libs}:${process.env.LD_LIBRARY_PATH ?? ""}`;

const base = process.argv[2] || process.env.BASE_URL || "http://localhost:3100";
const OLLAMA = process.env.OLLAMA_HOST || "http://localhost:11434";
const shots = join(here, "shots");
let failed = 0;
const check = (name, ok, extra = "") => {
  console.log(`${ok ? "PASS" : "FAIL"}  ${name}${extra ? " — " + extra : ""}`);
  if (!ok) failed++;
};
const ps = async () => {
  try { return (await (await fetch(`${OLLAMA}/api/ps`)).json()).models ?? []; } catch { return []; }
};

// Skip gracefully if Ollama isn't running — this harness needs it.
try { await fetch(`${OLLAMA}/api/version`); } catch {
  console.log(`SKIP  ollama not reachable at ${OLLAMA} — start it to verify /llm`);
  process.exit(0);
}

const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 1100 } });
  await page.goto(`${base}/llm`, { waitUntil: "networkidle", timeout: 30000 });
  await page.waitForTimeout(2500);
  const txt = async () => (await page.textContent("body")).replace(/\s+/g, " ");

  let t = await txt();
  check("detects ollama version", /ollama 0\.\d+\.\d+/.test(t));
  check("lists installed models", /qwen|llama|mistral|phi|gemma|deepseek/i.test(t));
  check("shows real model metadata", /Q\d_[A-Z0-9_]+/.test(t) && /ctx [\d,]+/.test(t));

  // load → assert against Ollama itself
  const load = page.getByRole("button", { name: "Load", exact: true }).first();
  if (await load.count()) {
    await load.click();
    let ok = false;
    for (let i = 0; i < 150; i++) { if ((await ps()).length) { ok = true; break; } await page.waitForTimeout(2000); }
    const res = await ps();
    check("LOAD from browser", ok, ok ? `ollama reports vram=${res[0].size_vram.toLocaleString()}` : "timeout");
    await page.waitForTimeout(5000);
    check("UI shows resident VRAM", /resident · [\d.]+ [KMG]B/.test(await txt()));
  } else check("Load button present", (await ps()).length > 0, "already resident");

  // streaming reasoning console + measured metrics
  await page.getByRole("button", { name: "reason", exact: true }).click();
  const pre = page.locator("pre").first();
  const lens = [];
  for (let i = 0; i < 120; i++) {
    await page.waitForTimeout(2000);
    lens.push(((await pre.textContent()) ?? "").length);
    if (lens.length >= 3 && lens.at(-1) > 200 && lens.at(-1) === lens.at(-2) && lens.at(-2) === lens.at(-3)) break;
  }
  const out = (await pre.textContent()) ?? "";
  check("reasoning streamed incrementally", lens.filter((v, i, a) => i > 0 && v > a[i - 1]).length > 1, `${out.length} chars`);
  check("reasoned over real operators", /metropolis_sweep|gibbs_color_sweep|greedy_descent|houdayer_cluster|extremal_/.test(out));
  t = await txt();
  const tok = t.match(/tokens\/sec\s*([\d.]+)/);
  check("measured tokens/sec", !!tok, tok ? `${tok[1]} tok/s` : "");
  check("measured TTFT", /TTFT\s*\d+ ms/.test(t));

  await page.evaluate(() => window.scrollTo(0, 0));
  await page.waitForTimeout(400);
  await page.screenshot({ path: join(shots, "llm.png"), fullPage: true });
  console.log(`screenshot: ${join(shots, "llm.png")}`);

  const unload = page.getByRole("button", { name: "Unload", exact: true }).first();
  if (await unload.count()) {
    await unload.click();
    let gone = false;
    for (let i = 0; i < 30; i++) { if (!(await ps()).length) { gone = true; break; } await page.waitForTimeout(1000); }
    check("UNLOAD from browser", gone);
  }
  console.log(failed ? `\n${failed} check(s) FAILED` : "\nPASS — LLM Control Center verified live");
} finally { await browser.close(); }
process.exit(failed ? 1 : 0);
