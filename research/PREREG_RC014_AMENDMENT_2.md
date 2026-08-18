# PREREG — RC-014 Amendment 2: corrections before D-14 is run

**Status:** binding amendment to `PREREG_RC014.md` and Amendment 1, written and
committed **before D-14 executes**. It corrects one overclaim in the Gate-A
record, sharpens one non-gating prediction, and records a corpus limitation that
D-14b's instance clause cannot satisfy as written.

**It does not change D-14.** D-14a and D-14b run exactly as pre-registered; only
their instance *resolution* is affected by a fact about the benchmark corpus
(A2.3), and that resolution follows the fallback clause already in the text.

**Reason for amendment:** an independent review found (a) the Peskun citation in
`RC014_COUNTERFACTUAL_SUBSTITUTION.md` §4 claims more than the theorem covers,
(b) the `T → 0` mechanism prediction is ambiguous without a tie caveat, and
(c) D-14 as written discriminates on two instances, which can measure generator
properties rather than regimes.

---

## A2.1 — The Peskun claim is weakened; the direction is re-labelled

`RC014_COUNTERFACTUAL_SUBSTITUTION.md` §4 states that Peskun ordering "predicts
Metropolis dominates heat-bath for the same target", and uses that to label the
observed direction **KNOWN**. That is more than the theorem gives.

**What Peskun ordering actually covers:** asymptotic efficiency (variance of
ergodic averages) of *reversible* kernels sharing a *common stationary
distribution*, in the *long-run* limit.

**What this cycle measures:** a *finite-budget* (16 sweeps) *best-of-32-replicas*
energy, followed by `greedy_descent`, on a *ladder* spanning `T = 4.0 → 0.1`. That
observable is not an ergodic average, the chains are not run to stationarity, and
the min-over-replicas functional is not what Peskun bounds.

**Corrected statement, binding from here on:**

> Peskun ordering explains the *local* move-acceptance advantage — Metropolis
> accepts every `ΔE ≤ 0` move with probability 1, heat-bath with
> `1/(1+exp(−|h|/T)) < 1` — and is **consistent with** the observed direction. It
> is **not** a theorem about the finite-budget best-of-ensemble objective `Y`
> measured here, and must not be cited as predicting it.

**Novelty re-label, on the project's own four-level scale
(`RESEARCH_GAPS.md` §5c):** the direction moves from **KNOWN** to
**LIKELY KNOWN** — finite-budget Metropolis-vs-heat-bath comparisons are common
in the annealing literature and we have not located the exact citation, but no
theorem we can name covers this observable. This *raises* the residual novelty of
the direction slightly; the cycle's stated contribution is unchanged and remains
the **methodological** one (deletion cannot separate two operators of one
capability; substitution can).

---

## A2.2 — The `T → 0` prediction is sharpened, and tie mass becomes a required observable

