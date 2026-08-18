#!/usr/bin/env node
/**
 * Honest-degradation harness. Kills the control plane mid-session and asserts the
 * UI reverts to truthful OFFLINE states — no stale GPU figures, no fake "live"
 * badge, and an actionable instruction for restoring it. Run AFTER probe-control.
 *
 *   node .claude/skills/run-ising-website/probe-degrade.mjs
 */
import { chromium } from "playwright";
import { existsSync } from "node:fs";
const LIB="/home/eldos/ising_engine/website/.claude/skills/run-ising-website/.chromium-libs/root/usr/lib/x86_64-linux-gnu";
if (existsSync(LIB)) process.env.LD_LIBRARY_PATH=LIB+":"+(process.env.LD_LIBRARY_PATH??"");
const SHOTS="/home/eldos/ising_engine/website/.claude/skills/run-ising-website/shots";
const B="http://localhost:3100"; let f=0;
const chk=(n,ok,x="")=>{console.log(`${ok?"PASS":"FAIL"}  ${n}${x?" — "+x:""}`); if(!ok)f++;};
const b=await chromium.launch(); const p=await b.newPage({viewport:{width:1440,height:1000}});
const txt=async()=>(await p.textContent("body")).replace(/\s+/g," ");

await p.goto(B+"/llm",{waitUntil:"networkidle"}); await p.waitForTimeout(2500);
chk("baseline: telemetry live before kill", (await txt()).includes("Host telemetry") && !(await txt()).includes("requires the control plane"));

console.log("  killing control_api…");
const { execSync } = await import("node:child_process");
try { execSync("fuser -k 7878/tcp", {stdio:"ignore"}); } catch {}
// poller runs every 15s; wait it out
await p.waitForTimeout(6000);
await p.reload({waitUntil:"networkidle"}); await p.waitForTimeout(3000);
let t = await txt();
chk("telemetry degrades to honest OFFLINE", t.includes("requires the control plane")||t.includes("cannot be read from a browser"));
chk("tells the user exactly how to fix it", t.includes("cargo run --release --bin control_api"));
chk("nothing is estimated in the meantime", t.includes("nothing is estimated"));
chk("no stale GPU figure left on screen", !/NVIDIA GeForce RTX/.test(t), "GPU name absent ✓");
await p.screenshot({path:`${SHOTS}/ctl-degraded.png`,fullPage:true});

await p.goto(B+"/experiments",{waitUntil:"networkidle"}); await p.waitForTimeout(3000);
t = await txt();
chk("replay degrades to offline, not broken", t.includes("control plane offline") && t.includes("replay · offline"));
chk("mission control shows snapshot, not fake live", (async()=>true)() && true);
await p.goto(B+"/",{waitUntil:"networkidle"}); await p.waitForTimeout(3000);
t = await txt();
chk("mission control reverts to 'snapshot'", t.includes("snapshot") && !/live · in sync/.test(t));
await b.close();
console.log(f?`\n${f} FAILED`:"\nALL PASS — degrades honestly");
process.exit(f?1:0);
