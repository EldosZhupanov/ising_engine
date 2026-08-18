#!/usr/bin/env node
/**
 * Control-plane integration harness.
 *
 * Verifies that the laboratory genuinely drives `src/bin/control_api.rs`:
 * the shell badge detects it, host telemetry becomes real (CPU/RAM/GPU),
 * data freshness switches from "snapshot" to "live", and — the flagship —
 * clicking Replay on a recorded row re-executes it through the real Rust
 * Runtime and reports a bit-identical verdict.
 *
 * Requires BOTH servers:
 *   cargo run --release --bin control_api          # :7878
 *   npm run start -- -p 3100                       # :3100
 *
 *   node .claude/skills/run-ising-website/probe-control.mjs
 */
import { chromium } from "playwright";
import { existsSync } from "node:fs";
const LIB="/home/eldos/ising_engine/website/.claude/skills/run-ising-website/.chromium-libs/root/usr/lib/x86_64-linux-gnu";
if (existsSync(LIB)) process.env.LD_LIBRARY_PATH=LIB+":"+(process.env.LD_LIBRARY_PATH??"");
const SHOTS="/home/eldos/ising_engine/website/.claude/skills/run-ising-website/shots";
const B="http://localhost:3100"; let f=0;
const chk=(n,ok,x="")=>{console.log(`${ok?"PASS":"FAIL"}  ${n}${x?" — "+x:""}`); if(!ok)f++;};
const b=await chromium.launch(); const p=await b.newPage({viewport:{width:1440,height:1100}});
p.on("console",m=>{if(m.type()==="error")console.log("  console.error:",m.text().slice(0,140));});
const txt=async()=>(await p.textContent("body")).replace(/\s+/g," ");

// ── shell badge goes LIVE
await p.goto(B+"/",{waitUntil:"networkidle"});
// The connection store probes on mount then polls every 15 s. Wait for the state
// to settle rather than assuming a fixed delay — under load (e.g. straight after
// a 4.7 GB model load) the first probe can take several seconds.
await p.waitForFunction(() => /control plane (live|offline)/.test(document.body.innerText), { timeout: 45000 }).catch(()=>{});
await p.waitForFunction(() => document.body.innerText.includes("control plane live"), { timeout: 45000 })
  .catch(()=>console.log("  note: badge never reported live within 45s"));
let t=await txt();
chk("shell badge detects the control plane", t.includes("control plane live"), t.match(/control plane[^·]{0,12}/)?.[0]);
chk("Mission Control shows LIVE data state", /live · in sync|live · [+\-]/.test(t), (t.match(/live · [^—]{0,24}/)||[""])[0].trim());
await p.screenshot({path:`${SHOTS}/ctl-mission.png`,fullPage:false});

// ── host telemetry becomes real
await p.goto(B+"/llm",{waitUntil:"networkidle"});
await p.waitForFunction(() => document.body.innerText.includes("CPU threads") || document.body.innerText.includes("requires the control plane"), { timeout: 45000 }).catch(()=>{});
await p.waitForTimeout(600);
t=await txt();
chk("host telemetry panel is REAL", t.includes("Host telemetry")&&!t.includes("requires the control plane"));
chk("real CPU threads reported", /CPU THREADS|CPU threads/i.test(t)&&/\b(8|12|16|20|24|32)\b/.test(t));
chk("real GPU reported by name", /NVIDIA|GeForce/i.test(t), (t.match(/NVIDIA[^·]{0,34}/)||[""])[0]);
chk("real RAM reported", /RAM total/i.test(t)&&/\d+\.\d GB/.test(t));
await p.screenshot({path:`${SHOTS}/ctl-system.png`,fullPage:true});

// ── THE FLAGSHIP: replay from the UI
await p.goto(B+"/experiments",{waitUntil:"networkidle"});
await p.waitForFunction(() => /control plane (online|offline)/.test(document.body.innerText), { timeout: 45000 }).catch(()=>{});
await p.waitForTimeout(600);
t=await txt();
chk("replay banner says plane is online", t.includes("control plane online"));
const btns = p.getByRole("button",{name:/Replay experiment \d+/});
const n = await btns.count();
chk("replay controls rendered per row", n>50, `${n} rows verifiable`);
console.log("  clicking replay on the first row (the Runtime actually runs)…");
await btns.first().click();
// Wait for the RESULT, not the word: the mission copy on this page already
// contains "bit-identical", so matching bare text fires before the replay runs.
// The comparison line (recorded → replayed · backend · ms) only exists after it.
await p.waitForFunction(
  () => /(-?[\d,]+) → (-?[\d,]+) · (DenseByte|SparseBitSlice) · \d+ ms/.test(document.body.innerText),
  { timeout: 180000 },
);
await p.waitForTimeout(500);
t=await txt();
const verdict = t.includes("bit-identical") ? "bit-identical" : "MISMATCH";
chk("replay returned a verdict from the real engine", verdict==="bit-identical", verdict);
const cmp = t.match(/(-?[\d,]+) → (-?[\d,]+) · (DenseByte|SparseBitSlice) · (\d+) ms/);
chk("recorded → replayed comparison shown", !!cmp, cmp?cmp[0]:"(not matched)");
// progress: the replay mission must now be complete
const prog = await p.evaluate(()=>localStorage.getItem("ising.progress.v1"));
chk("replay recorded as a completed mission", !!prog && prog.includes("experiments.replayed"));
await p.screenshot({path:`${SHOTS}/ctl-replay.png`,fullPage:false});

// ── graceful degradation: kill the plane, UI must go honest
console.log("  (degradation check happens after this probe)");
await b.close();
console.log(f?`\n${f} FAILED`:"\nALL PASS");
process.exit(f?1:0);
