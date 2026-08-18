#!/usr/bin/env node
/**
 * Ising Engine — real-data ingestion.
 *
 * Reads the platform's on-disk artifacts from $ISING_DATA_DIR (default
 * ../experiments) and emits typed JSON bundles into src/data/generated/. The
 * browser Observatory renders ONLY from these — no fabricated numbers.
 *
 * Honesty rules:
 *  - Never ship raw 110k rows: emit aggregates + a bounded stratified sample.
 *  - Stamp provenance (source file, row count, mtime) on everything.
 *  - If the data dir is absent, write a tiny fixture so `next build` never breaks;
 *    the fixture is clearly flagged so the UI can say "bundled sample".
 *
 * DB row schema (v2, 22 pipe-delimited fields) — from src/engine_v2/ai_scientist/db.rs:
 *  0 id · 1 hypothesis_id(-1=none) · 2 sequence(csv) · 3 sweeps(csv) · 4 temp_hi
 *  5 temp_lo · 6 replicas · 7 seed · 8 backend · 9 work · 10 score · 11 baseline
 *  12 density · 13 clustering · 14 instance_id · 15 n · 16 mean_degree
 *  17 degree_cv · 18 wall_ms · 19 campaign_id · 20 generation_id · 21 timestamp
 */

import { readFileSync, writeFileSync, mkdirSync, existsSync, statSync, readdirSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const WEB = resolve(HERE, "..");
const DATA_DIR = process.env.ISING_DATA_DIR || resolve(WEB, "..", "experiments");
const OUT = join(WEB, "src", "data", "generated");
const SAMPLE_CAP = 4000; // stratified rows kept for the client table

const PRIMARY = "platform_gset";
const CAMPAIGNS = ["platform_gset", "platform_grow", "platform_grow_div"];

mkdirSync(OUT, { recursive: true });

const log = (...a) => console.log("[ingest]", ...a);
const provenance = [];
const relPath = (file) => file.includes("/ising_engine/") ? file.split("/ising_engine/").slice(1).join("/ising_engine/") : file.replace(WEB + "/", "");
const stamp = (label, file, rows) => {
  let mtime = null;
  try { mtime = statSync(file).mtime.toISOString(); } catch {}
  const rec = { label, file: relPath(file), rows, mtime, exists: existsSync(file) };
  provenance.push(rec);
  return rec;
};
const write = (name, obj) => {
  writeFileSync(join(OUT, name), JSON.stringify(obj));
  log(`wrote ${name} (${(JSON.stringify(obj).length / 1024).toFixed(0)} kB)`);
};

// ---------- parsers ----------
function parseDbLine(line) {
  const f = line.split("|");
  if (f.length < 22) return null;
  return {
    id: +f[0],
    hyp: f[1] === "-1" ? null : +f[1],
    seq: f[2] ? f[2].split(",") : [],
    sweeps: f[3] ? f[3].split(",").map(Number) : [],
    tempHi: +f[4], tempLo: +f[5], replicas: +f[6], seed: f[7],
    backend: f[8], work: +f[9], score: +f[10], baseline: +f[11],
    density: +f[12], clustering: +f[13], instance: f[14], n: +f[15],
    meanDeg: +f[16], degreeCv: +f[17], wallMs: +f[18],
    campaign: +f[19], generation: +f[20], ts: +f[21],
  };
}

function readLines(file) {
  if (!existsSync(file)) return [];
  return readFileSync(file, "utf8").split("\n").filter(Boolean);
}

// ---------- aggregate one campaign's DB in a single streaming pass ----------
function aggregateDb(dir, label) {
  const file = join(dir, "ai_experiments.txt");
  const lines = readLines(file);
  const agg = {
    label, rows: 0, instances: new Set(), backends: new Set(), operators: new Set(),
    campaigns: new Set(), generations: new Set(),
    best: null, // most-negative score (minimization)
    byOperator: new Map(), byBackend: new Map(), byInstance: new Map(), byCampaign: new Map(),
    improvements: [], // Δ = score - baseline (negative = better than baseline)
  };
  const bump = (m, k, rec) => {
    let e = m.get(k);
    if (!e) { e = { key: k, runs: 0, sumImpr: 0, best: Infinity }; m.set(k, e); }
    e.runs++; e.sumImpr += rec.score - rec.baseline; if (rec.score < e.best) e.best = rec.score;
  };
  for (const line of lines) {
    const r = parseDbLine(line);
    if (!r) continue;
    agg.rows++;
    agg.instances.add(r.instance); agg.backends.add(r.backend);
    for (const op of r.seq) agg.operators.add(op);
    agg.campaigns.add(r.campaign); agg.generations.add(`${r.campaign}:${r.generation}`);
    if (!agg.best || r.score < agg.best.score) agg.best = { score: r.score, baseline: r.baseline, instance: r.instance, seq: r.seq, seed: r.seed };
    for (const op of r.seq) bump(agg.byOperator, op, r);
    bump(agg.byBackend, r.backend, r);
    bump(agg.byInstance, r.instance, r);
    bump(agg.byCampaign, String(r.campaign), r);
    if (agg.improvements.length < 20000) agg.improvements.push(+(r.score - r.baseline).toFixed(2));
  }
  stamp(label + ":db", file, agg.rows);
  return agg;
}

function rollupToArr(m) {
  return [...m.values()].map((e) => ({
    key: e.key, runs: e.runs, meanImpr: +(e.sumImpr / e.runs).toFixed(2), best: e.best === Infinity ? null : e.best,
  })).sort((a, b) => b.runs - a.runs);
}

// stratified sample by instance×first-operator, capped
function sampleDb(dir, cap) {
  const file = join(dir, "ai_experiments.txt");
  const lines = readLines(file);
  const buckets = new Map();
  for (const line of lines) {
    const r = parseDbLine(line);
    if (!r) continue;
    const k = `${r.instance}|${r.seq[0] || "?"}`;
    if (!buckets.has(k)) buckets.set(k, []);
    buckets.get(k).push(r);
  }
  const keys = [...buckets.keys()];
  const per = Math.max(1, Math.floor(cap / Math.max(1, keys.length)));
  const out = [];
  for (const k of keys) {
    const arr = buckets.get(k);
    const step = Math.max(1, Math.floor(arr.length / per));
    for (let i = 0; i < arr.length && out.length < cap; i += step) {
      const r = arr[i];
      out.push({
        id: r.id, instance: r.instance, seq: r.seq, backend: r.backend,
        score: r.score, baseline: r.baseline, impr: +(r.score - r.baseline).toFixed(2),
        sweeps: r.sweeps, seed: r.seed, tempHi: r.tempHi, tempLo: r.tempLo,
        replicas: r.replicas, n: r.n, density: +r.density.toFixed(4),
        campaign: r.campaign, generation: r.generation, wallMs: +r.wallMs.toFixed(2),
      });
    }
  }
  return out;
}

// histogram helper
function histogram(vals, bins = 40) {
  if (!vals.length) return { bins: [], min: 0, max: 0 };
  const min = Math.min(...vals), max = Math.max(...vals);
  const span = max - min || 1;
  const h = new Array(bins).fill(0);
  for (const v of vals) { const i = Math.min(bins - 1, Math.floor(((v - min) / span) * bins)); h[i]++; }
  return { bins: h, min: +min.toFixed(2), max: +max.toFixed(2) };
}

// ---------- knowledge graph ----------
function parseGraph(dir, label) {
  const file = join(dir, "knowledge_graph.txt");
  const facts = [];
  for (const line of readLines(file)) {
    // subj|rel|obj||weight|support|variance|"hypothesis #N (conf X)"
    const f = line.split("|");
    if (f.length < 8) continue;
    const confM = (f[7] || "").match(/conf\s+([0-9.]+)/);
    facts.push({
      subject: f[0], relation: f[1], object: f[2] || "",
      weight: +f[4], support: +f[5], variance: +f[6],
      confidence: confM ? +confM[1] : null, source: f[7],
    });
  }
  stamp(label + ":graph", file, facts.length);
  return facts;
}

// ---------- benchmarks ----------
function parseBenchmarks() {
  const idxFile = join(DATA_DIR, "results", "index.json");
  let index = null;
  try { index = JSON.parse(readFileSync(idxFile, "utf8")); stamp("benchmarks:index", idxFile, (index.runs || []).length); } catch {}
  const specsDir = join(DATA_DIR, "specs");
  const specs = [];
  if (existsSync(specsDir)) {
    for (const fn of readdirSync(specsDir).filter((n) => n.endsWith(".json"))) {
      try { specs.push(JSON.parse(readFileSync(join(specsDir, fn), "utf8"))); } catch {}
    }
    stamp("benchmarks:specs", specsDir, specs.length);
  }
  return { index, specs };
}

// ---------- dataset manifest ----------
function parseDataset(dir) {
  const file = join(dir, "dataset", "foundation_manifest.md");
  if (!existsSync(file)) return null;
  const md = readFileSync(file, "utf8");
  const num = (re) => { const m = md.match(re); return m ? +m[1].replace(/,/g, "") : null; };
  const milestones = [];
  const re = /\|\s*(\d[\d,]*)\s*\|\s*(\d[\d,]*)\s*\|\s*(no|yes)\s*\|\s*(\d[\d,]*)×/gi;
  let m; while ((m = re.exec(md))) milestones.push({ milestone: +m[1].replace(/,/g, ""), examples: +m[2].replace(/,/g, ""), reached: m[3].toLowerCase() === "yes", multiplier: +m[4].replace(/,/g, "") });
  stamp("dataset:manifest", file, milestones.length);
  return {
    examples: num(/Examples \(experiments\):\s*\*\*(\d[\d,]*)\*\*/),
    instances: num(/Distinct instances:\s*(\d+)/),
    operators: num(/Operator vocabulary:\s*(\d+)/),
    milestones, markdown: md,
  };
}

// ---------- evaluation ----------
function parseEvaluation(dir) {
  const file = join(dir, "evaluation_report.md");
  if (!existsSync(file)) return null;
  const md = readFileSync(file, "utf8");
  const rows = [];
  // | `signature` | 23 (G1, …) | 1.00 | example |
  const re = /\|\s*`([^`]+)`\s*\|\s*(\d+)\s*\(([^)]*)\)\s*\|\s*([0-9.]+)\s*\|\s*([^|]+?)\s*\|/g;
  let m; while ((m = re.exec(md))) {
    const sig = m[1];
    const family = sig.split(":")[0];
    rows.push({ signature: sig, family, instances: +m[2], confidence: +m[4], example: m[5].trim() });
  }
  stamp("evaluation:report", file, rows.length);
  return { rows, markdown: md };
}

// ---------- reports (the "papers") ----------
function parseReports(dir) {
  const rdir = join(dir, "reports");
  const papers = [];
  if (existsSync(rdir)) {
    for (const fn of readdirSync(rdir).filter((n) => /^report_\d+\.md$/.test(n)).sort()) {
      const md = readFileSync(join(rdir, fn), "utf8");
      const exp = (md.match(/#\s*Research Report\s*—\s*(\d+)\s*experiments/) || [])[1];
      const best = (md.match(/Best result so far:\s*(.+)/) || [])[1] || null;
      const discoveries = [...md.matchAll(/^-\s+(.+?)\s*\(support\s+(\d+),\s*confidence\s+([0-9.]+)\)/gm)]
        .map((d) => ({ text: d[1], support: +d[2], confidence: +d[3] }));
      papers.push({ file: fn, experiments: exp ? +exp : null, best, discoveries, markdown: md });
    }
    stamp("papers:reports", rdir, papers.length);
  }
  return papers.sort((a, b) => (a.experiments || 0) - (b.experiments || 0));
}

// ---------- operator proposals (mined research leads) ----------
function parseProposals(dir) {
  const pdir = join(dir, "proposals");
  const out = [];
  if (!existsSync(pdir)) return out;
  for (const fn of readdirSync(pdir).filter((n) => /^operator_proposal_.*\.md$/.test(n)).sort()) {
    const md = readFileSync(join(pdir, fn), "utf8");
    const name = (md.match(/#\s*Operator Proposal:\s*`([^`]+)`/) || [])[1] ?? fn.replace(/^operator_proposal_|\.md$/g, "");
    const provenance = (md.match(/^Provenance:\s*(.+)$/m) || [])[1] ?? "";
    // pull each ## section body
    const sec = (h) => {
      const m = md.match(new RegExp(`##\\s+${h}\\s*\\n([\\s\\S]*?)(?=\\n##\\s|$)`, "i"));
      return m ? m[1].trim() : "";
    };
    const bullets = (h) => sec(h).split("\n").map((l) => l.replace(/^[-*]\s*/, "").trim()).filter(Boolean);
    out.push({
      file: `proposals/${fn}`, name, provenance,
      idea: sec("Idea"), math: sec("Mathematical description"),
      pseudocode: sec("Pseudocode").replace(/^```[a-z]*\n?|```$/g, "").trim(),
      properties: bullets("Expected properties"),
      capabilities: bullets("Required capabilities"),
      complexity: sec("Expected complexity"), status: sec("Status"),
    });
  }
  stamp("proposals", pdir, out.length);
  return out;
}

