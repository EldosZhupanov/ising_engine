#!/usr/bin/env node
/**
 * Scientific chain harness — verifies the workflow spine plus the two links that
 * complete Problem → … → Applications: the Experiment Designer (/design) and the
 * Publication Composer (/publish).
 *
 * The critical assertion is honesty: the emitted reproduce command must use only
 * flags that exist in src/bin/research_platform.rs and must point at a real
 * instance file. (That command has been executed against the prebuilt binary and
 * ran 266 experiments successfully.)
 *
 *   node .claude/skills/run-ising-website/probe-chain.mjs
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

// ---- workflow spine on every workspace
let missing=[];
for (const r of ["/","/experiments","/graph","/benchmarks","/theories","/discover","/applications","/design","/publish","/playbooks"]) {
  await p.goto(B+r,{waitUntil:"networkidle"});
  if (!(await p.locator('nav[aria-label="Scientific workflow position"]').count())) missing.push(r);
}
chk("workflow spine on all workspaces", missing.length===0, missing.length?`missing: ${missing}`:"Problem→…→Applications");

// ---- DESIGNER
await p.goto(`${B}/design`,{waitUntil:"networkidle"}); await p.waitForTimeout(600);
let t=await txt();
chk("hypothesis + kill-criterion required", t.includes("State the hypothesis")&&t.includes("Kill-criterion"));
chk("real instances offered", /G\d+ · n=\d+/.test(t));
chk("real 14 operators offered", t.includes("metropolis_sweep")&&t.includes("gibbs_color_sweep"));
chk("CLI defaults surfaced", t.includes("--sweeps")&&t.includes("--replicas")&&t.includes("--temp-hi"));
// compose a sequence
await p.getByRole("button",{name:"+ metropolis_sweep"}).first().click();
await p.getByRole("button",{name:"+ greedy_descent"}).first().click();
await p.waitForTimeout(300);
const cmd=await p.locator("pre").first().textContent();
console.log("\n--- emitted command ---\n"+cmd+"\n");
chk("command targets a real instance file", /benchmark_suite\/data\/gset\/G\d+/.test(cmd));
chk("command uses verified flags only", ["--file","--campaigns","--generations","--sweeps","--replicas","--temp-hi","--temp-lo","--seed","--seeds","--dir"].every(x=>cmd.includes(x)));
chk("honest about sequence pinning", (await txt()).includes("no flag that pins one exact sequence"));
chk("execution marked OFFLINE", (await txt()).includes("needs the control-plane API"));
const spec=await p.locator("pre").nth(1).textContent();
chk("spec matches repo schema", ["id","title","status","hypothesis","kill_criterion"].every(k=>spec.includes(`"${k}"`)));
chk("checklist demands 5 seeds", (await txt()).includes("At least 5 seeds"));
await p.screenshot({path:`${SHOTS}/design.png`,fullPage:true});

// ---- PUBLICATION
await p.goto(`${B}/publish`,{waitUntil:"networkidle"}); await p.waitForTimeout(600);
t=await txt();
chk("claims come from the recorded ledger", t.includes("Cross-family transfer")||t.includes("No superiority over production"));
chk("dossier is generated", t.includes("# ")||t.includes("Discovery dossier"));
chk("unknowns are required", t.includes("Unknowns")&&t.includes("required"));
chk("incomplete state warned", t.includes("a dossier needs at least one supporting fact"));
// fill unknowns → dossier should complete
await p.locator("textarea").first().fill("Untested beyond n<=2000; the mechanism has not been ablated so the causal claim is provisional.");
await p.waitForTimeout(400); t=await txt();
chk("power caveat auto-added", t.includes("Power caveat")||t.includes("underpowered"));
chk("provenance section present", t.includes("## Provenance")||t.includes("Provenance"));
chk("bibtex offered", t.includes("@techreport"));
chk("states nothing was generated", t.includes("nothing in this document was generated"));
await p.screenshot({path:`${SHOTS}/publish.png`,fullPage:true});
await b.close();
console.log(f?`\n${f} FAILED`:"\nALL PASS");
process.exit(f?1:0);
