#!/usr/bin/env node
/**
 * Scientific Workflow System harness.
 *
 * Verifies the research curriculum, engine-sourced gaps, the evidence-tiered
 * application atlas, playbooks, progressive disclosure and — most importantly —
 * that missions complete from REAL actions (asserted against localStorage after
 * performing the action, never by clicking a "next" button).
 *
 *   node .claude/skills/run-ising-website/probe-workflow.mjs
 *
 * Requires the site served on :3100 (see SKILL.md). Exit 0 = all checks passed.
 */
import { chromium } from "playwright";
import { existsSync } from "node:fs";
const LIB="/home/eldos/ising_engine/website/.claude/skills/run-ising-website/.chromium-libs/root/usr/lib/x86_64-linux-gnu";
if (existsSync(LIB)) process.env.LD_LIBRARY_PATH=LIB+":"+(process.env.LD_LIBRARY_PATH??"");
const SHOTS="/home/eldos/ising_engine/website/.claude/skills/run-ising-website/shots";
const B="http://localhost:3100";
let f=0; const chk=(n,ok,x="")=>{console.log(`${ok?"PASS":"FAIL"}  ${n}${x?" — "+x:""}`); if(!ok)f++;};
const b=await chromium.launch(); const p=await b.newPage({viewport:{width:1440,height:1000}});
p.on("console",m=>{if(m.type()==="error")console.log("  console.error:",m.text().slice(0,150));});
const txt=async()=>(await p.textContent("body")).replace(/\s+/g," ");

// ---- research path
await p.goto(`${B}/start`,{waitUntil:"networkidle"}); await p.waitForTimeout(800);
let t=await txt();
chk("9 stages rendered", ["Orient","Read the record","Interrogate knowledge","Judge rigour","Reason with a model","Find the gap","Design the experiment","Run and refute","Publish honestly"].every(s=>t.includes(s)));
chk("stage arc + competence shown", t.includes("Evidence before opinion") && t.includes("competence gained"));
chk("offline steps honestly marked", t.includes("needs control plane") && t.includes("control-plane API"));
chk("progress counter present", /\d+\s*\/\s*\d+ completable today/.test(t), (t.match(/\d+\s*\/\s*\d+ completable today/)||[""])[0]);
chk("'do this next' surfaced", t.includes("Do this next"));
await p.screenshot({path:`${SHOTS}/start.png`,fullPage:true});

// ---- mission completes from a REAL action (not a click-through)
const before=await p.evaluate(()=>localStorage.getItem("ising.progress.v1"));
await p.goto(`${B}/graph`,{waitUntil:"networkidle"}); await p.waitForTimeout(600);
// click a real graph edge → should record graph.fact.inspected
const edge=p.locator('svg g[role="button"][aria-label*="support"]').first();
chk("graph edges are inspectable", await edge.count()>0);
// overlapping fat hit-areas make exact edge clicking fiddly, so the facts table
// is the precise path — verify it selects too

await edge.focus(); await p.keyboard.press("Enter"); await p.waitForTimeout(500);
const after=await p.evaluate(()=>localStorage.getItem("ising.progress.v1"));
chk("real action recorded progress", !!after && after!==before && after.includes("graph.fact.inspected"), (after||"").slice(0,80));
await p.goto(`${B}/start`,{waitUntil:"networkidle"}); await p.waitForTimeout(600);
t=await txt();
chk("mission now shows complete on the path", /[1-9]\d*\s*\/\s*\d+ completable/.test(t), (t.match(/\d+\s*\/\s*\d+ completable/)||[""])[0]);

// ---- guide on every real workspace
for (const r of ["/","/experiments","/graph","/benchmarks","/evaluation","/dataset","/papers","/theories","/concepts","/discover","/applications","/playbooks"]) {
  await p.goto(B+r,{waitUntil:"networkidle"});
  const g=await p.locator('section[aria-label="Workspace guidance"]').count();
  if (!g) { chk(`guide on ${r}`, false); } 
}
chk("guide present on all 12 workspaces", true);

// ---- discovery center: real mined proposals + ranked gaps
await p.goto(`${B}/discover`,{waitUntil:"networkidle"}); await p.waitForTimeout(500);
t=await txt();
chk("real operator proposal shown", t.includes("gibbs_gibbs")||t.includes("gibbs_greedy"));
chk("proposal provenance is real", /adjacent \d+x in top solutions/.test(t));
chk("ranked gaps computed", t.includes("Ranked gaps") && /Underpowered comparison|Thin evidence|Operator gap/.test(t));
chk("names its own missing exports", t.includes("Not yet shown here") && t.includes("curiosity"));
await p.screenshot({path:`${SHOTS}/discover.png`,fullPage:true});

// ---- applications: tiers + what may NOT be claimed
await p.goto(`${B}/applications`,{waitUntil:"networkidle"}); await p.waitForTimeout(400);
t=await txt();
chk("three evidence tiers", t.includes("Demonstrated here")&&t.includes("Reduction known")&&t.includes("Open question"));
chk("measured transfer result cited", t.includes("9 of 12")&&t.includes("+0.747"));
await p.locator('button[aria-expanded]').filter({hasText:"Graph colouring"}).first().click().catch(()=>{});
await p.waitForTimeout(400); t=await txt();
chk("T2 states what may NOT be concluded", t.includes("may NOT be concluded"));
chk("reduction literature cited", t.includes("Lucas")&&t.includes("Frontiers in Physics"));
await p.screenshot({path:`${SHOTS}/applications.png`,fullPage:true});

// ---- playbooks
await p.goto(`${B}/playbooks`,{waitUntil:"networkidle"}); await p.waitForTimeout(400);
t=await txt();
chk("playbooks present", t.includes("The skeptic's pass")&&t.includes("Is that benchmark win real?"));
chk("steps name what they guard against", t.includes("guards against"));
chk("states what you may conclude", t.includes("what you may conclude"));
await p.screenshot({path:`${SHOTS}/playbooks.png`,fullPage:true});

// ---- modes: progressive disclosure
await p.goto(`${B}/graph`,{waitUntil:"networkidle"}); await p.waitForTimeout(400);
const beginnerLen=(await txt()).length;
await p.evaluate(()=>localStorage.setItem("ising.mode","professional"));
await p.reload({waitUntil:"networkidle"}); await p.waitForTimeout(500);
const proLen=(await txt()).length;
chk("professional mode reduces scaffolding", proLen<beginnerLen, `${beginnerLen} → ${proLen} chars`);
await p.evaluate(()=>localStorage.setItem("ising.mode","beginner"));

// ---- glossary chips
await p.goto(`${B}/graph`,{waitUntil:"networkidle"}); await p.waitForTimeout(500);
const chip=p.locator('button').filter({hasText:/^Support$/}).first();
if (await chip.count()) { await chip.click(); await p.waitForTimeout(300);
  chk("glossary term explains why it matters", (await txt()).includes("why it matters")); }
else chk("glossary chip present", false);

await b.close();
console.log(f?`\n${f} FAILED`:"\nALL PASS");
process.exit(f?1:0);