// ---------- model registry (versioned weights + lineage) ----------
// Metadata ONLY: the payload is a large weight vector and must never reach the
// browser. We keep its byte length as an honest proxy for model size.
function parseModelRegistry(dirs) {
  const out = [];
  for (const dir of dirs) {
    const file = join(dir, "model_registry.txt");
    if (!existsSync(file)) continue;
    const label = dir.split("/").pop();
    let n = 0;
    for (const line of readLines(file)) {
      const f = line.split("|");
      if (f.length < 9) continue;
      const payload = f.slice(8).join("|");
      out.push({
        campaign: label,
        kind: f[0],
        version: +f[1],
        parent: f[2] === "-1" ? null : +f[2],
        campaign_id: +f[3],
        generation_id: +f[4],
        instance: f[5],
        trained_on: +f[6],
        timestamp: +f[7],
        payload_bytes: payload.length,
      });
      n++;
    }
    stamp(`models:${label}`, file, n);
  }
  return out;
}

// ---------- per-operator behaviour index ----------
// Everything the record actually knows about each operator, keyed by name:
// usage rollup, mined rule signatures, graph facts, and report discovery lines.
function operatorBehaviour(ops, rollup, evaluation, graphFacts, papers) {
  const idx = {};
  for (const op of ops) {
    idx[op] = { operator: op, rollup: rollup.find((r) => r.key === op) ?? null, signatures: [], facts: [], discoveries: [] };
  }
  for (const r of evaluation?.rows ?? []) {
    for (const op of ops) {
      if (r.signature.includes(op) || r.example.includes(op)) {
        idx[op].signatures.push({ signature: r.signature, family: r.family, instances: r.instances, confidence: r.confidence, example: r.example });
      }
    }
  }
  for (const f of graphFacts ?? []) {
    if (idx[f.subject]) {
      idx[f.subject].facts.push({ relation: f.relation, object: f.object, weight: f.weight, support: f.support, confidence: f.confidence });
    }
  }
  // newest report's discoveries mentioning the operator
  const newest = papers?.[papers.length - 1];
  for (const d of newest?.discoveries ?? []) {
    for (const op of ops) {
      if (d.text.startsWith(op + " ") || d.text.includes(" " + op + " ")) {
        idx[op].discoveries.push(d);
      }
    }
  }
  // cap the noisiest lists so the bundle stays lean
  for (const op of ops) {
    idx[op].signatures = idx[op].signatures.slice(0, 8);
    idx[op].facts = idx[op].facts.sort((a, b) => b.support - a.support).slice(0, 8);
    idx[op].discoveries = idx[op].discoveries.slice(0, 6);
  }
  return idx;
}

