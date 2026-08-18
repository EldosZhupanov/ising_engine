"use client";

/** Ising Engine OS — shared instrument component kit (dark). */

import { useEffect, useRef, useState } from "react";
import { animate, motion, useInView, useReducedMotion } from "motion/react";
import { ICONS, FALLBACK_ICON, type LucideProps } from "./icons";
import type { Status } from "@/lib/nav";
import { STATUS_LABEL } from "@/lib/nav";

export const EASE = [0.16, 1, 0.3, 1] as const;

export function Icon({ name, ...p }: { name: string } & LucideProps) {
  const C = ICONS[name] ?? FALLBACK_ICON;
  return <C {...p} />;
}

export function StatusDot({ status, className = "" }: { status: Status; className?: string }) {
  const c = status === "real" ? "var(--os-confirmed)" : status === "offline" ? "var(--os-warn)" : "var(--os-faint)";
  return <span className={`inline-block h-1.5 w-1.5 rounded-full ${className}`} style={{ background: c, boxShadow: status === "real" ? `0 0 8px ${c}` : "none" }} />;
}

export function StatusBadge({ status }: { status: Status }) {
  const map: Record<Status, string> = {
    real: "text-[var(--os-confirmed)] border-[color:rgba(57,184,122,0.35)] bg-[rgba(57,184,122,0.08)]",
    offline: "text-[var(--os-warn)] border-[color:rgba(224,167,59,0.35)] bg-[rgba(224,167,59,0.08)]",
    planned: "text-[var(--os-faint)] border-[var(--os-hair-2)] bg-[var(--os-raised)]",
  };
  return <span className={`mono inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[10px] tracking-[.12em] ${map[status]}`}><StatusDot status={status} />{STATUS_LABEL[status]}</span>;
}

export function Eyebrow({ n, children }: { n: string; children: React.ReactNode }) {
  return (
    <div className="mono mb-3 flex items-center gap-3 text-[11px] uppercase tracking-[.2em] text-[var(--os-muted)]">
      <span className="text-[var(--os-accent)]">{n}</span>
      <span className="h-px w-7 bg-[var(--os-hair-2)]" />
      <span>{children}</span>
    </div>
  );
}

export function Panel({ children, className = "", ticks = false }: { children: React.ReactNode; className?: string; ticks?: boolean }) {
  return <div className={`os-panel ${ticks ? "os-ticks" : ""} ${className}`}>{children}</div>;
}

export function PanelHeader({ title, sub, status, right }: { title: string; sub?: string; status?: Status; right?: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-3 border-b border-[var(--os-hair)] px-5 py-3.5">
      <div className="min-w-0">
        <div className="flex items-center gap-2.5">
          <h3 className="truncate text-[14px] font-semibold tracking-[-.01em] text-[var(--os-ink)]">{title}</h3>
          {status && <StatusBadge status={status} />}
        </div>
        {sub && <div className="mono mt-0.5 truncate text-[11px] text-[var(--os-faint)]">{sub}</div>}
      </div>
      {right}
    </div>
  );
}

export function Readout({ label, value, sub, accent = false, className = "" }: { label: string; value: React.ReactNode; sub?: string; accent?: boolean; className?: string }) {
  return (
    <div className={className}>
      <div className={`mono text-[2rem] font-semibold leading-none tracking-[-.02em] tnum ${accent ? "text-[var(--os-accent)]" : "text-[var(--os-ink)]"}`}>{value}</div>
      <div className="mono mt-2.5 text-[11px] uppercase tracking-[.06em] text-[var(--os-muted)]">{label}</div>
      {sub && <div className="mt-1 text-[12px] text-[var(--os-faint)]">{sub}</div>}
    </div>
  );
}

