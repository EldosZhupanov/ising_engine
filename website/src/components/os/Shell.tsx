"use client";

/** Ising Engine OS — the shell: rail + top bar + status bar + ⌘K palette. */

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { AnimatePresence, motion } from "motion/react";
import { NAV, ALL_ITEMS, type Status } from "@/lib/nav";
import { summary, dataAvailable } from "@/data/summary";
import { Icon, StatusDot, StatusBadge } from "./kit";
import { ThemeToggle } from "./Theme";
import { useControl } from "@/lib/control/useControl";
import ModeToggle from "./workflow/ModeToggle";

function Logo() {
  return (
    <Link href="/" className="flex items-center gap-2.5 px-2.5 py-1">
      <span className="relative inline-flex h-7 w-7 items-center justify-center">
        <svg viewBox="0 0 32 32" className="h-7 w-7" fill="none" stroke="var(--os-accent)" strokeWidth="2.4" strokeLinecap="round">
          <path d="M11 13.5a5 5 0 0 1 0-1l-2.2-2.2a5.5 5.5 0 1 0 7.8 7.8" />
          <path d="M21 18.5a5 5 0 0 1 0 1l2.2 2.2a5.5 5.5 0 1 0-7.8-7.8" />
        </svg>
      </span>
      <div className="leading-none">
        <div className="text-[13.5px] font-semibold tracking-[-.01em] text-[var(--os-ink)]">Ising Engine</div>
        <div className="mono text-[9.5px] uppercase tracking-[.18em] text-[var(--os-faint)]">Scientific OS</div>
      </div>
    </Link>
  );
}

function Rail() {
  const path = usePathname();
  const active = (href: string) => (href === "/" ? path === "/" : path.startsWith(href));
  return (
    <aside className="fixed left-0 top-0 z-30 hidden h-screen w-[248px] flex-col border-r border-[var(--os-hair)] bg-[var(--os-ground-2)] lg:flex">
      <div className="border-b border-[var(--os-hair)] px-3 py-3"><Logo /></div>
      <nav className="flex-1 overflow-y-auto px-2.5 py-3">
        {NAV.map((group) => (
          <div key={group.group} className="mb-4">
            <div className="mono px-2.5 pb-1.5 text-[9.5px] uppercase tracking-[.18em] text-[var(--os-faint)]">{group.group}</div>
            {group.items.map((it) => {
              const on = active(it.href);
              return (
                <Link key={it.href} href={it.href}
                  className={`group relative flex items-center gap-2.5 rounded-[9px] px-2.5 py-2 text-[13px] transition-colors ${on ? "bg-[var(--os-accent-soft)] text-[var(--os-ink)]" : "text-[var(--os-muted)] hover:bg-[var(--os-raised)] hover:text-[var(--os-ink)]"}`}>
                  {on && <span className="absolute left-0 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-[var(--os-accent)]" />}
                  <Icon name={it.icon} size={15.5} strokeWidth={1.9} className={on ? "text-[var(--os-accent)]" : ""} />
                  <span className="flex-1 truncate">{it.label}</span>
                  <StatusDot status={it.status} />
                </Link>
              );
            })}
          </div>
        ))}
      </nav>
    </aside>
  );
}

function ConnBadge() {
  // Real connection state — polled from control_api. When it is down the badge
  // says so and the live surfaces stay honestly OFFLINE.
  const { status, url, health, recheck } = useControl();
  const label =
    status === "online" ? "control plane live" : status === "checking" ? "control plane…" : "control plane offline";
  const dot: Status = status === "online" ? "real" : status === "checking" ? "planned" : "offline";
  const title =
    status === "online"
      ? `${health?.service ?? "control_api"} at ${url} · dir ${health?.dir ?? "?"} · replay + host telemetry available`
      : `Not reachable at ${url}. Start it with: cargo run --release --bin control_api`;
  return (
    <button
      onClick={recheck}
      title={title}
      aria-label={`${label} — click to re-check`}
      className="mono hidden items-center gap-1.5 rounded-full border border-[var(--os-hair-2)] bg-[var(--os-panel)] px-2.5 py-1 text-[11px] text-[var(--os-muted)] transition-colors hover:text-[var(--os-ink)] sm:inline-flex"
    >
      <StatusDot status={dot} /> {label}
    </button>
  );
}