// ---------- MAIN ----------
log(`data dir: ${DATA_DIR} (${existsSync(DATA_DIR) ? "found" : "MISSING → fixture"})`);

if (!existsSync(join(DATA_DIR, PRIMARY, "ai_experiments.txt"))) {
  // fixture fallback so the build never breaks
  log("primary DB absent — writing fixture");
  const fixture = { fixture: true };
  write("provenance.json", { fixture: true, sources: [], generatedFrom: DATA_DIR });
  for (const n of ["overview", "experiments", "graph", "benchmarks", "dataset", "evaluation", "papers", "operators"]) write(`${n}.json`, { ...fixture });
  write("proposals.json", []);
  write("models.json", []);
  write("operator_behaviour.json", {});
  process.exit(0);
}

// aggregate every campaign once; the primary is one of them (no double-count)
const allAggs = CAMPAIGNS.filter((c) => existsSync(join(DATA_DIR, c, "ai_experiments.txt"))).map((c) => aggregateDb(join(DATA_DIR, c), c));
const primaryAgg = allAggs.find((a) => a.label === PRIMARY) ?? allAggs[0];
const totalRows = allAggs.reduce((s, a) => s + a.rows, 0);
const bestOverall = allAggs.map((a) => a.best).filter(Boolean).sort((a, b) => a.score - b.score)[0];

