"use client";

/**
 * Mission progress + operating mode.
 *
 * Progress is earned by performing real actions, not by clicking "next": each
 * workspace calls `recordAction(...)` when the scientist actually does the thing.
 * State lives in localStorage and is read through useSyncExternalStore, so there
 * is no setState-in-effect and no hydration mismatch (see Theme.tsx for the same
 * pattern and the reason).
 */

import { useCallback, useSyncExternalStore } from "react";
import { MISSIONS, type ActionEvent, type Mission } from "./curriculum";

const KEY_DONE = "ising.progress.v1";
const KEY_MODE = "ising.mode";

export type Mode = "beginner" | "professional";

type State = { actions: string[]; manual: string[]; visited: string[] };
const EMPTY: State = { actions: [], manual: [], visited: [] };

const listeners = new Set<() => void>();
const emit = () => listeners.forEach((l) => l());

function subscribe(cb: () => void) {
  listeners.add(cb);
  if (typeof window !== "undefined") window.addEventListener("storage", cb);
  return () => {
    listeners.delete(cb);
    if (typeof window !== "undefined") window.removeEventListener("storage", cb);
  };
}

function read(): State {
  try {
    const raw = window.localStorage.getItem(KEY_DONE);
    if (!raw) return EMPTY;
    const p = JSON.parse(raw) as Partial<State>;
    return { actions: p.actions ?? [], manual: p.manual ?? [], visited: p.visited ?? [] };
  } catch { return EMPTY; }
}
function write(s: State) {
  try { window.localStorage.setItem(KEY_DONE, JSON.stringify(s)); } catch { /* private mode */ }
  emit();
}

/* ---- snapshot caching: useSyncExternalStore requires a stable reference ---- */
let cachedRaw: string | null = null;
let cachedState: State = EMPTY;
function getSnapshot(): State {
  try {
    const raw = window.localStorage.getItem(KEY_DONE);
    if (raw !== cachedRaw) { cachedRaw = raw; cachedState = read(); }
    return cachedState;
  } catch { return EMPTY; }
}
const getServerSnapshot = (): State => EMPTY;

/** Called by workspaces when a real scientific action happens. */
export function recordAction(event: ActionEvent) {
  if (typeof window === "undefined") return;
  const s = read();
  if (s.actions.includes(event)) return;
  write({ ...s, actions: [...s.actions, event] });
}

export function recordVisit(route: string) {
  if (typeof window === "undefined") return;
  const s = read();
  if (s.visited.includes(route)) return;
  write({ ...s, visited: [...s.visited, route] });
}

export function isComplete(m: Mission, s: State): boolean {
  switch (m.verify.kind) {
    case "action": return s.actions.includes(m.verify.event);
    case "visited": return s.visited.includes(m.verify.route);
    case "manual": return s.manual.includes(m.id);
  }
}

export function useProgress() {
  const state = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);

  const done = useCallback((m: Mission) => isComplete(m, state), [state]);
  const toggleManual = useCallback((id: string) => {
    const s = read();
    const has = s.manual.includes(id);
    write({ ...s, manual: has ? s.manual.filter((x) => x !== id) : [...s.manual, id] });
  }, []);
  const reset = useCallback(() => write(EMPTY), []);

  const completed = MISSIONS.filter((m) => isComplete(m, state));
  const available = MISSIONS.filter((m) => m.status !== "offline");
  /** The next mission a scientist should take: earliest stage, not yet done, not blocked. */
  const nextMission = available.find((m) => !isComplete(m, state)) ?? null;

  return {
    state, done, toggleManual, reset,
    completedCount: completed.length,
    totalCount: MISSIONS.length,
    availableCount: available.length,
    availableCompleted: available.filter((m) => isComplete(m, state)).length,
    nextMission,
    stageProgress: (n: number) => {
      const ms = MISSIONS.filter((m) => m.stage === n);
      return { done: ms.filter((m) => isComplete(m, state)).length, total: ms.length };
    },
  };
}

/* ---------------- operating mode (progressive disclosure) ---------------- */
const modeListeners = new Set<() => void>();
const emitMode = () => modeListeners.forEach((l) => l());
function subscribeMode(cb: () => void) {
  modeListeners.add(cb);
  if (typeof window !== "undefined") window.addEventListener("storage", cb);
  return () => {
    modeListeners.delete(cb);
    if (typeof window !== "undefined") window.removeEventListener("storage", cb);
  };
}
function getMode(): Mode {
  try { return (window.localStorage.getItem(KEY_MODE) as Mode) || "beginner"; } catch { return "beginner"; }
}
const getServerMode = (): Mode => "beginner";

export function useMode() {
  const mode = useSyncExternalStore(subscribeMode, getMode, getServerMode);
  const setMode = useCallback((m: Mode) => {
    try { window.localStorage.setItem(KEY_MODE, m); } catch { /* ignore */ }
    emitMode();
  }, []);
  return { mode, setMode, isBeginner: mode === "beginner" };
}
