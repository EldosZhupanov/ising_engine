"use client";

/**
 * Experiment Designer — the missing link between a gap and a result.
 *
 * A scientist does two things here, in order: state a falsifiable hypothesis
 * with the kill-criterion decided *first*, then specify the exact run that tests
 * it. The output is two real artifacts:
 *
 *   1. a spec in the repository's own schema (experiments/specs/*.json:
 *      {id, title, status, hypothesis, kill_criterion})
 *   2. a `research_platform` invocation whose flags were verified against
 *      src/bin/research_platform.rs — every flag and default below is real.
 *
 * Executing it needs the control plane, so the button copies rather than runs,
 * and says so. The specification IS the scientific work.
 */

import { useMemo, useState } from "react";
import Link from "next/link";
import { recordAction } from "@/lib/workflow/progress";
import { Icon, Panel, PanelHeader, Pill, StatusBadge } from "@/components/os/kit";
import TermChip from "@/components/os/workflow/TermChip";

export type InstanceInfo = { key: string; runs: number; n?: number; best: number | null };

/** Real CLI defaults, read from research_platform.rs. */
const DEFAULTS = { sweeps: 50, replicas: 32, campaigns: 1, generations: 4, maxOps: 4, tempHi: 2.5, tempLo: 0.1 };

export default function Designer({
  instances, operators, gapSeed,
}: { instances: InstanceInfo[]; operators: string[]; gapSeed?: string }) {
  // ── 1. hypothesis
  const [title, setTitle] = useState("");
  const [hypothesis, setHypothesis] = useState(gapSeed ?? "");
  const [mechanism, setMechanism] = useState("");
  const [kill, setKill] = useState("");

  // ── 2. specification
  const [instance, setInstance] = useState(instances[0]?.key ?? "G22");
  const [seq, setSeq] = useState<string[]>([]);
  const [sweeps, setSweeps] = useState(DEFAULTS.sweeps);
  const [replicas, setReplicas] = useState(DEFAULTS.replicas);
  const [tempHi, setTempHi] = useState(DEFAULTS.tempHi);
  const [tempLo, setTempLo] = useState(DEFAULTS.tempLo);
  const [seed, setSeed] = useState(20260725);
  const [seeds, setSeeds] = useState(5);
  const [copied, setCopied] = useState<string | null>(null);

  const complete = hypothesis.trim().length > 20 && kill.trim().length > 20 && seq.length > 0;

  const cmd = useMemo(() => {
    // Flags verified against src/bin/research_platform.rs (arg/argn parsing).
    const parts = [
      "cargo run --release --bin research_platform --",
      `--file benchmark_suite/data/gset/${instance}`,
      `--campaigns 1 --generations 1`,
      `--sweeps ${sweeps}`,
      `--replicas ${replicas}`,
      `--temp-hi ${tempHi} --temp-lo ${tempLo}`,
      `--seed ${seed}`,
      `--seeds ${seeds}`,
      `--dir experiments/my_campaign`,
    ];
    return parts.join(" \\\n    ");
  }, [instance, sweeps, replicas, tempHi, tempLo, seed, seeds]);

  const specJson = useMemo(() => JSON.stringify({
    id: "EXP-XXXX",
    title: title || "(title)",
    status: "proposed",
    hypothesis: hypothesis || "(state a falsifiable claim)",
    mechanism: mechanism || "(why it should hold)",
    kill_criterion: kill || "(the condition under which this idea is dead)",
    plan: {
      instance, operator_sequence: seq, sweeps, replicas,
      temp_hi: tempHi, temp_lo: tempLo, seed, seeds,
    },
  }, null, 2), [title, hypothesis, mechanism, kill, instance, seq, sweeps, replicas, tempHi, tempLo, seed, seeds]);

  const copy = (what: string, text: string) => {
    navigator.clipboard?.writeText(text).then(() => {
      setCopied(what);
      recordAction("experiments.recipe.copied");
      setTimeout(() => setCopied(null), 1800);
    }).catch(() => setCopied("clipboard blocked"));
  };

  const toggleOp = (op: string) => setSeq((s) => s.length < 8 ? [...s, op] : s);

  return (
    <div className="grid gap-4 lg:grid-cols-[1.05fr_1fr]">
      {/* ─── 1. HYPOTHESIS ─── */}
      <div className="space-y-4">
        <Panel>
          <PanelHeader title="1 · State the hypothesis" status="real"
            sub="the kill-criterion is written before the run, not after" />
          <div className="space-y-3.5 p-5">
            <Field label="Working title" hint="how you will refer to this experiment">
              <input value={title} onChange={(e) => setTitle(e.target.value)}
                placeholder="Fusing gibbs_color_sweep with greedy_descent"
                className={inputCls} />
            </Field>
            <Field label="Falsifiable claim" hint="must be stated so that an outcome could contradict it">
              <textarea value={hypothesis} onChange={(e) => setHypothesis(e.target.value)} rows={3}
                placeholder="A fused gibbs→greedy operator reaches the same energy as the two-operator sequence in fewer total sweeps on sparse G-Set instances."
                className={inputCls} />
            </Field>
            <Field label="Proposed mechanism" hint="why it should hold — the causal story">
              <textarea value={mechanism} onChange={(e) => setMechanism(e.target.value)} rows={2}
                placeholder="Both phases touch the same neighbourhood, so ΔE can be computed once per edge and reused instead of recomputed."
                className={inputCls} />
            </Field>
            <Field label="Kill-criterion" hint="decide now what result ends this idea" danger>
              <textarea value={kill} onChange={(e) => setKill(e.target.value)} rows={2}
                placeholder="If the fused operator does not beat the two-operator sequence at matched total sweeps on ≥ 60% of instances over 5 seeds, the idea is dead and recorded as refuted."
                className={inputCls} />
            </Field>
            <div className="flex flex-wrap items-center gap-1.5 border-t border-[var(--os-hair)] pt-3">
              <span className="mono text-[10px] uppercase tracking-[.12em] text-[var(--os-faint)]">concepts</span>
              {["kill-criterion", "pre-registration", "hypothesis", "causal"].map((k) => <TermChip key={k} k={k} />)}
            </div>
          </div>
        </Panel>

        <Panel>
          <PanelHeader title="2 · Specify the run" status="real" sub="every field maps to a real CLI flag" />
          <div className="space-y-3.5 p-5">
            <Field label="Instance" hint={`${instances.length} instances in the record`}>
              <select value={instance} onChange={(e) => setInstance(e.target.value)} className={inputCls}>
                {instances.map((i) => (
                  <option key={i.key} value={i.key}>
                    {i.key}{i.n ? ` · n=${i.n}` : ""} · {i.runs.toLocaleString()} runs{i.best !== null ? ` · best ${i.best.toLocaleString()}` : ""}
                  </option>
                ))}
              </select>
            </Field>

            <Field label="Operator sequence" hint="click to append · the engine composes these in order">
              <div className="flex flex-wrap gap-1.5">
                {operators.map((op) => (
                  <button key={op} onClick={() => toggleOp(op)}
                    className="mono rounded-[7px] border border-[var(--os-hair-2)] px-2 py-1 text-[11px] text-[var(--os-muted)] transition-colors hover:border-[var(--os-accent)] hover:text-[var(--os-ink)]">
                    + {op}
                  </button>
                ))}
              </div>
              <div className="mt-2.5 min-h-[38px] rounded-[8px] border border-dashed border-[var(--os-hair-2)] bg-[var(--os-ground)] p-2">
                {seq.length === 0 ? (
                  <span className="mono text-[11px] text-[var(--os-faint)]">no operators yet — a sequence is the algorithm</span>
                ) : (
                  <div className="flex flex-wrap items-center gap-1.5">
                    {seq.map((op, i) => (
                      <span key={i} className="mono inline-flex items-center gap-1 rounded-[6px] bg-[var(--os-accent-soft)] px-2 py-1 text-[11px] text-[var(--os-ink)]">
                        {op}
                        <button onClick={() => setSeq((s) => s.filter((_, j) => j !== i))} aria-label={`remove ${op}`}
                          className="text-[var(--os-faint)] hover:text-[var(--os-refuted)]"><Icon name="X" size={10} /></button>
                      </span>
                    ))}
                    <button onClick={() => setSeq([])} className="mono ml-1 text-[10px] text-[var(--os-faint)] hover:text-[var(--os-ink)]">clear</button>
                  </div>
                )}
              </div>
            </Field>

            <div className="grid grid-cols-2 gap-3">
              <Num label="sweeps" flag="--sweeps" value={sweeps} set={setSweeps} min={1} max={500} def={DEFAULTS.sweeps} />
              <Num label="replicas" flag="--replicas" value={replicas} set={setReplicas} min={1} max={128} def={DEFAULTS.replicas} />
              <Num label="temp hi" flag="--temp-hi" value={tempHi} set={setTempHi} min={0.1} max={20} step={0.1} def={DEFAULTS.tempHi} />
              <Num label="temp lo" flag="--temp-lo" value={tempLo} set={setTempLo} min={0.01} max={5} step={0.01} def={DEFAULTS.tempLo} />
              <Num label="seed" flag="--seed" value={seed} set={setSeed} min={0} max={999999999} />
              <Num label="seeds (repeats)" flag="--seeds" value={seeds} set={setSeeds} min={1} max={50} />
            </div>
            <p className="mono text-[10.5px] leading-[1.5] text-[var(--os-faint)]">
              Repeats matter: a single seed cannot separate an effect from variance. Five seeds is the minimum for a
              claim, and the engine replays each one bit-identically.
            </p>
          </div>
        </Panel>
      </div>

      {/* ─── OUTPUT ─── */}
      <div className="space-y-4">
        <Panel ticks>
          <PanelHeader title="Reproduce command" status="real"
            sub="flags verified against src/bin/research_platform.rs"
            right={<button onClick={() => copy("cmd", cmd)} className="mono rounded-[7px] border border-[var(--os-hair-2)] px-2.5 py-1 text-[11px] text-[var(--os-muted)] hover:text-[var(--os-ink)]">
              {copied === "cmd" ? "copied ✓" : "copy"}
            </button>} />
          <div className="p-5">
            <pre className="mono overflow-x-auto rounded-[8px] border border-[var(--os-hair)] bg-[var(--os-ground)] p-3.5 text-[11.5px] leading-[1.6] text-[var(--os-ink)]">{cmd}</pre>
            {seq.length > 0 && (
              <p className="mono mt-2.5 text-[10.5px] leading-[1.55] text-[var(--os-warn)]">
                Note: the CLI evolves operator sequences itself — it has no flag that pins one exact sequence. Your
                chosen sequence <span className="text-[var(--os-ink)]">[{seq.join(" → ")}]</span> is recorded in the spec
                below as the hypothesis under test; pinning it for execution is a control-plane capability.
              </p>
            )}
            <div className="mt-3 flex flex-wrap items-center gap-2 border-t border-[var(--os-hair)] pt-3">
              <StatusBadge status="offline" />
              <span className="mono text-[10.5px] text-[var(--os-faint)]">
                Running it from the browser needs the control-plane API. Copy and run it in a terminal today.
              </span>
            </div>
          </div>
        </Panel>

        <Panel ticks>
          <PanelHeader title="Experiment spec" status="real"
            sub="the repository's own schema — experiments/specs/*.json"
            right={<button onClick={() => copy("spec", specJson)} className="mono rounded-[7px] border border-[var(--os-hair-2)] px-2.5 py-1 text-[11px] text-[var(--os-muted)] hover:text-[var(--os-ink)]">
              {copied === "spec" ? "copied ✓" : "copy"}
            </button>} />
          <div className="p-5">
            <pre className="mono max-h-[340px] overflow-auto rounded-[8px] border border-[var(--os-hair)] bg-[var(--os-ground)] p-3.5 text-[11px] leading-[1.6] text-[var(--os-muted)]">{specJson}</pre>
          </div>
        </Panel>

        <Panel className="p-5">
          <div className="mb-2 flex items-center gap-2">
            <Pill tone={complete ? "confirmed" : "neutral"}>{complete ? "specification complete" : "incomplete"}</Pill>
          </div>
          <ul className="space-y-1.5 text-[12.5px] text-[var(--os-muted)]">
            <Check ok={hypothesis.trim().length > 20}>A falsifiable claim is stated</Check>
            <Check ok={kill.trim().length > 20}>A kill-criterion is fixed in advance</Check>
            <Check ok={seq.length > 0}>An operator sequence is proposed</Check>
            <Check ok={seeds >= 5}>At least 5 seeds, so variance can be separated from effect</Check>
          </ul>
          {complete && (
            <p className="mt-3 border-t border-[var(--os-hair)] pt-3 text-[12.5px] leading-[1.55] text-[var(--os-ink)]">
              This is a complete unit of scientific work. When it has run, bring the outcome to{" "}
              <Link href="/publish" className="text-[var(--os-accent)] hover:underline">Publication</Link> — with its
              evidence, confidence and unknowns.
            </p>
          )}
        </Panel>
      </div>
    </div>
  );
}