export function Pill({ tone, children }: { tone: "confirmed" | "refuted" | "open" | "neutral"; children: React.ReactNode }) {
  const map = {
    confirmed: "text-[var(--os-confirmed)] bg-[rgba(57,184,122,0.1)] border-[color:rgba(57,184,122,0.3)]",
    refuted: "text-[var(--os-refuted)] bg-[rgba(229,98,74,0.1)] border-[color:rgba(229,98,74,0.3)]",
    open: "text-[var(--os-open)] bg-[rgba(73,182,232,0.1)] border-[color:rgba(73,182,232,0.3)]",
    neutral: "text-[var(--os-muted)] bg-[var(--os-raised)] border-[var(--os-hair-2)]",
  } as const;
  return <span className={`mono inline-flex items-center whitespace-nowrap rounded-full border px-2.5 py-1 text-[11px] tracking-[.03em] ${map[tone]}`}>{children}</span>;
}

export function ProvenanceTag({ file, rows, mtime }: { file?: string; rows?: number; mtime?: string | null }) {
  if (!file) return null;
  return (
    <span className="mono inline-flex items-center gap-2 rounded-md border border-[var(--os-hair)] bg-[var(--os-ground-2)] px-2 py-1 text-[10.5px] text-[var(--os-faint)]" title="Provenance">
      <Icon name="FileCode2" size={11} /> {file}{typeof rows === "number" ? ` · ${rows.toLocaleString()} rows` : ""}{mtime ? ` · ${mtime.slice(0, 10)}` : ""}
    </span>
  );
}

export function CountUp({ to, decimals = 0, suffix = "" }: { to: number; decimals?: number; suffix?: string }) {
  const ref = useRef<HTMLSpanElement>(null);
  const inView = useInView(ref, { once: true, margin: "-30px" });
  const reduce = useReducedMotion();
  const [v, setV] = useState(0);
  useEffect(() => {
    if (!inView) return;
    const c = animate(0, to, { duration: reduce ? 0 : 1.3, ease: EASE, onUpdate: setV });
    return () => c.stop();
  }, [inView, reduce, to]);
  return <span ref={ref} className="tnum">{decimals ? v.toFixed(decimals) : Math.round(v).toLocaleString()}{suffix && <span className="ml-0.5 text-[0.55em] text-[var(--os-accent)]">{suffix}</span>}</span>;
}

/** horizontal bar for a normalized value */
export function Bar({ frac, tone = "accent", label, value }: { frac: number; tone?: "accent" | "cyan"; label?: string; value?: string }) {
  const col = tone === "cyan" ? "var(--os-cyan)" : "var(--os-accent)";
  return (
    <div className="grid grid-cols-[130px_1fr_58px] items-center gap-3 py-1.5 sm:grid-cols-[160px_1fr_64px]">
      {label && <span className="mono truncate text-[12px] text-[var(--os-muted)]">{label}</span>}
      <span className="h-1.5 overflow-hidden rounded-full bg-[var(--os-raised)]">
        <motion.span initial={{ width: 0 }} whileInView={{ width: `${Math.max(1, Math.min(100, frac * 100))}%` }} viewport={{ once: true, margin: "-30px" }} transition={{ duration: 1.1, ease: EASE }} className="block h-full rounded-full" style={{ background: col }} />
      </span>
      {value && <span className="mono text-right text-[12px] tnum text-[var(--os-ink)]">{value}</span>}
    </div>
  );
}

export function EmptyState({ title, hint }: { title: string; hint?: string }) {
  return (
    <div className="flex flex-col items-center justify-center gap-2 px-6 py-16 text-center">
      <Icon name="SearchX" size={22} className="text-[var(--os-faint)]" />
      <div className="text-[14px] text-[var(--os-muted)]">{title}</div>
      {hint && <div className="mono text-[12px] text-[var(--os-faint)]">{hint}</div>}
    </div>
  );
}

/** Offline/planned scrim over a designed surface with the go-live note. */
export function NotLive({ status, note }: { status: Status; note?: string }) {
  return (
    <div className="os-panel os-ticks flex flex-col items-start gap-3 p-6">
      <StatusBadge status={status} />
      <p className="max-w-[60ch] text-[13.5px] leading-[1.6] text-[var(--os-muted)]">
        {note ?? "This surface is designed against a typed API seam. It is not wired to a live backend in this build."}
      </p>
    </div>
  );
}
