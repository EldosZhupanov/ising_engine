"use client";

/**
 * Appearance engine. Themes are handcrafted token sets in globals.css; this only
 * decides which one is active and persists the choice.
 *
 * "auto" follows the OS via prefers-color-scheme and keeps following it live.
 * A blocking inline script (see `themeScript`) applies the stored choice before
 * first paint so there is no flash of the wrong theme.
 */

import { useCallback, useEffect, useState, useSyncExternalStore } from "react";
import { Icon } from "./kit";

export type ThemeChoice = "auto" | "dark" | "light" | "contrast" | "presentation";

export const THEMES: { id: ThemeChoice; label: string; hint: string; icon: string }[] = [
  { id: "auto", label: "Auto", hint: "follow the operating system", icon: "MonitorSmartphone" },
  { id: "dark", label: "Dark Laboratory", hint: "the default instrument", icon: "Moon" },
  { id: "light", label: "Light Laboratory", hint: "warm paper, ink-dark type", icon: "Sun" },
  { id: "contrast", label: "High Contrast", hint: "maximum legibility", icon: "Contrast" },
  { id: "presentation", label: "Presentation", hint: "projector: larger, calmer", icon: "Projector" },
];

const KEY = "ising.theme";

/** Runs before paint in <head>; keep it tiny and dependency-free. */
export const themeScript = `(function(){try{var c=localStorage.getItem('${KEY}')||'auto';var m=window.matchMedia('(prefers-color-scheme: light)').matches;var t=c==='auto'?(m?'light':'dark'):c;document.documentElement.setAttribute('data-theme',t);document.documentElement.setAttribute('data-theme-choice',c);}catch(e){document.documentElement.setAttribute('data-theme','dark');}})();`;

function resolve(choice: ThemeChoice): string {
  if (choice !== "auto") return choice;
  if (typeof window === "undefined") return "dark";
  return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

/* ---- external store: localStorage + the OS colour-scheme media query ----
   Read via useSyncExternalStore so there is no setState-in-effect and no
   hydration mismatch. */
const listeners = new Set<() => void>();
const emit = () => listeners.forEach((l) => l());

function subscribe(cb: () => void) {
  listeners.add(cb);
  const mq = window.matchMedia("(prefers-color-scheme: light)");
  mq.addEventListener("change", cb);
  window.addEventListener("storage", cb); // other tabs
  return () => {
    listeners.delete(cb);
    mq.removeEventListener("change", cb);
    window.removeEventListener("storage", cb);
  };
}
function getChoice(): ThemeChoice {
  try { return (window.localStorage.getItem(KEY) as ThemeChoice) || "auto"; } catch { return "auto"; }
}
const getServerChoice = (): ThemeChoice => "auto";

export function useTheme() {
  const choice = useSyncExternalStore(subscribe, getChoice, getServerChoice);

  // React hydration discards attributes the server did not render, which strips
  // what the pre-paint inline script set. Re-apply after mount, and on change.
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", resolve(choice));
    document.documentElement.setAttribute("data-theme-choice", choice);
  }, [choice]);

  const setTheme = useCallback((c: ThemeChoice) => {
    try { window.localStorage.setItem(KEY, c); } catch { /* private mode */ }
    document.documentElement.setAttribute("data-theme", resolve(c));
    document.documentElement.setAttribute("data-theme-choice", c);
    emit();
  }, []);

  return { choice, setTheme };
}

/** Compact cycling control for the top bar. */
export function ThemeToggle() {
  const { choice, setTheme } = useTheme();
  const [open, setOpen] = useState(false);
  const cur = THEMES.find((t) => t.id === choice) ?? THEMES[0];
  return (
    <div className="relative">
      <button
        onClick={() => setOpen((v) => !v)}
        aria-label={`Appearance: ${cur.label}`}
        aria-expanded={open}
        className="mono flex items-center gap-1.5 rounded-[9px] border border-[var(--os-hair-2)] bg-[var(--os-panel)] px-2.5 py-1.5 text-[11.5px] text-[var(--os-muted)] transition-colors hover:text-[var(--os-ink)]"
      >
        <Icon name={cur.icon} size={13} />
        <span className="hidden lg:inline">{cur.label.replace(" Laboratory", "")}</span>
      </button>
      {open && (
        <>
          <button className="fixed inset-0 z-40 cursor-default" aria-hidden tabIndex={-1} onClick={() => setOpen(false)} />
          <div role="menu" className="os-glass absolute right-0 top-[calc(100%+8px)] z-50 w-[248px] overflow-hidden rounded-[12px] p-1.5">
            {THEMES.map((t) => (
              <button key={t.id} role="menuitemradio" aria-checked={choice === t.id}
                onClick={() => { setTheme(t.id); setOpen(false); }}
                className={`flex w-full items-center gap-2.5 rounded-[9px] px-2.5 py-2 text-left transition-colors ${choice === t.id ? "bg-[var(--os-accent-soft)]" : "hover:bg-[var(--os-raised)]"}`}>
                <Icon name={t.icon} size={14} className={choice === t.id ? "text-[var(--os-accent)]" : "text-[var(--os-muted)]"} />
                <span className="flex-1">
                  <span className="block text-[12.5px] text-[var(--os-ink)]">{t.label}</span>
                  <span className="mono block text-[10px] text-[var(--os-faint)]">{t.hint}</span>
                </span>
                {choice === t.id && <Icon name="Check" size={13} className="text-[var(--os-accent)]" />}
              </button>
            ))}
            <div className="mono border-t border-[var(--os-hair)] px-2.5 pt-2 pb-1 text-[9.5px] leading-[1.5] text-[var(--os-faint)]">
              Print uses a dedicated ink-on-paper theme automatically.
            </div>
          </div>
        </>
      )}
    </div>
  );
}
