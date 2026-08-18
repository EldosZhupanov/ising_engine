#!/usr/bin/env node
// Ising Engine website — headless driver / smoke harness.
//
// Launches Playwright's bundled headless Chromium against a running Next.js
// server, screenshots the full page, and asserts that the real section content
// rendered. It does NOT start the server — start it yourself first (see SKILL.md)
// so the driver stays a pure, re-runnable probe.
//
//   node .claude/skills/run-ising-website/driver.mjs [baseURL]
//
// Env:
//   BASE_URL   override target (default http://localhost:3100)
//   SHOT_DIR   screenshot output dir (default <this-dir>/shots)
//
// Exit code 0 = page served and every expected marker was found; 1 = failure.

import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { mkdirSync, existsSync } from "node:fs";

const here = dirname(fileURLToPath(import.meta.url));

// If the no-sudo fallback libs are present (see setup-libs.sh), make Chromium
// find them without the caller having to export LD_LIBRARY_PATH. Must happen
// before Playwright spawns the browser child, which inherits this env.
const bundledLibs = join(here, ".chromium-libs/root/usr/lib/x86_64-linux-gnu");
if (existsSync(bundledLibs)) {
  process.env.LD_LIBRARY_PATH = process.env.LD_LIBRARY_PATH
    ? `${bundledLibs}:${process.env.LD_LIBRARY_PATH}`
    : bundledLibs;
}

const { chromium } = await import("playwright");
const baseURL = process.argv[2] || process.env.BASE_URL || "http://localhost:3100";
const shotDir = process.env.SHOT_DIR || join(here, "shots");
mkdirSync(shotDir, { recursive: true });

// Real content markers on Mission Control (/) — the OS shell + real telemetry.
const MARKERS = [
  "Ising Engine",
  "Mission Control",
  "autonomous scientist",
  "Experiments recorded",
  "Research loop",
  "Confirmed",
  "Knowledge Graph",
  "Latest discoveries",
];

const fail = (msg) => {
  console.error(`FAIL: ${msg}`);
  process.exit(1);
};

const browser = await chromium.launch(); // headless by default
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const resp = await page.goto(baseURL, { waitUntil: "networkidle", timeout: 30000 })
    .catch((e) => fail(`could not reach ${baseURL} — is the server up? (${e.message})`));
  if (!resp || !resp.ok()) fail(`HTTP ${resp ? resp.status() : "no response"} from ${baseURL}`);

  const title = await page.title();
  console.log(`title: ${title}`);

  // Normalize non-breaking spaces (&nbsp;) and runs of whitespace so markers
  // match the visible text regardless of the exact markup that produced it.
  const body = (await page.textContent("body")).replace(/\s+/g, " ");
  const missing = MARKERS.filter((m) => !body.includes(m));
  if (missing.length) fail(`missing rendered markers: ${missing.join(", ")}`);
  console.log(`markers: ${MARKERS.length}/${MARKERS.length} present`);

  // The page reveals content on scroll (motion/react whileInView + useInView
  // count-ups). A full-page screenshot captures from the top, so below-fold
  // sections would render blank/zero unless we first scroll them into view to
  // trip their `once: true` reveals. Step down the page, then back to the top.
  await page.evaluate(async () => {
    const step = window.innerHeight * 0.8;
    for (let y = 0; y < document.body.scrollHeight; y += step) {
      window.scrollTo(0, y);
      await new Promise((r) => setTimeout(r, 250));
    }
    window.scrollTo(0, 0);
  });
  await page.waitForTimeout(800); // let count-up / bar-fill animations settle

  const shot = join(shotDir, "home.png");
  await page.screenshot({ path: shot, fullPage: true });
  console.log(`screenshot: ${shot}`);

  // Capture the real workspaces by navigating routes (each is deep-linkable).
  for (const route of ["start", "discover", "design", "publish", "applications", "playbooks", "experiments", "graph", "benchmarks", "evaluation", "dataset", "papers", "llm", "operators", "models", "runtime", "features", "docs", "settings", "solve", "campaigns"]) {
    try {
      await page.goto(`${baseURL}/${route}`, { waitUntil: "networkidle", timeout: 20000 });
      await page.evaluate(async () => { const s = window.innerHeight * 0.8; for (let y = 0; y < document.body.scrollHeight; y += s) { window.scrollTo(0, y); await new Promise((r) => setTimeout(r, 180)); } window.scrollTo(0, 0); });
      await page.waitForTimeout(500);
      const f = join(shotDir, route + ".png");
      await page.screenshot({ path: f, fullPage: true });
      console.log(`screenshot: ${f}`);
    } catch (e) {
      console.log(`note: could not capture "${route}" (${e.message.split("\n")[0]})`);
    }
  }

  console.log("PASS");
} finally {
  await browser.close();
}
