# Ising Engine Laboratory — Design Reference Analysis

> Written **before** implementation. The goal is not to copy any of these, but to
> extract a coherent design language for a *laboratory instrument*, not a SaaS
> landing page. The organizing reference is the supplied image
> (`reference/…png`): a warm-white workspace, a floating glass left rail, a huge
> glowing search field, a soft-lit 3D object with an orange rim, faint grid +
> crosshairs, enormous empty space. Everything below is judged against that.

## North star

**Apple × DeepMind × CERN × OpenAI Research × Figma Dev Mode** — a control room a
scientist *uses*. Warm white, glass, soft shadows, one restrained orange accent,
a whisper of scientific blue. No neon, no gradients-as-decoration, no SaaS hero.
Motion is slow and expensive. Every number on screen is real (the append-only
record: 18,570 runs, 3 backends, 14 operators, 100% bit-identical replay).

---

## The 24 references

### 1. Apple — hardware & pro-app pages
- **Adopt:** reverence for empty space; a single hero object under studio
  lighting; type that carries the page; restraint as luxury.
- **Reject:** consumer marketing scroll-jacking; oversized emotional copy.
- **Translate:** the hero object becomes the **Ising lattice / knowledge graph**
  under soft light, not a phone. Copy is instrument labels, not slogans.

### 2. DeepMind — research & AlphaFold
- **Adopt:** science communicated with beauty; structured, evidence-led sections;
  serene palette; the sense of a *system* behind the page.
- **Reject:** heavy hero video; long-form article density.
- **Translate:** each subsystem (Theory Engine, Knowledge Graph) gets a calm,
  diagram-led "instrument panel" instead of a marketing card.

### 3. CERN — LHC / experiment dashboards
- **Adopt:** the aesthetic of *real instruments* — monospace readouts, run IDs,
  provenance, status strips; the dignity of raw measured data.
- **Reject:** dated institutional chrome; cluttered legacy tables.
- **Translate:** a persistent **system status strip** (runs recorded, replay
  determinism, backend throughput) rendered like a beamline monitor.

### 4. OpenAI Research — index & model cards
- **Adopt:** quiet authority; whitespace; small monospace metadata; "this is a
  lab, not a store."
- **Reject:** minimal to the point of coldness.
- **Translate:** section eyebrows in mono (`03 · KNOWLEDGE GRAPH`), warm ground so
  it reads as a workspace, not a void.

### 5. Figma — Dev Mode
- **Adopt:** the *inspector* metaphor — a left rail of tools, a live canvas, a
  right panel of properties; the feeling of software with real state.
- **Reject:** productivity-tool busyness.
- **Translate:** the whole site is an inspector: **left lab-nav → center
  workspace → contextual readouts.** Nav switches the workspace view.

### 6. Linear
- **Adopt:** crispness; perfect hairlines; keyboard-grade focus states; fast,
  tiny, confident transitions; the mono/sans duet.
- **Reject:** its dark-first identity; purple gradient.
- **Translate:** hairline `#E7E3DC` dividers on warm white; 120–180ms eases;
  `⌘K`-style universal search affordance.

### 7. Raycast
- **Adopt:** the **command palette** as a first-class surface; result rows with
  type badges; glassy floating panel.
- **Reject:** launcher-app density.
- **Translate:** the huge hero search becomes **Universal Scientific Search** —
  one field over concepts, experiments, theories, graph nodes, memory.

### 8. Arc Browser
- **Adopt:** playful-but-premium floating sidebar; soft depth; rounded glass;
  delightful micro-motion on hover.
- **Reject:** whimsy/color that would undercut scientific seriousness.
- **Translate:** the left rail floats with a soft shadow and 16px radius, exactly
  like the reference; hovers lift 1–2px, no bounce.

### 9. Vercel
- **Adopt:** geometric precision; dot/grid backdrops; monochrome discipline.
- **Reject:** black SaaS hero; triangle-brand energy.
- **Translate:** faint **grid + crosshair "+" markers** from the reference as the
  workspace backdrop, in warm gray at ~4% opacity.

### 10. Stripe
- **Adopt:** information density done elegantly; layered subtle gradients for
  depth; immaculate alignment.
- **Reject:** the gradient hero cliché; fintech gloss.
- **Translate:** depth via soft shadows + glass, never via saturated gradient.

### 11. Bloomberg Terminal
- **Adopt:** unapologetic data density; mono numerics; the authority of a
  professional instrument packed with live readouts.
- **Reject:** its hostile ergonomics and black CRT palette.
- **Translate:** dense but *calm* readout panels — tabular-nums, aligned columns,
  a warm-white terminal rather than a black one.

### 12. Observable / D3
- **Adopt:** data viz as a first-class citizen; live, inspectable graphics.
- **Reject:** notebook clutter.
- **Translate:** the Knowledge Graph and Concept Evolution render as **live SVG/
  canvas graphs**, not screenshots.

### 13. Rerun.io (robotics viz)
- **Adopt:** the "engineering instrument" look — timelines, entity trees, spatial
  views; monochrome + one accent.
- **Reject:** heaviness.
- **Translate:** the Experiment/Campaign view is a **timeline of runs** with a
  provenance rail.

### 14. Weights & Biases
- **Adopt:** experiment tracking as identity; run tables; metric sparklines.
- **Reject:** dashboard sprawl and chart-junk.
- **Translate:** a restrained **run ledger** with sparkline energy traces, honest
  labels ("append-only record", not "live").

