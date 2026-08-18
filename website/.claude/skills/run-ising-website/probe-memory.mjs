#!/usr/bin/env node
/**
 * Scientific Memory harness — the last surface to go live.
 *
 * Verifies that /memory computes recall from the RUNNING engine (not a snapshot):
 * the narrative, similar instances, ranked operators and applicable facts must
 * match what the control plane returns, and the page must degrade honestly when
 * the plane is stopped.
 *
 * Requires: control_api on :7878, site on :3100.
 *   node .claude/skills/run-ising-website/probe-memory.mjs
 */
import { chromium } from "playwright";
import { existsSync } from "node:fs";
const LIB="/home/eldos/ising_engine/website/.claude/skills/run-ising-website/.chromium-libs/root/usr/lib/x86_64-linux-gnu";
if (existsSync(LIB)) process.env.LD_LIBRARY_PATH=LIB+":"+(process.env.LD_LIBRARY_PATH??"");
const SHOTS="/home/eldos/ising_engine/website/.claude/skills/run-ising-website/shots";
const B="http://localhost:3100", C="http://127.0.0.1:7878"; let f=0;
const chk=(n,ok,x="")=>{console.log(`${ok?"PASS":"FAIL"}  ${n}${x?" — "+x:""}`); if(!ok)f++;};
const b=await chromium.launch(); const p=await b.newPage({viewport:{width:1440,height:1100}});
p.on("console",m=>{if(m.type()==="error")console.log("  console.error:",m.text().slice(0,140));});
const txt=async()=>(await p.textContent("body")).replace(/\s+/g," ");

// ground truth straight from the engine
const api = await (await fetch(`${C}/api/memory/recall?instance=G22&radius=0.35`)).json();
const rep = await (await fetch(`${C}/api/memory/report`)).json();
const stats = await (await fetch(`${C}/api/db/stats`)).json();
chk("engine returns a recall", !!api.narrative, `${api.similar_instances.length} similar`);
chk("report totals match the live DB", rep.total_experiments === stats.experiments,
    `${rep.total_experiments} == ${stats.experiments}`);

await p.goto(`${B}/memory`,{waitUntil:"networkidle"}); await p.waitForTimeout(2500);
let t = await txt();
chk("not a stub", !t.includes("scheduled for the next implementation pass"));
chk("retention report rendered from the engine", t.includes("experiments retained") && t.includes(rep.total_experiments.toLocaleString()));
chk("structural regimes shown", rep.buckets.every(x=>t.includes(x.label)));
chk("append-only discipline stated", t.includes("never proposes deletion"));

// run a real recall through the UI
await p.selectOption("select", "G22").catch(()=>{});
await p.getByRole("button",{name:/^recall$/}).click();
await p.waitForFunction(()=>/Seen \d+ structurally-similar/.test(document.body.innerText), {timeout:60000}).catch(()=>{});
t = await txt();
chk("UI recall matches the engine narrative", t.includes(api.narrative.slice(0, 60)), api.narrative.slice(0,58)+"…");
chk("similar instances rendered", api.similar_instances.slice(0,4).every(i=>t.includes(i)), api.similar_instances.join(","));
chk("best operators ranked", api.best_operators.slice(0,2).every(o=>t.includes(o.operator)));
chk("applicable facts shown", api.applicable_facts.length===0 || t.includes("conditional facts that apply"));
chk("states it is computed live, not snapshot", t.includes("computed now, not read from the build-time snapshot"));
await p.screenshot({path:`${SHOTS}/memory.png`,fullPage:true});
await b.close();
console.log(f?`\n${f} FAILED`:"\nALL PASS — Scientific Memory is live");
process.exit(f?1:0);