function TopBar({ onCmd, onMenu }: { onCmd: () => void; onMenu: () => void }) {
  const path = usePathname();
  const item = ALL_ITEMS.find((i) => (i.href === "/" ? path === "/" : path.startsWith(i.href)));
  return (
    <header className="os-glass sticky top-0 z-20 flex items-center gap-3 border-x-0 border-t-0 px-4 py-2.5 md:gap-4 md:px-6">
      {/* mobile: open the workspace drawer (the rail is hidden under lg) */}
      <button onClick={onMenu} aria-label="Open navigation"
        className="rounded-[8px] border border-[var(--os-hair-2)] p-1.5 text-[var(--os-muted)] hover:text-[var(--os-ink)] lg:hidden">
        <Icon name="Menu" size={16} />
      </button>
      <div className="mono flex min-w-0 items-center gap-2 text-[12px] text-[var(--os-faint)]">
        <span className="hidden sm:inline">os</span><Icon name="ChevronRight" size={12} className="hidden sm:inline" />
        <span className="truncate text-[var(--os-muted)]">{item?.label ?? "—"}</span>
        {item && <span className="hidden sm:inline"><StatusBadge status={item.status} /></span>}
      </div>
      <div className="flex-1" />
      <button onClick={onCmd} aria-label="Search and commands"
        className="mono flex items-center gap-2 rounded-[9px] border border-[var(--os-hair-2)] bg-[var(--os-panel)] px-3 py-1.5 text-[12px] text-[var(--os-muted)] transition-colors hover:border-[var(--os-faint)] hover:text-[var(--os-ink)]">
        <Icon name="Search" size={13} /> <span className="hidden sm:inline">Search &amp; commands</span>
        <kbd className="ml-1 hidden rounded border border-[var(--os-hair-2)] px-1 text-[10px] sm:inline">⌘K</kbd>
      </button>
      <ModeToggle />
      <ThemeToggle />
      <ConnBadge />
    </header>
  );
}

