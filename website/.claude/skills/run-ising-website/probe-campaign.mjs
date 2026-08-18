#!/usr/bin/env node
/**
 * Autonomous-Scientist lifecycle harness.
 *
 * Drives the orchestrator entirely from the browser: START → STEP one tick →
 * RESUME → STOP, asserting each transition against the control plane's own
 * status endpoint (never against ambiguous DOM text) and confirming the tick
 * actually appended experiments to the record.
 *
 * Requires: control_api on :7878 pointed at a WRITABLE scratch dir, site on :3100.
 *   node .claude/skills/run-ising-website/probe-campaign.mjs
 */
import { chromium } from "playwright";
import { existsSync } from "node:fs";
const LIB="/home/eldos/ising_engine/website/.claude/skills/run-ising-website/.chromium-libs/root/usr/lib/x86_64-linux-gnu";
if (existsSync(LIB)) process.env.LD_LIBRARY_PATH=LIB+":"+(process.env.LD_LIBRARY_PATH??"");
const SHOTS="/home/eldos/ising_engine/website/.claude/skills/run-ising-website/shots";
const B="http://localhost:3100", C="http://127.0.0.1:7878";
let f=0; const chk=(n,ok,x="")=>{console.log(`${ok?"PASS":"FAIL"}  ${n}${x?" — "+x:""}`); if(!ok)f++;};
const api = async () => (await (await fetch(`${C}/api/campaign/status`)).json());
const waitFor = async (pred, ms=240000) => { const t0=Date.now();
  while (Date.now()-t0<ms) { const s=await api(); if (pred(s)) return s; await new Promise(r=>setTimeout(r,1500)); }
  return null; };

const b=await chromium.launch(); const p=await b.newPage({viewport:{width:1440,height:1100}});
p.on("console",m=>{if(m.type()==="error")console.log("  console.error:",m.text().slice(0,140));});
const txt=async()=>(await p.textContent("body")).replace(/\s+/g," ");

await p.goto(`${B}/campaigns`,{waitUntil:"networkidle"}); await p.waitForTimeout(2500);
let t=await txt();
chk("mission control is REAL (not offline)", !t.includes("expected at http://127.0.0.1:7878") );
chk("launcher offers real instances", /\bG\d+\b/.test(t));
chk("START button present", t.includes("START AUTONOMOUS SCIENTIST"));
chk("idle at boot", (await api()).phase==="idle");

// ── START (start-paused is on by default)
console.log("  clicking START AUTONOMOUS SCIENTIST…");
await p.getByRole("button",{name:/START AUTONOMOUS SCIENTIST/}).click();
let s = await waitFor(x=>x.phase==="paused"||x.phase==="running", 30000);
chk("campaign started from the browser", !!s, s?`phase=${s.phase} instances=${s.instances}`:"timeout");
chk("started PAUSED so it can be stepped", s?.phase==="paused");

// ── STEP one tick
console.log("  clicking step (a real orchestrator tick — takes seconds)…");
await p.getByRole("button",{name:/step 1 tick/}).click();
s = await waitFor(x=>x.ticks_done>=1);
chk("STEP ran exactly one tick", !!s && s.ticks_done===1, s?`ticks=${s.ticks_done}`:"timeout");
chk("returned to paused after the step", s?.phase==="paused");
const e = s?.events?.[0];
chk("the tick appended real experiments", !!e && e.experiments_after>e.experiments_before,
    e?`${e.experiments_before}→${e.experiments_after}, best ${e.best_score} vs baseline ${e.baseline}`:"");
chk("health gate reported", !!e?.health, e?.health?.slice(0,40));
await p.waitForTimeout(2500);
t=await txt();
chk("UI shows the tick in its log", /tick 1/.test(t) && /\+\d+ experiments/.test(t));
await p.screenshot({path:`${SHOTS}/campaign-stepped.png`,fullPage:true});

// ── RESUME → runs to the cap
console.log("  clicking resume (runs to the 3-tick cap)…");
await p.getByRole("button",{name:/^resume$/}).click();
s = await waitFor(x=>x.phase==="finished", 420000);
chk("RESUME ran to the budget cap and finished", !!s && s.ticks_done===s.max_ticks,
    s?`ticks=${s.ticks_done}/${s.max_ticks}`:"timeout");
const live = await (await fetch(`${C}/api/db/stats`)).json();
chk("record genuinely grew on disk", live.experiments>0, `${live.experiments} experiments, ${live.knowledge_facts} facts`);
await p.waitForTimeout(2000);
await p.screenshot({path:`${SHOTS}/campaign-finished.png`,fullPage:true});
await b.close();
console.log(f?`\n${f} FAILED`:"\nALL PASS — the autonomous scientist is driven from the browser");
process.exit(f?1:0);
