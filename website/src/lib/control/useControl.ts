"use client";

/**
 * Control-plane connection state — one shared poller for the whole app.
 *
 * Held in a module-level store and read through `useSyncExternalStore`, for the
 * same reason the theme and mode stores are: seeding `useState` from a store
 * freezes the SERVER snapshot at hydration, so later changes never apply. That
 * bug has bitten this codebase three times; do not reintroduce it.
 */

import { useCallback, useSyncExternalStore } from "react";
import {
  getHealth, loadControlUrl, saveControlUrl, DEFAULT_CONTROL_URL, type Health,
} from "./client";

export type ConnStatus = "checking" | "online" | "offline";

type State = { status: ConnStatus; url: string; health: Health | null; error: string | null; checkedAt: number };

let state: State = { status: "checking", url: DEFAULT_CONTROL_URL, health: null, error: null, checkedAt: 0 };
const SERVER_STATE: State = { status: "checking", url: DEFAULT_CONTROL_URL, health: null, error: null, checkedAt: 0 };

const listeners = new Set<() => void>();
const emit = () => listeners.forEach((l) => l());
const set = (p: Partial<State>) => { state = { ...state, ...p }; emit(); };

let poller: ReturnType<typeof setInterval> | null = null;
let started = false;

async function probe() {
  const url = state.url;
  try {
    const health = await getHealth(url);
    set({ status: "online", health, error: null, checkedAt: Date.now() });
  } catch (e) {
    set({
      status: "offline", health: null,
      error: e instanceof Error ? e.message : String(e),
      checkedAt: Date.now(),
    });
  }
}

function start() {
  if (started) return;
  started = true;
  state = { ...state, url: loadControlUrl() };
  void probe();
  // 15 s is frequent enough to notice the plane coming up, rare enough to be free.
  poller = setInterval(() => { void probe(); }, 15_000);
}

function subscribe(cb: () => void) {
  listeners.add(cb);
  start();
  return () => {
    listeners.delete(cb);
    if (listeners.size === 0 && poller) { clearInterval(poller); poller = null; started = false; }
  };
}

const getSnapshot = () => state;
const getServerSnapshot = () => SERVER_STATE;

export function useControl() {
  const s = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
  const setUrl = useCallback((u: string) => {
    saveControlUrl(u);
    set({ url: u, status: "checking" });
    void probe();
  }, []);
  const recheck = useCallback(() => { set({ status: "checking" }); void probe(); }, []);
  return { ...s, online: s.status === "online", setUrl, recheck };
}