const primaryGraph = parseGraph(join(DATA_DIR, PRIMARY), PRIMARY);
const benchmarks = parseBenchmarks();
const dataset = parseDataset(join(DATA_DIR, PRIMARY));
const evaluation = parseEvaluation(join(DATA_DIR, PRIMARY));
const papers = parseReports(join(DATA_DIR, PRIMARY));
const proposals = parseProposals(join(DATA_DIR, PRIMARY));
const models = parseModelRegistry(CAMPAIGNS.map((c) => join(DATA_DIR, c)));
const operators = [...primaryAgg.operators].sort();

// overview
write("overview.json", {
  primary: PRIMARY,
  totals: {
    runsPrimary: primaryAgg.rows,
    runsAll: totalRows,
    instances: primaryAgg.instances.size,
    operators: operators.length,
    backends: [...primaryAgg.backends],
    generations: primaryAgg.generations.size,
  },
  best: bestOverall,
  perCampaign: allAggs.map((a) => ({ label: a.label, runs: a.rows, instances: a.instances.size, generations: a.generations.size })),
  graphFactCount: primaryGraph.length,
  latestPaper: papers.length ? { file: papers[papers.length - 1].file, experiments: papers[papers.length - 1].experiments, best: papers[papers.length - 1].best, discoveries: papers[papers.length - 1].discoveries.slice(0, 8) } : null,
});