### 15. Hugging Face
- **Adopt:** the model/dataset "card" as a unit; monospace tags; open-research
  warmth.
- **Reject:** community-feed busyness.
- **Translate:** each operator/subsystem gets a **passport card** (its real
  capability metadata), echoing the engine's capability passports.

### 16. Palantir Foundry
- **Adopt:** the ontology/graph-as-product feel; serious, dark-instrument gravity;
  linked data.
- **Reject:** opaque enterprise coldness; dark theme.
- **Translate:** the Knowledge Graph is the centerpiece "ontology," but rendered
  in warm light, legible and inviting.

### 17. Teenage Engineering
- **Adopt:** industrial-design joy; precise grids; one hero accent color; objects
  that look *engineered*.
- **Reject:** playful color explosions.
- **Translate:** the single **orange accent** (from the reference) used like a
  hardware power LED — sparingly, meaningfully (active state, primary action).

### 18. Braun / Dieter Rams
- **Adopt:** "less but better"; honest materials; function-first typography; the
  10 principles as a filter.
- **Reject:** nothing — this is the discipline.
- **Translate:** every element must justify itself; no decoration that isn't
  information. Matches the project's "truth over optimism" ethos.

### 19. NASA JPL "Eyes on the Solar System"
- **Adopt:** real telemetry made cinematic; a 3D object you can gently orbit;
  authoritative HUD readouts.
- **Reject:** game-like camera.
- **Translate:** the hero lattice responds to the mouse with **parallax + slow
  drift**, never a game orbit; HUD-style corner readouts.

### 20. Ableton Live
- **Adopt:** dense professional tool that still feels calm; timeline + device
  rack metaphor; confident use of one accent.
- **Reject:** its saturated theme.
- **Translate:** the Autonomous Loop reads like a **signal chain / device rack** —
  stages as linked modules with flow between them.

### 21. Superhuman
- **Adopt:** speed as a feeling; keyboard-first; premium restraint; the sense of
  an instrument for experts.
- **Reject:** email-app specifics.
- **Translate:** snappy view-switching between lab sections; `⌘K` search; focus
  rings that feel keyboard-grade.

### 22. Notion
- **Adopt:** calm document surfaces; soft neutral palette; content that breathes.
- **Reject:** blocky editor chrome.
- **Translate:** the Documentation / Architecture views are calm, readable
  reading surfaces inside the lab shell.

### 23. Awwwards "Sites of the Day" (general canon)
- **Adopt:** bespoke motion; a signature hero moment; cohesive art direction end
  to end; the 1% polish (cursor, easing, load choreography).
- **Reject:** motion-for-its-own-sake; heavy WebGL that tanks Lighthouse.
- **Translate:** one **signature hero** (the living lattice) + disciplined,
  reduced-motion-safe micro-interactions everywhere else. Canvas over heavy WebGL
  to protect Lighthouse >95.

### 24. The supplied reference image (primary)
- **Adopt (verbatim language):** warm-white ground; floating glass left rail with
  logo + minimal line icons + bottom orange circular `→`; enormous empty
  workspace; faint grid + crosshair marks; huge glass search bar with an **orange
  glow border**; a soft-lit hero object with an orange rim and a subtle contact
  shadow.
- **Reject (explicitly):** the **globe** and the **fiber-optic cable** — they are
  generic tech stock. Replace with Ising Engine visualizations.
- **Translate:** globe → **living knowledge-graph / Ising lattice**; fiber cable →
  gone (or a faint particle field of "experiments" drifting in). Keep the exact
  rail, search, grid, lighting, and accent discipline.

---

## Synthesized design language (the filter for every commit)

**Palette (light laboratory)**
- Ground `#F4F1EC` warm paper · panel `#FBFAF7` · glass `rgba(255,255,255,.6)`
- Hairline `#E7E3DC` · ink `#1A1714` · muted `#6B655C` · faint `#A49E93`
- Accent orange `#E8792B` (the one LED) · scientific blue `#3E6FF3` (systems only)
- Semantics: confirmed `#2F9E67` · refuted `#D0533B` · open `#3E6FF3`

**Type:** Geist (display + UI) / Geist Mono (all data, IDs, labels, readouts).
Big quiet headlines; mono eyebrows `NN · SECTION`.

**Space & grid:** 8-pt system; generous margins; faint 32px grid + crosshairs at
~4%; content breathes like the reference.

**Material:** glass panels (blur + 60% white), 14–18px radius, soft layered
shadows (`0 1px 2px`, `0 20px 40px -24px`). Depth from light, never from color.

**Motion:** slow, expensive eases `[0.16,1,0.3,1]`, 400–900ms for reveals,
120–180ms for hovers; magnetic primary buttons; parallax hero; **everything
gated by `useReducedMotion`.** Canvas/SVG for the living systems; no heavy WebGL.

**Structure = inspector:** floating glass **lab-nav** (Overview, Research,
Experiments, Theory Engine, Knowledge Graph, Memory, Campaigns, Predictor,
Runtime, Feature Registry, Concept Evolution, Documentation) → **center
workspace** that swaps per selection → **universal scientific search** on top →
persistent **system status strip**. The website *is* the laboratory.

**Honesty rule (from the repo's constitution):** animations are design; **data is
real**. Illustrative visuals are labeled as schematic; measured numbers come from
the append-only record. Never imply a live engine stream that isn't there.