The non-gating mechanism prediction in `PREREG_RC014.md` §12 ("the Metropolis
advantage must shrink as `T → 0`") is ambiguous, because the two kernels do
**not** converge at `ΔE = 0`:

| | `ΔE < 0` | `ΔE = 0` | `ΔE > 0` |
|---|---|---|---|
| Metropolis (as implemented, `metropolis_sweep.rs:117`) | accept w.p. 1 | **accept w.p. 1** (the `d <= 0.0` branch) | `exp(−ΔE/T) → 0` |
| heat-bath | `1/(1+exp(−\|h\|/T)) → 1` | **flip w.p. 1/2** | `→ 0` |

At zero local field the kernels differ by a **constant** `1/2` versus `1`,
independent of temperature. So cooling closes the gap only where `|h| > 0`.

**Corrected prediction:**

> The Metropolis advantage must shrink as `T → 0` **provided the mass of `ΔE = 0`
> proposals is small**. The tie fraction is therefore a **required co-observable**:
> a flat or growing advantage at low `T` is only interpretable alongside it.

**Additional exploratory check, declared now** (correlational, non-gating): the
tie fraction is measured on all six Gate-A instances and compared against the
already-recorded effect sizes. The two toroidal ±1 lattices (G11, G32, degree 4)
carry the *largest* Gate-A effects (`rel` 0.87–2.41% against 0.17–0.47%
elsewhere) and are the instances where integer fields most easily vanish. If tie
mass tracks effect size across the six, tie-handling — not the general downhill
gap — is the candidate mechanism. This is a hypothesis-generating comparison at
n = 6 and is reported as such, never as a test.

---

## A2.3 — Corpus limitation: no weighted BiqMac instance is sparse enough

`PREREG_RC014.md` §12 resolves D-14b's instance as "`biqmac/gka1a` **or** the
sparsest available weighted BiqMac instance with density ≤ 0.06".

**Measured over all 125 BiqMac instances** (`n`, `m` from each file header;
"weighted" = more than two distinct edge weights):

- `gka1a` has density **0.127**, not ≤ 0.06;
- **every** BiqMac instance is weighted, and the **sparsest is `gka8a`
  (n = 100, m = 403, density 0.0814, 166 distinct weights)**;
- **zero** weighted instances have density ≤ 0.06.

The named instance fails its own clause and the fallback set is **empty**.

**Resolution, under the fallback's intent rather than its letter:** D-14b uses
**`gka8a.sparse` (density 0.0814)** — the sparsest weighted instance that exists
locally — and the deviation is recorded here. D-14b's *prediction* is unchanged.

**Why the test still discriminates.** `gka8a` at 0.0814 sits adjacent to the
G-Set band (0.002–0.06) and far from `be100.1` (0.991). If the sign follows
density, `gka8a` gives `I > 0` like G-Set; if it follows weighting, `gka8a` gives
`I < 0` like `be100.1`. The two hypotheses still predict opposite signs.

**Why it is weaker than intended.** The weighted density axis is bounded below at
≈ 0.08 in this corpus while G-Set spans 0.002–0.06, so the two regimes' density
ranges **do not overlap**. A clean weighted-sparse point is not obtainable from
the available benchmarks — a **third** benchmark-availability finding, after
RC-004's two (guide axis unidentifiable; frustration axis confounded). Recorded
in `memory/OPEN_PROBLEMS.md` §1 territory.

---

## A2.4 — A matched set follows D-14; D-14 itself is not extended

D-14 as written discriminates on **two instances**, one per regime, which can
measure a generator's or an instance's idiosyncrasies rather than a regime.

**Declared now, so it is not a post-hoc extension:** after D-14 runs *unchanged*,
a matched set of **≥ 4 realisations per regime cell** is run over the 2×2 design
(weighted × dense), with the regime-cell mean as the unit of analysis and the
same materiality and replication rules. Any synthetic instance used there is
labelled **synthetic and exploratory**; it is not folded into D-14's verdict, and
D-14's own result stands or falls on its two pre-registered points.

If the matched set contradicts D-14, **both** are reported, and D-14's two-point
result is the one that carries the pre-registration.

---

## A2.5 — Gate B must compare both estimands

Gate B condition 3 is made explicit: the effect must be reported under **both**

- `I_replace_work` — equal sweeps and equal draws (the Gate-A estimand), and
- `I_replace_cost` — integer sweep counts chosen from the frozen interleaved
  calibration table (Amendment 1 A1),

on the Gate-B instances. Equal work is **not** evidence of preference under an
equal wall-clock budget: the two kernels differ in instruction mix (heat-bath
evaluates its exponential unconditionally; Metropolis only on the uphill branch).
A Gate-A result that reverses or vanishes under measured equal cost is a
scheduler-relevant negative and is reported as such.

---

## A2.6 — Nothing else changes

No Runtime change, no `theory.rs` semantic change, no new published fact type, no
Foundry or DSL work, no dataset growth. The published deletion facts stay exactly
as they are until Gate B decides.