// experiments
write("experiments.json", {
  totalRows: primaryAgg.rows,
  sample: sampleDb(join(DATA_DIR, PRIMARY), SAMPLE_CAP),
  rollups: {
    byOperator: rollupToArr(primaryAgg.byOperator),
    byBackend: rollupToArr(primaryAgg.byBackend),
    byInstance: rollupToArr(primaryAgg.byInstance),
    byCampaign: rollupToArr(primaryAgg.byCampaign),
  },
  histogram: histogram(primaryAgg.improvements),
});

write("graph.json", { facts: primaryGraph, relations: [...new Set(primaryGraph.map((f) => f.relation))], nodes: [...new Set(primaryGraph.flatMap((f) => [f.subject, f.object].filter(Boolean)))] });
write("benchmarks.json", benchmarks);
write("dataset.json", dataset || { missing: true });
write("evaluation.json", evaluation || { missing: true });
write("papers.json", papers);
write("operators.json", { operators, rollup: rollupToArr(primaryAgg.byOperator) });
write("proposals.json", proposals);
write("models.json", models);
write("operator_behaviour.json",
  operatorBehaviour(operators, rollupToArr(primaryAgg.byOperator), evaluation, primaryGraph, papers));
write("provenance.json", { generatedFrom: DATA_DIR, primary: PRIMARY, sources: provenance });

log(`DONE — primary ${primaryAgg.rows} rows, all-campaigns ${totalRows}, ${primaryGraph.length} facts, ${operators.length} operators, ${papers.length} papers, ${proposals.length} proposals, ${models.length} model snapshots`);