/** Mobile workspace drawer — without this the app is a dead end below lg. */
function MobileNav({ open, onClose }: { open: boolean; onClose: () => void }) {
  const path = usePathname();
  const active = (href: string) => (href === "/" ? path === "/" : path.startsWith(href));
  return (
    <AnimatePresence>
      {open && (
        <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
          className="fixed inset-0 z-50 bg-black/60 lg:hidden" onClick={onClose}>
          <motion.nav aria-label="Workspaces"
            initial={{ x: -300 }} animate={{ x: 0 }} exit={{ x: -300 }}
            transition={{ type: "spring", stiffness: 380, damping: 34 }}
            onClick={(e) => e.stopPropagation()}
            className="h-full w-[276px] overflow-y-auto border-r border-[var(--os-hair)] bg-[var(--os-ground-2)] p-3">
            <div className="mb-3 flex items-center justify-between border-b border-[var(--os-hair)] pb-3">
              <Logo />
              <button onClick={onClose} aria-label="Close navigation" className="rounded-md p-1.5 text-[var(--os-muted)]">
                <Icon name="X" size={16} />
              </button>
            </div>
            {NAV.map((group) => (
              <div key={group.group} className="mb-4">
                <div className="mono px-2.5 pb-1.5 text-[9.5px] uppercase tracking-[.18em] text-[var(--os-faint)]">{group.group}</div>
                {group.items.map((it) => (
                  <Link key={it.href} href={it.href} onClick={onClose}
                    className={`flex items-center gap-2.5 rounded-[9px] px-2.5 py-2.5 text-[13.5px] ${active(it.href) ? "bg-[var(--os-accent-soft)] text-[var(--os-ink)]" : "text-[var(--os-muted)]"}`}>
                    <Icon name={it.icon} size={16} strokeWidth={1.9} className={active(it.href) ? "text-[var(--os-accent)]" : ""} />
                    <span className="flex-1 truncate">{it.label}</span>
                    <StatusDot status={it.status} />
                  </Link>
                ))}
              </div>
            ))}
          </motion.nav>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

function StatusBar() {
  return (
    <div className="mono flex flex-wrap items-center gap-x-5 gap-y-1 border-t border-[var(--os-hair)] bg-[var(--os-ground-2)] px-4 py-2 text-[10.5px] text-[var(--os-faint)] md:px-6">
      <span className="flex items-center gap-1.5"><StatusDot status={dataAvailable ? "real" : "planned"} />{dataAvailable ? "recorded data loaded" : "fixture data"}</span>
      {dataAvailable && <span>{summary.totals.runsAll.toLocaleString()} experiments · {summary.totals.instances} instances · {summary.totals.operators} operators</span>}
      <span className="ml-auto">deterministic · bit-identical replay · truth over optimism</span>
    </div>
  );
}

function CommandPalette({ onClose }: { onClose: () => void }) {
  const router = useRouter();
  const [q, setQ] = useState("");
  const [hi, setHi] = useState(0);
  const results = useMemo(() => {
    const s = q.trim().toLowerCase();
    const base = ALL_ITEMS.map((i) => ({ kind: "Workspace", label: i.label, href: i.href, status: i.status, icon: i.icon }));
    if (!s) return base;
    return base.filter((i) => i.label.toLowerCase().includes(s)).slice(0, 10);
  }, [q]);
  const go = (r?: { href: string }) => { const t = r ?? results[hi]; if (t) { router.push(t.href); onClose(); } };
  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="fixed inset-0 z-50 flex items-start justify-center bg-black/50 p-4 pt-[14vh]" onClick={onClose}>
      <motion.div initial={{ opacity: 0, y: 10, scale: 0.99 }} animate={{ opacity: 1, y: 0, scale: 1 }} exit={{ opacity: 0, y: 6 }} transition={{ duration: 0.18 }}
        onClick={(e) => e.stopPropagation()} className="os-glass w-full max-w-[560px] overflow-hidden rounded-[14px]">
        <div className="flex items-center gap-3 border-b border-[var(--os-hair)] px-4 py-3">
          <Icon name="Search" size={16} className="text-[var(--os-faint)]" />
          <input autoFocus value={q} onChange={(e) => { setQ(e.target.value); setHi(0); }}
            onKeyDown={(e) => { if (e.key === "ArrowDown") { e.preventDefault(); setHi((i) => Math.min(i + 1, results.length - 1)); } if (e.key === "ArrowUp") { e.preventDefault(); setHi((i) => Math.max(i - 1, 0)); } if (e.key === "Enter") { e.preventDefault(); go(); } }}
            placeholder="Go to workspace…" className="w-full bg-transparent text-[14px] text-[var(--os-ink)] placeholder:text-[var(--os-faint)] focus:outline-none" />
          <kbd className="mono rounded border border-[var(--os-hair-2)] px-1.5 py-0.5 text-[10px] text-[var(--os-faint)]">esc</kbd>
        </div>
        <div className="max-h-[46vh] overflow-y-auto p-1.5">
          {results.map((r, i) => (
            <button key={r.href} onMouseEnter={() => setHi(i)} onClick={() => go(r)}
              className={`flex w-full items-center gap-3 rounded-[9px] px-3 py-2.5 text-left ${i === hi ? "bg-[var(--os-accent-soft)]" : "hover:bg-[var(--os-raised)]"}`}>
              <Icon name={r.icon} size={15} className="text-[var(--os-muted)]" />
              <span className="flex-1 text-[13.5px] text-[var(--os-ink)]">{r.label}</span>
              <StatusBadge status={r.status} />
            </button>
          ))}
        </div>
      </motion.div>
    </motion.div>
  );
}

export default function Shell({ children }: { children: React.ReactNode }) {
  const [cmd, setCmd] = useState(false);
  const [menu, setMenu] = useState(false);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") { e.preventDefault(); setCmd((v) => !v); }
      if (e.key === "Escape") setCmd(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  return (
    <div className="min-h-screen">
      <Rail />
      <MobileNav open={menu} onClose={() => setMenu(false)} />
      <div className="flex min-h-screen flex-col lg:pl-[248px]">
        <TopBar onCmd={() => setCmd(true)} onMenu={() => setMenu(true)} />
        <main id="workspace" className="os-grid flex-1">{children}</main>
        <StatusBar />
      </div>
      <AnimatePresence>{cmd && <CommandPalette key="cmd" onClose={() => setCmd(false)} />}</AnimatePresence>
    </div>
  );
}
