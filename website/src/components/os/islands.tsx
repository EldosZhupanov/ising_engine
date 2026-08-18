"use client";

/** Small animated client islands — receive only tiny props (never big data). */

import { useEffect, useState } from "react";
import { motion, useReducedMotion } from "motion/react";

type Stage = { n: string; name: string; module: string; desc: string };

/** The research loop as a ring; the active stage travels (recorded-shaped). */
export function LoopRing({ stages }: { stages: readonly Stage[] }) {
  const reduce = useReducedMotion();
  const [active, setActive] = useState(0);
  useEffect(() => {
    if (reduce) return;
    const id = setInterval(() => setActive((a) => (a + 1) % stages.length), 1400);
    return () => clearInterval(id);
  }, [reduce, stages.length]);

  const R = 128, cx = 160, cy = 160;
  const pts = stages.map((_, i) => {
    const a = (i / stages.length) * Math.PI * 2 - Math.PI / 2;
    return { x: cx + Math.cos(a) * R, y: cy + Math.sin(a) * R };
  });
  const cur = stages[active];

  return (
    <div className="relative mx-auto aspect-square w-full max-w-[340px]">
      <svg viewBox="0 0 320 320" className="h-full w-full">
        <circle cx={cx} cy={cy} r={R} fill="none" stroke="var(--os-hair)" strokeWidth="1" />
        {pts.map((p, i) => { const n = pts[(i + 1) % pts.length]; return <line key={i} x1={p.x} y1={p.y} x2={n.x} y2={n.y} stroke="var(--os-hair)" strokeWidth="1" />; })}
        {pts.map((p, i) => {
          const on = i === active;
          return (
            <g key={i}>
              {on && <circle cx={p.x} cy={p.y} r="13" fill="var(--os-accent)" opacity="0.18" />}
              <circle cx={p.x} cy={p.y} r={on ? 5.5 : 3.5} fill={on ? "var(--os-accent)" : "var(--os-ground)"} stroke={on ? "var(--os-accent)" : "var(--os-hair-2)"} strokeWidth="1.5" style={{ transition: "all .4s" }} />
              <text x={p.x} y={p.y - 16} textAnchor="middle" className="mono" fontSize="9" fill={on ? "var(--os-ink)" : "var(--os-faint)"}>{cur && i === active ? "" : ""}{stages[i].name}</text>
            </g>
          );
        })}
      </svg>
      <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center text-center">
        <div className="mono text-[10px] uppercase tracking-[.2em] text-[var(--os-accent)]">{cur.n} · {cur.name}</div>
        <div className="mono mt-1 text-[10px] text-[var(--os-faint)]">{cur.module}</div>
        <p className="mt-2 max-w-[15ch] text-[11px] leading-[1.45] text-[var(--os-muted)]">{cur.desc}</p>
      </div>
    </div>
  );
}

/** Histogram from precomputed bins. */
export function Histogram({ bins, min, max, height = 120 }: { bins: number[]; min: number; max: number; height?: number }) {
  const peak = Math.max(...bins, 1);
  const [hover, setHover] = useState<number | null>(null);
  return (
    <div>
      <div className="flex items-end gap-[2px]" style={{ height }}>
        {bins.map((b, i) => (
          <div key={i} className="group relative flex-1" onMouseEnter={() => setHover(i)} onMouseLeave={() => setHover(null)}>
            <motion.div initial={{ height: 0 }} whileInView={{ height: `${(b / peak) * 100}%` }} viewport={{ once: true }} transition={{ duration: 0.7, delay: i * 0.005 }}
              className="w-full rounded-t-[2px]" style={{ background: hover === i ? "var(--os-accent)" : "var(--os-cyan)", opacity: hover === i ? 1 : 0.55 }} />
          </div>
        ))}
      </div>
      <div className="mono mt-2 flex justify-between text-[10px] text-[var(--os-faint)]">
        <span>Δ {min.toLocaleString()}</span>
        <span>{hover !== null ? `${bins[hover].toLocaleString()} runs in bin` : "score − baseline (negative = better)"}</span>
        <span>{max.toLocaleString()}</span>
      </div>
    </div>
  );
}
