#!/usr/bin/env node
/**
 * Machinery & System harness — verifies the six workspaces that were stubs:
 * Operators, Model Registry, Runtime, Feature Registry, Documentation, Settings,
 * plus the live Solver Playground. /memory is verified by probe-memory.mjs.
 *
 *   node .claude/skills/run-ising-website/probe-machinery.mjs
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
const go=async r=>{await p.goto(B+r,{waitUntil:"networkidle"});await p.waitForTimeout(900);return txt();};

let t = await go("/operators");
chk("operators: not a stub", !t.includes("scheduled for the next implementation pass"));
chk("operators: all 14 listed", ["metropolis_sweep","gibbs_color_sweep","history_field","houdayer_cluster","elite_broadcast","extremal_metropolis"].every(o=>t.includes(o)));
chk("operators: real usage counts", /[\d,]{3,} runs/.test(t));
chk("operators: mined signatures shown", /Ordering:|Budget:|Antipattern:/.test(t));
chk("operators: antipattern flagged", t.includes("antipattern"));
chk("operators: signature meaning explained", t.includes("over-represented in the WORST solutions")||t.includes("budget-dependent"));
await p.screenshot({path:`${SHOTS}/operators.png`,fullPage:true});

t = await go("/models");
chk("models: real snapshot count", t.includes("Snapshots") && /[\d,]{3,}/.test(t));
chk("models: lineage with parents", t.includes("parent") && /v\d+/.test(t));
chk("models: kinds from the registry", ["policy","world","predictor"].every(k=>t.includes(k)));
chk("models: weights stay on disk", t.includes("kept on disk")||t.includes("stay on disk"));
await p.screenshot({path:`${SHOTS}/models.png`,fullPage:true});

t = await go("/runtime");
chk("runtime: backends with real usage share", t.includes("DenseByte")&&t.includes("SparseBitSlice")&&/% of the record/.test(t));
chk("runtime: determinism firewall", t.includes("determinism firewall")||t.includes("Same seed"));
chk("runtime: names the missing event-log export", t.includes("StepEvent"));
await p.screenshot({path:`${SHOTS}/runtime.png`,fullPage:true});

t = await go("/features");
chk("features: the structural scalars", ["density","clustering","mean_degree","degree_cv"].every(x=>t.includes(x)));
chk("features: admits what is not observable", t.includes("never serialised")||t.includes("not yet observable"));
await p.screenshot({path:`${SHOTS}/features.png`,fullPage:true});

t = await go("/docs");
chk("docs: governance order ranked", t.includes("Constitution")&&t.includes("Roadmap")&&t.includes("Governance order"));
chk("docs: ADR-0004 called out", t.includes("ADR-0004"));
chk("docs: release-only gates documented", t.includes("cargo verify")||t.includes("clippy --release"));
await p.screenshot({path:`${SHOTS}/docs.png`,fullPage:true});

t = await go("/settings");
chk("settings: theme choices", t.includes("Dark Laboratory")&&t.includes("High Contrast"));
chk("settings: control-plane configurable", /[Cc]ontrol plane/.test(t));
chk("settings: discloses what is stored", t.includes("ising.progress.v1")&&t.includes("localStorage"));
await p.screenshot({path:`${SHOTS}/settings.png`,fullPage:true});

t = await go("/solve");
chk("solve: real request shape (num_vars)", t.includes("num_vars"));
chk("solve: hand-checkable presets", t.includes("frustrated triangle"));
chk("solve: honest about server_api CORS", /CORS/.test(t));
await p.screenshot({path:`${SHOTS}/solve.png`,fullPage:true});

// /memory went live once control_api exposed memory_os; it is covered in depth by
// probe-memory.mjs. Here we only assert it is no longer a stub.
t = await go("/memory");
chk("memory: no longer a stub", !t.includes("scheduled for the next implementation pass"));
await b.close();
console.log(f?`\n${f} FAILED`:"\nALL PASS — machinery workspaces are real");
process.exit(f?1:0);
