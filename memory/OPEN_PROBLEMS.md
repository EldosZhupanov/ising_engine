# memory/OPEN_PROBLEMS.md — pointer + findings that live nowhere else

**Authoritative sources**
- `../research/RESEARCH_GAPS.md` — research gaps
- `../ROADMAP.md` §"Technical debt / unfinished" — engineering debt

The two sections below are **not** in either source and must not be lost: they
invalidate entire classes of future experiment on the default benchmark.

---

## 0. Open decisions awaiting the maintainer (all trajectory-changing)

Four findings are verified and recorded but **deliberately not applied**, because
each alters plan ranking or run trajectories and is therefore behaviour-changing
under ADR-0004 (needs approval + A/B at identical seeds). Full detail:
`../research/RESEARCH_INVENTORY.md`.

| # | Finding | Proposed change | Cost of leaving it |
|---|---|---|---|
| 1 | **RC-005** — `cost_model` is shape-only; true cost carries flip density φ with a regime change at φ=1/3 | adopt `W = W_scan + W_flip·min(φ,⅓)` | Decision Engine **overcharges cold regimes up to 2.1×**, penalising low-temperature refinement |
| 2 | **RC-007** — 28/28 operator pairs commute in distribution (ρ < 1.4) | narrow the Evolution Engine's ordering search | compute spent on distinctions that do not exist at pair level |
| 3 | **RC-011** — the predictor's LOO Spearman is provably invariant to instance features | add instance×schedule interaction terms, **or** score across folds | no metric currently able to detect instance-conditional transfer |
| 4 | **RC-002** — all 18,570 runs start all-zeros; diverse init is worth 0.09–0.46% | one `random_flip_sweep` before the thermal phase | measured quality left on the table |
| 5 | **RC-012** — `--early-stop` cannot activate (6 rows < fit's 20-row floor), and even a fitted model's ε-gate is vacuous (predicts ≡0) | fix bootstrap AND training-target pathology **together**; fixing only the bootstrap ships a silent 50% truncation | a shipped flag that does nothing; ROADMAP Done-item 2 annotated |

**Why none was applied:** changing any of them breaks bit-identical replay of the
recorded corpus, or changes which plans the platform selects. That is the
maintainer's call, not an autonomous one.

Also open, non-trajectory: **`src/core/simd_utils.rs`** — orphaned, declared in no
`mod.rs`, never compiled, self-described as "a theoretical implementation".
Delete it or implement it; leaving it reads as a performance claim the code does
not support. **RC-013 evidence leans DELETE:** measured floors show the sparse
kernel is bookkeeping/RNG-bound in cache and gather-bound at DRAM scale —
neither is the f64 SAXPY the file targets. (Scope: engine_v2 backend measured;
ultimate.rs's MSC kernel was not.)

## 1. Benchmark degeneracy — G-Set cannot test two questions *in principle*

Not a statistical-power problem. An **identifiability** problem: no quantity of
additional G-Set runs fixes either one.

**(a) The guide axis is unidentifiable.** Every G-Set instance is unweighted
(J ∈ {−1,+1}); `dimacs_maxcut/sg3dl051000.mc` likewise. For J ∈ {±1}, with V
violated and S satisfied edges, E = V − S and V + S = |E|, so

    E = 2V − |E|

an affine bijection. Ranking by violation count **is** ranking by energy, so
energy-guided and constraint-guided search are one method written two ways.
All 18,570 recorded experiments ran on such instances and could not have
detected a difference between them even if one existed.

**(b) The frustration axis is confounded.** G-Set contains **zero unfrustrated
triangles** — most instances are all-positive-weight so every coupling is
antiferromagnetic and every triangle is frustrated by construction; the ±1
instances (G11–13, G32–34) are toroidal grids with no triangles at all. So
"frustrated triangle" and "any triangle" denote the same edge set, and a
frustration split has **no control group**. It also correlates with degree by
definition (69.3 vs 26.3 mean local degree), so any such comparison measures
density.

**Both need `biqmac`** (`be100.1.sparse` has 145 distinct weights, −100…100),
not G-Set.

**Method rule this produced:** before interpreting any grouped comparison, check
that the control group is non-empty and that the grouping variable is not forced
by graph structure to correlate with degree.

## 2. Empty capabilities and the orphaned SIMD file

**Three of six declared capabilities have zero implementing operators** —
`ExactInference`, `Approximate`, `Warmstart`. A capability query for them can
never return anything, silently. Note RC-002 (see [RESEARCH.md](RESEARCH.md))
gives a measured reason not to rush `Warmstart`: what an initialization
contributes is ensemble *entropy*, not solution *quality*.

**`src/core/simd_utils.rs`** exists (1,545 B) but is declared in no `mod.rs`
(`src/core/mod.rs` lists only `csr_matrix`, `hubo`, `anls`) — orphaned, never
compiled, self-described as "a theoretical implementation … a foundation for
replacing the serial iteration in `ultimate.rs`". **Decision needed: delete it
or implement it.** Leaving it reads as a performance claim the code does not
support, and it is the first thing an external reviewer would find.

## 3. From the authoritative sources (summary only — read them for detail)

- ~~`cargo doc` broken by rustdoc-link errors~~ **RESOLVED 2026-07-27** — 18 sites
  across 9 files, doc-comments only; now a `docs` job in CI with
  `RUSTDOCFLAGS: -D warnings`.
- ~~CI never checked the `research/` workspace member~~ **RESOLVED 2026-07-27** —
  root cause of the surviving E0063 break, see §4 below.
- Cloud routing recommended but no API key available
- World-filtered ideation and Dynamics early-stop are opt-in only
- `--early-stop` bootstraps one Dynamics model from `instance[0]`
- Consensus/theory facts publish at support=1 → low graph confidence
- Multi-instance `investigate` confidence stays low until run across many instances
- `experiments/` is generated state (gitignored, regenerable)

## 4. OPEN — the operator cost model omits its dominant variable (RC-005)

Every `cost_model` in the 18-operator library is a function of instance **shape**
only. But `SparseBitSlice::apply_flips` (`sparse_bitslice.rs:265`) dispatches on
**flip count**: scattered `O(deg·flips)` below φ=1/3, dense `O(deg·r)` above.
So the true cost carries a hidden parameter — **flip density φ = flips/r, which
equals the acceptance rate** — and the declared model is blind to it.

Measured (`src/bin/exp_cost_model.rs`, 4 G-Set instances, interleaved
temperatures + median so host drift cannot fake a trend): declared work constant,
actual ms varies **1.61–2.09×**. Corrected law
`W(φ) = W_scan + W_flip·min(φ, 1/3)` fits **R² 0.93–0.97**; the flip term is
**57–79% of cost at saturation**. The `min` cap is provably inert on the two
instances that never reach φ=1/3 (capped R² identical to uncapped to 4 d.p.) and
lifts R² by 5–8 points on the two that cross it — the signature of a correct
functional form, not a free parameter.

**Consequence:** the declared model is the corrected law with φ ≡ 1/3 hard-coded,
so it **overcharges by up to 2.1×, worst in the cold regime**. The Decision
Engine ranks plans by utility per unit cost, so low-temperature refinement is
systematically penalised — the same regime where
[[state-dependent-operator-selection]] says the best operator changes.

**Not fixed.** Correcting `cost_model` changes plan ranking and therefore
trajectories → behaviour-changing under ADR-0004, needs approval + A/B at
identical seeds. Full record: `../research/RC005_COST_MODEL_BLINDNESS.md`.

## 5. Resolved 2026-07-27 — the two gates that did not cover what they claimed

Both were the same failure class: a green pipeline that structurally could not
see the thing it was supposed to protect. Recording the mechanism, because the
pattern will recur.

**(a) CI excluded the `research/` workspace member.** `cargo metadata` shows
`workspace_default_members` = the **root package alone**, so `cargo build
--all-targets` (CI's command, no `--workspace`) never compiled `research/`.
That is why `research/src/bin/test_engine.rs` sat broken (E0063, missing
`energy_offset`) indefinitely — CI reported green because it never looked.
Fixed by adding `--workspace` to build/clippy/test and `--all` to fmt.

**(b) No documentation gate at all.** 18 rustdoc errors had accumulated across
9 files. Fixed, and a `docs` job added with `RUSTDOCFLAGS: -D warnings`.

**Verification standard used:** every one of the 7 `run:` commands in
`.github/workflows/ci.yml` was extracted and executed locally — all pass.
Widening CI scope is only safe once the newly-covered code is known to build,
so the debug-profile commands were run first, separately.

**Rule this produced:** a passing gate proves nothing until you check its
*scope*. For a cargo workspace, confirm `workspace_default_members` before
trusting any `cargo` command in CI. Note also that rustdoc aborts per crate, so
doc errors cascade — one clean run after a fix only proves the first failure is
gone; re-run until the count stops changing.
