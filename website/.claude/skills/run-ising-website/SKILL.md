---
name: run-ising-website
description: Build, run, and screenshot the Ising Engine Scientific OS (Next.js 16 app in website/) and its Rust control plane. Use to launch it, preview it headlessly, verify a change renders, replay a recorded experiment, or capture full-page screenshots.
---

# Run the Ising Engine website

The site is a **Next.js 16 (Turbopack) + React 19 + Tailwind v4 + motion/react**
app — the *Scientific Operating System* for the Ising Engine platform. It is a
**26-route workspace shell** (`src/app/(app)/*`) over a dark instrument design
system, rendering the platform's REAL append-only record (ingested at build time
by `scripts/ingest.mjs` — run automatically as `prebuild`/`predev`).

Key surfaces: Mission Control (`/`), Research Path (`/start`), Discovery Center
(`/discover`), Experiment Designer (`/design`), Publication (`/publish`),
Applications (`/applications`), Playbooks (`/playbooks`), Experiments, Knowledge
Graph, Benchmarks, Evaluation, Dataset, Papers, and the **LLM Control Center**
(`/llm`, live against Ollama).

Everything is driven headlessly by Playwright harnesses in this skill directory —
that is the agent path. Use them to confirm a change works:

| harness | verifies |
|---|---|
| `driver.mjs` | Mission Control markers + screenshots 11 routes |
| `probe-llm.mjs` | LLM cockpit live against Ollama (load/unload, tok/s, streaming) |
| `probe-workflow.mjs` | curriculum, missions-from-real-actions, gaps, modes |
| `probe-chain.mjs` | workflow spine, Experiment Designer, Publication Composer |
| `probe-control.mjs` | control plane: telemetry, freshness, **Replay** |
| `probe-campaign.mjs` | START → STEP → RESUME → finish (needs a **writable** `--dir`) |
| `probe-machinery.mjs` | Operators, Models, Runtime, Features, Docs, Settings, Solve |
| `probe-memory.mjs` | Scientific Memory: live recall + retention report |
| `probe-degrade.mjs` | honest OFFLINE fallback when the plane dies |

**Honesty invariant to preserve:** no route may claim `status: "real"` in
`src/lib/nav.ts` while its page renders `components/os/Stub`. Exactly one stub
should remain: **zero**. Every nav item is `real`. Audit with:

```bash
python3 - <<'P'
import re, pathlib
nav = pathlib.Path("src/lib/nav.ts").read_text()
for label, href, status in re.findall(r'label: "([^"]+)", href: "([^"]+)", icon: "[^"]+", status: "([a-z]+)"', nav):
    f = pathlib.Path("src/app/(app)/page.tsx") if href == "/" else pathlib.Path(f"src/app/(app){href}/page.tsx")
    if status == "real" and "components/os/Stub" in f.read_text():
        print("CONTRADICTION", href)
P
```

**All paths below are relative to `website/`** (the unit dir). The driver lives
at `.claude/skills/run-ising-website/driver.mjs`.

## Prerequisites (one time)

Node 20+ and the project's deps (`node_modules/` — run `npm install` if absent).

The driver needs Playwright's Chromium. Install the browser binary:

```bash
npx playwright install chromium
```

Headless Chromium also needs a few system libs (`libnspr4`, `libnss3`,
`libasound2`). Two ways to get them:

```bash
# Preferred, if you have sudo:
sudo npx playwright install-deps chromium
```

```bash
# No-sudo fallback — downloads the .deb files and extracts them locally;
# driver.mjs auto-detects them (no LD_LIBRARY_PATH needed):
bash .claude/skills/run-ising-website/setup-libs.sh
```

## Build

```bash
npm run lint
npm run build
```

Both are clean. `build` prints one static route (`○ /`). Do the production build
before `npm run start` — `next start` serves the built output, it does not compile.

## Run (agent path) — build once, serve, drive, screenshot

Start the production server on port 3100 in the background:

```bash
nohup npm run start -- -p 3100 > /tmp/next-server.log 2>&1 &
```

Wait for it, then drive it:

```bash
for i in $(seq 1 20); do curl -sf -o /dev/null http://localhost:3100/ && break; sleep 1; done
node .claude/skills/run-ising-website/driver.mjs
```

Expected output — the run **fails loudly** (exit 1) if the server is down or any
section didn't render:

```
title: Ising Engine — Scientific Operating System
markers: 8/8 present
screenshot: …/shots/home.png
screenshot: …/shots/start.png        (+ discover, design, publish, applications,
                                      playbooks, experiments, graph, benchmarks,
                                      evaluation, dataset, papers, llm)
PASS
```

Screenshots land in `.claude/skills/run-ising-website/shots/` (full page, 1440px).
**Open them and actually look.** Mission Control should show real telemetry
(109,758 experiments · 23 instances · 14 operators), the animated research-loop
ring, real mined discoveries with support/confidence, and the confirmed/refuted
ledger. Run the other harnesses for the interactive surfaces.

Note: full-page screenshots render `position: fixed` chrome (the rail) at its
scroll offset, so the sidebar can appear mid-page. That is a Playwright artifact,
not a layout bug.

Point the driver at a different origin with an arg or `BASE_URL`:

```bash
node .claude/skills/run-ising-website/driver.mjs http://localhost:3000
```

Stop the server when done — kill by port so you don't accidentally match other
processes (or the invoking shell):

```bash
fuser -k 3100/tcp
```

## Run (human path)

`npm run dev` (Turbopack dev server on `http://localhost:3000`, hot reload) for
interactive editing. Useless headless — nothing screenshots it — so for
verifying a change programmatically use the driver above instead.

## Control plane (optional — unlocks the live surfaces)

The site works fully without it (build-time snapshot). Starting it makes three
surfaces genuinely live: host telemetry (CPU/RAM/GPU), data freshness
(snapshot → live), **Scientific Memory** (recall is computed, never stored, so
`/memory` cannot work without it), campaign lifecycle, and **Replay** (re-execute any recorded experiment from its
seed and verify it comes back bit-identical).

```bash
cd ~/ising_engine
cargo run --release --bin control_api          # binds 127.0.0.1:7878
```

Verify the integration (needs both servers up):

```bash
node .claude/skills/run-ising-website/probe-control.mjs
node .claude/skills/run-ising-website/probe-degrade.mjs   # kills it; checks honest fallback
```

**Build in release, always.** `.cargo/config.toml` forces a global
`-Copt-level=3`, so the *debug* profile mixes opt-level-3 with debuginfo and
`rust-lld` fails to link bins/tests with `undefined hidden symbol: anon.…llvm.…`.
That is pre-existing config, not broken code — the repo ships aliases
`cargo verify` (= `test --release`) and `cargo lint` (= `clippy --release -- -D warnings`).

## Gotchas

- **Scroll-reveal blanks a naive screenshot.** Every section below the hero uses
  motion/react `whileInView` / `useInView` (and count-up KPIs). A full-page
  screenshot captures from the top scroll position, so those elements render
  **blank or showing `0`** unless something scrolls them into view first. The
  driver steps down the whole page (tripping the `once: true` reveals), returns
  to the top, waits for the count-ups to settle, *then* screenshots. If you
  screenshot by other means, replicate that scroll or you'll get an empty page.
- **`next start` ≠ compile.** It serves the last `next build`. Forget to rebuild
  and you'll test stale output. `npm run dev` compiles on the fly; `start` doesn't.
- **Hero headline can overflow at ~1440px.** The `clamp()` hero type is large;
  the last word ("scientist.") clips at the right edge in the full-width shot.
  Cosmetic, not a failure — flagged so you don't chase it as a driver bug.
- **Replay needs the real operator registry.** `OperatorRegistry::new()` is
  EMPTY; `OperatorRegistry::standard()` has all 14. Using `new()` silently yields
  a non-finite score, which serialises to `null` and reads as "not identical".
- **`useReducedMotion`.** Components honor it; under a reduced-motion browser the
  count-ups jump straight to final values (still correct, no animation).

## Troubleshooting

- `error while loading shared libraries: libnspr4.so` (or `libnss3.so`,
  `libasound.so.2`) — Chromium libs missing. Run the sudo or no-sudo step under
  Prerequisites. The no-sudo `setup-libs.sh` extracts them locally and the driver
  finds them automatically.
- `FAIL: could not reach http://localhost:3100` — server isn't up. Check
  `/tmp/next-server.log`; confirm `curl http://localhost:3100/` returns 200.
- `FAIL: missing rendered markers: …` — the named section didn't render. A real
  regression: inspect that component and the build output.
- `Executable doesn't exist … ms-playwright` — run `npx playwright install chromium`.