const inputCls = "mono mt-1 w-full rounded-[8px] border border-[var(--os-hair-2)] bg-[var(--os-ground)] px-3 py-2 text-[12.5px] leading-[1.55] text-[var(--os-ink)] placeholder:text-[var(--os-faint)] focus:border-[var(--os-accent)] focus:outline-none";

function Field({ label, hint, children, danger }: { label: string; hint?: string; children: React.ReactNode; danger?: boolean }) {
  return (
    <label className="block">
      <span className={`mono text-[10px] uppercase tracking-[.12em] ${danger ? "text-[var(--os-refuted)]" : "text-[var(--os-faint)]"}`}>{label}</span>
      {hint && <span className="mono ml-2 text-[10px] text-[var(--os-faint)]">{hint}</span>}
      {children}
    </label>
  );
}

function Num({ label, flag, value, set, min, max, step = 1, def }:
  { label: string; flag: string; value: number; set: (n: number) => void; min: number; max: number; step?: number; def?: number }) {
  return (
    <label className="block">
      <span className="mono flex items-baseline justify-between text-[10px] uppercase tracking-[.1em] text-[var(--os-faint)]">
        <span>{label}</span><span className="text-[var(--os-hair-2)]">{flag}</span>
      </span>
      <input type="number" value={value} min={min} max={max} step={step}
        onChange={(e) => set(Number(e.target.value))}
        className="mono mt-1 w-full rounded-[8px] border border-[var(--os-hair-2)] bg-[var(--os-ground)] px-2.5 py-1.5 text-[12px] tnum text-[var(--os-ink)] focus:border-[var(--os-accent)] focus:outline-none" />
      {def !== undefined && value !== def && (
        <span className="mono mt-0.5 block text-[9.5px] text-[var(--os-faint)]">CLI default {def}</span>
      )}
    </label>
  );
}

function Check({ ok, children }: { ok: boolean; children: React.ReactNode }) {
  return (
    <li className="flex items-start gap-2">
      <span className={`mt-[3px] flex h-3.5 w-3.5 shrink-0 items-center justify-center rounded-full border ${ok ? "border-[var(--os-confirmed)] bg-[var(--os-confirmed)]" : "border-[var(--os-hair-2)]"}`}>
        {ok && <Icon name="Check" size={9} className="text-[var(--os-ground)]" />}
      </span>
      <span className={ok ? "text-[var(--os-ink)]" : ""}>{children}</span>
    </li>
  );
}
