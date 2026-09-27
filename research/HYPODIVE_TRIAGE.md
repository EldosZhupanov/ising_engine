# CD001 HYPODIVE falsification triage

Date 2026-09-17. Mode A: cheapest decisive checks before a new solver.
The user explicitly authorized this investigation after an earlier audit stop.

Strongest tested claim H11: low interface rank alone bounds the number of unique
conditional ground-state responses polynomially. Exact symbolic counterexample:
E(x,z)=(sum 2^i(x_i-z_i))^2, rectangular interface -2aa^T, unique response x=z.
There are 2^b answers, yet the response program simply copies b bits.

Falsifier: direct proof plus three-way objective check and exhaustive minimization
of every context for b1..6. [Result](breakthrough/h11/RESULTS.md). No holdout,
random worlds, seed independence or significance claims; the family was expressly
constructed to attack the assumption. Baselines are exhaustive minimization and
the strongest possible simple rule for this family, identity. There is no timing
race against a weak generic solver. All responses are returned, no abstention.

Decision **NO-GO** for H11. **NOT NOVEL** for finite-state exponential energy
reweighting, exactly Hedge. **NARROW/OVERLAPPING** for parameter-cell memory:
known lower-envelope geometry, analogous to explicit control but different from
continuous convex QP. No candidate survives as a scientifically new method.
This is a successful cheap rejection, not evidence all response compression fails.

Top risks: (1) conflating interface rank with zero-diagonal Ising matrix rank;
(2) hiding coefficient precision/normalized margin; (3) conflating response count
with program complexity. Explicitly addressed in the derivation/result.
Novelty search: [source matrix](PRIOR_ART_MATRIX.md), inspected primary equations
and source sections. The exact witness's historical priority remains UNKNOWN;
its known squared-penalty ingredients do not establish a new algorithm.

Stop condition met by counterexample. No automatic rescue under new assumptions.
Independent finished-code review passed on retry; Git freeze completed in this
CD001 commit. A materially different, precisely scoped hypothesis requires its own
selection and design. Broad research can continue under the user instruction; this
particular claim is terminated.

---

## Application search — 2026-09-27, base `3af7c25`

**Mode A; decision NARROW.** User requested further application discovery using
Hypodive, not another implementation campaign. Builder was consulted for the
eventual handoff but not activated. This iteration preserves the CD001 verdict
above and all frozen results. No production source, model weights, dependency,
solver process or benchmark dataset was changed. Review used the scoped catalogue,
Git state/history, an explicit read-only dependency map and primary literature;
it does not claim every file or all scientific literature was read afresh.

### Strongest defensible asset and scope

The repository can evaluate native Boolean interactions through degree four at
the MSC research kernel, with independently checked witness collection. This is
an implemented capability, **not established superiority**. HUBO-C001 tied native
OpenJij in 100/100 cells; HUBO-Q002 Phase B is MIXED, qualifying 2/4 strata against
the required 3/4. These opened synthetic cases are diagnostic only.

| Asset / actual caller | What it enables | Boundary |
|---|---|---|
| `src/core/hubo.rs`, `src/solver/engine.rs`, `research/examples/hubo_compare.rs` | Native degree-3/4 candidate objectives | Public `UltimateSolver::solve` still takes QUBO; explicit constant accounting; no arbitrary-order or quantum-state simulator |
| `src/presolve/qpbo.rs`, caller `src/solver/ultimate.rs` | Certified quadratic persistencies and residual search | Persistency does not certify semantic truth; quadratic reduction needs its own correctness proof |
| `research/examples/laya_bridge.rs`, `research/laya_semantic/` | Existing small semantic-to-optimization bridge | LAYA-001 is synthetic and fully presolved; bridge's exact oracle is limited to 16 variables; no real-data or training advantage |
| `src/engine_v2/ai_scientist/{policy,predictor,campaign}.rs` | Learned operator proposals and candidate filtering | Existing small policy/ridge mechanisms, not a demonstrated foundation model; simply adding a learned dispatcher is not new |
| `research/experiments/hubo_corpus_qualification/` | Exact reductions, instrumentation and auditable negative outcomes | Reusable methodology; opened data must not become the next holdout |

### Ranked hypotheses and concrete first gates

Ranking is a research judgment balancing existing code, application value and cost
to falsify, **not a probability of success**. All competitive application claims
are **HYPOTHESIS / INCONCLUSIVE**, not SUPPORTED by the toy checks below.

| Rank / ID | Application and beneficiary | Candidate contribution / strongest competitor | First decisive gate and stop condition |
|---|---|---|---|
| 1 / APP-01 | Globally consistent entity resolution: duplicate product records and catalogues; later other annotated entities | Frozen semantic scores, cubic transitivity objective, certified quadratic reductions if used, then residual search. Compare weighted cluster editing via ILP/CP-SAT and a simple closure heuristic; separate the scorer's contribution from the optimizer's. | Exact small partitions first; then WDC Products intake with entity/source-disjoint partitions and labels held from search. Require nonempty residuals, actual clustering accuracy and matched total cost. Stop solver-value claim if presolve solves everything or the same-score specialist dominates. |
| 2 / APP-02 | External Boolean constraint tasks; optimization/Ising researchers | Native degree-3 clauses without auxiliaries, compared with faithful native OpenJij, NeuroSA-HO and SAT specialist Kissat. Direct higher-order SAT is established prior art. | Exact signed-clause and tiny-instance semantics, independent CNF witness checker, pinned external SATLIB inputs. Qualification only after a protocol freezes instance family, budget, seeds and thresholds. Stop competitive claim if only a weak quadratic arm loses. |
| 3 / APP-03 | Post-training weight rounding for smaller models; inference/deployment teams | Conditional block optimization of reconstruction loss, with exact cross-block linear fields. Compare nearest rounding, coordinate search, AdaRound and model-appropriate GPTQ under the same grid/calibration data. | Tiny exact blocks, then held-out activation error AND downstream model quality at equal bitwidth and total optimization cost. Stop if gains disappear outside calibration or a cheap coordinate baseline matches them. Formulation novelty is rejected; no new training rule has been shown. |
| 4 / APP-04 | Low-energy atomic configurations under binary-alloy cluster expansions; computational materials groups | Degree-3/4 cluster interactions fit the kernel; compare icet/HiGHS MIP and composition-preserving canonical annealing. | Exhaustive tiny periodic cell matches `ce.predict`, including constants, normalization, orbit multiplicities and repeated sites; fixed composition must hold. Then new larger cells. Stop if mapping differs or advantage requires dropping composition. Surrogate minimum is not a DFT-validated material discovery. |
| 5 / APP-05 | Correlated quantum-error decoding; QEC researchers | Potential non-graphlike interactions, but correct logical-class inference and latency are missing. Compare correlation-enabled PyMatching and an appropriate code-specific decoder. | Validate syndrome and logical equivalence, not merely minimum error energy. The degeneracy toy below rejects equating error-MAP with logical-ML. DEFER full application: no decoder/data adapter exists, current degree limit may exclude relevant likelihoods. |

APP-01 is the clearest route to a useful **Laya + solver** system. Example:
pair predictions A=B, B=C, A!=C cannot all be an equivalence relation; global
optimization chooses a consistent partition rather than silently taking closure.
Consistency alone can still choose the wrong entities: independent annotations
are essential. An omitted candidate edge means unknown, not automatically false.
Dense modelling has O(m²) pair variables and O(m³) triangles, so blocking and
construction cost belong inside the budget. No large-scale claim yet.

APP-02 is the least speculative next **solver** application intake. The inspected
NeuroSA-HO repository exposes native and quadratic CPU code plus SATLIB families;
its `--trial-id` names output files and is not an RNG seed. Do not mistake repeated
labels for controlled independent trials. Its FPGA results are not a CPU baseline.
SATLIB's random satisfiable instances establish an external test, not industrial
utility or novelty. SAT success and MaxSAT optimization need different protocols;
a heuristic timeout never proves UNSAT. New cubic work must reuse existing
compiler/parser capabilities where verified, not assume they are missing.

APP-03 is a concrete AI use, but its local quadratic proxy is not exact end-to-end
nonlinear network loss. Row decomposition can be exact; arbitrary sub-row splitting
cannot discard interactions. AdaRound's supplementary first-layer experiment
already shows that a generic QUBO solver can underperform nearest rounding.
Better search is a hypothesis, not an automatic consequence of using our engine.

### Executed cheap checks

Source: [checks.py](application_triage/checks.py). Retained output:
[checks.json](application_triage/checks.json), source SHA and base commit included.
Reproduce without overwriting evidence:

```bash
python3 research/application_triage/checks.py
```

Five deterministic checks **PASS**; no random seeds, timing, significance or
application-performance claims. Independent exact enumeration is the baseline.
The script uses integers; the following are algebraic witnesses, not executions
of the production solver or validation of future adapters.

1. **Transitivity:** all eight states satisfy
   `ab + ac + bc - 3abc = number of violated triangle implications`.
   Five states are partitions. The example score's exact penalized minimizers
   select one of two single links; naive closure is worse for that score.
2. **Rounding:** weights (0.49,0.49), input (1,1), binary grid. All four states
   match scaled reconstruction loss `2401-2400q0-2400q1+5000q0q1`.
   Nearest rounding has scaled loss 2401; either mixed pair has loss 1.
   This known possibility says nothing about a real neural model.
3. **SAT:** eight signed three-literal clauses, all eight assignments each:
   64 expanded polynomial energies equal the independent unsatisfied-clause bit.
   Current test scope is distinct literals, not a complete CNF adapter.
4. **QEC:** stabilizers X1X2 and Z1Z2Z3; conditional X-only errors 100,010,001,111
   at syndrome (0,1) have probabilities .35,.05,.30,.30. Error-MAP selects 100,
   whose logical class has mass .40; the other class has mass .60. This n=3, k=1
   toy has distance one; it demonstrates a known inference distinction, not a
   realistic threshold or failure of all Ising decoders.
5. **CD003 precision counterexample:** k=5..8 give response counts 52,94,170,312
   with 218,624 exact conditional energies evaluated. See the general proof next.

### Prospective correction to CD003 (APP-06)

The preserved [CD003 report](breakthrough/cd003/RESULTS.md) states an overly broad
polynomial-compression condition `p << b/r`. **FALSIFIED as a sufficient
polynomial response-count condition**, even with every boundary coefficient
nonzero. Its finite-precision counting bound is not rejected.

For integer k>=5 set b=k², boundary vector
`v=(1,2,...,2^(k-1),1,...,1)` of length b, and internal vector
`u=(1,2,...,2^k)`. Boolean x has k+1 coordinates and z has b coordinates. Define
`E(x,z)=(u·x-v·z)²`. The cross-interface matrix is `-2uv^T`, of rank one.
Every boundary variable participates. Here K=max|v_i|=2^(k-1), so p=k-1=o(b).

The binary prefix and remaining unit coefficients attain every integer theta in
`[0, 2^k-1+b-k]`. Since b-k<=2^k, every theta has a unique k+1-bit internal
encoding, the sole zero-energy minimizer. Hence
`Nresponses=2^k+b-k=2^sqrt(b)+b-sqrt(b)`, superpolynomial in b despite p/b->0.
The asymptotic argument is analytical; finite enumeration checks its witnesses.
The responses in this family also admit a compact arithmetic program: number of
distinct responses is not program size or a general optimization lower bound.

From the old bound `(2bK+1)^r`, a sufficient polynomial-count condition is
`r log(2bK+1)=O(log b)`. A subexponential-count condition instead requires
`r log(2bK+1)=o(b)`. Neither is necessary for every individual instance.
Counting contexts alone does not bound the cost of solving the interior; even
rank zero permits an arbitrary hard disconnected internal QUBO. No general
circumvention of treewidth or NP-hardness follows. **NARROW**, not a new solver.

### Fairness, uncertainty and next action

Top risks: (1) testing a surrogate instead of the application's objective;
(2) excluding mapping/inference/presolve from cost or omitting specialist controls;
(3) rediscovering known formulations and interpreting a successful toy as novelty.
All five applications still lack matched application experiments. No coverage,
abstention, transfer or user-demand gain was measured. Future experiments must
share input access, total resource budgets, stopping and postprocessing rules;
retain failures/timeouts and report quality at matched coverage. Account for
training separately and report any amortization assumption. Freeze development
and holdout identities before outcomes; never tune on the opened Q002 corpus.

**Next selected task:** APP-01 data/formulation intake only: verify WDC label and
split semantics, score availability, candidate-graph size, and a small exact
partition oracle, then draft a binding pilot protocol. Laya must earn its role
against a simpler scorer. APP-02 is the fallback scientific intake if usable
entity annotations or a nontrivial residual cannot be established. Do not launch
five builds, training, an overnight hunt or a public submission from this triage.

Primary sources and search boundaries: [updated matrix](PRIOR_ART_MATRIX.md).
Read-only independent mathematical/prior-art review narrowed CD003, rounding,
QEC and the materials baseline. Finished artifact review **PASS**: all five
checks replayed exactly including source SHA, new inventory hashes match, memory
gate passes (357 Markdown / 91 immutable), staged diff check passes, no src/Cargo
change. No scientifically novel advantage established.

---

## GitHub opportunity triage — 2026-09-27, base `c8391a6`

**Mode A; decision NARROW.** User requested concrete external capabilities and
possible new projects. This is a source/code compatibility triage, not a runtime,
security, adoption, revenue or performance evaluation. No external software was
installed/executed; no model training, solver campaign or product migration was
started. Earlier records above remain historical. ER-001 is closed; its next
scorer/blocking design is not silently replaced by this product hypothesis.

### Evidence and scoped decisions

GitHub metadata, current upstream READMEs and selected source paths were inspected.
Stars, commit counts and archival status do not establish correctness, demand or
cheap maintenance. Reasons for abandonment are UNKNOWN unless stated by owners.
Task Magic and Aevatar lack exact URLs in the supplied list and were not resolved;
PlayTorch was not deep-reviewed. This is not a TOP-50 or whole-GitHub census.

| Candidate / inspected revision | Finding | Decision for this repository |
| --- | --- | --- |
| [Graft](https://github.com/trailhq/Graft/tree/80692e5ad0bc8e8f7e1edea648247d90b9f76820) | MIT; current parent is `trailhq/Graft`. `naouaro/graft` is an older fork; `NanoNets/context-graph-engine` redirects to the parent. Rust has a generic grammar (`src/graph/queries/rust.scm`) and optional rust-analyzer resolution. | First navigation candidate; qualify on an isolated snapshot before adoption. |
| [Arq](https://github.com/AssahBismarkabah/Arq/tree/d6f613bb9cd699eb2c39421fddcace6edb2bfa30) | Archived; Apache-2.0. `crates/arq-core/src/knowledge/parser/rust.rs` contains a `syn` visitor for Rust structural extraction. | Reference parser/ontology only; no whole-agent fork. A static visitor does not prove compiler-complete resolution. |
| [Vexify](https://github.com/AnEntrypoint/vexify/tree/82615d649b18da6a3d5d28d15aeb874dba6b674c) | Archived; MIT. Node package with SQLite/vector search, ingestion and MCP; package has native dependencies and postinstall logic. | Reference local ingestion; not an existing dependency-free single binary. Start with ordinary file search/SQLite before vector infrastructure. |
| [Reor](https://github.com/reorproject/reor/tree/9b47fcaf1158cedda1f0160392de25588efe4c31) | Archived; AGPL-3.0. Local notes, similarity links and RAG desktop app. | UI reference after demand; a desktop fork adds maintenance outside solver expertise. |
| [FTL](https://github.com/fastertools/ftl/tree/72334e3dc4149f6fb46eeb4f55272a926c5e544e) | Archived; Apache-2.0. Go CLI, Rust/Wasm components, polyglot SDKs, Spin/Wasmtime foundation. | Defer runtime work: no demonstrated sandbox gap in this task. |
| [Vector Admin](https://github.com/Mintplex-Labs/vector-admin/tree/5cd91c322b7361095954fb3aa4aa30b239d38ea4) | Archived; MIT. README says team focus moved to AnythingLLM. Its old promise not to archive conflicts with current GitHub state. | No immediate fit: this project has no demonstrated multi-vector-database operations problem. |
| [Prime Agent](https://github.com/PrimeIntellect-ai/prime-agent/tree/cd1f215cffd09223316c54dddae5e1b654718c31) | Active, MIT; persistent execution and harness refinement. `packages/coding-agent/skills/refine/SKILL.md` describes changes to prompts/memories/skills/subagent specs. | Reference recovery and reviewed memory updates; `/refine` is not evidence of weight training or guaranteed improvement. Already surveyed in EXTERNAL_PROJECTS_BACKLOG.md. |
| [Scientific Agent Skills](https://github.com/K-Dense-AI/scientific-agent-skills/tree/49c6e97775eaa18ba791bebe23162a70ae601c18) | Active, MIT; original claude-scientific-skills URL redirects here. | Select individual database/domain procedures when an application needs them; do not import the entire catalogue. Metadata-level fit review only. |
| [Open Code Review](https://github.com/alibaba/open-code-review/tree/486022daaf14f7142275eddb9b3cacc3cc5dadfa) | Active, Apache-2.0; deterministic checks plus agent review. | Optional engineering aid, not replacement for numerical invariants/independent experiment validation; not an Ising research contribution. |
| [Agenta](https://github.com/Agenta-AI/agenta/tree/257d41543b63eb9062c152c1deb21117d2823951) | Active workspace for agents/automations; GitHub license classifier returns NOASSERTION, not a license conclusion. | Revisit when a model-backed product needs evaluation/trace operations; no deployment now. Metadata-level review only. |
| [BrowserSkill](https://github.com/Tencent/BrowserSkill/tree/8ca7911778c30a73ff0149d53d267023c31961bc) and [agent-browser](https://github.com/vercel-labs/agent-browser/tree/d01253d9db28d75080e36da3c1c31ef89454731e) | Active browser automation projects; MIT and Apache-2.0 respectively. | Tools for a demonstrated browser workflow, not the next research direction. No browser/session access was performed. |
| [Laya notification app](https://github.com/aayushch/laya/tree/bbd6c799a255ff7eee1731effe1d23ec78538de9) | Apache-2.0 local notification aggregator. | Different project from [NandhaKishorM/laya](https://github.com/NandhaKishorM/laya), the decision model already studied here. No inference/training advantage follows from this app. |

Graft's reported SWE-bench 33/50 versus 27/50 is an author result, not reproduced
here. Its generated graph is a regenerable cache; initialization can change agent
wiring and build can change ignore configuration. Our Git/authority/evidence
records must remain canonical. Rust support is source-confirmed, not runtime-
qualified on this repository. An older fork's absent Rust support must not be
misreported as a limitation of the current parent.

### Claims, decisive checks, risks and prior art

- **GH-01, ENGINEERING, PLAUSIBLE:** a derived code/dependency index can reduce
  repeated exploration here. Supporting asset: Graft extraction paths plus our
  existing `memory/CATALOG.md`, `memory/FILE_MAP.tsv` and `scripts/build_file_map.py`.
  Simple competitor: current scoped `rg`, catalogue and source reads. No measured
  saving yet. Failure: worse answer correctness, stale source spans, missed dirty
  files or no meaningful saving once indexing/update costs are included.
- **GH-02, PRODUCT, PLAUSIBLE; novelty UNKNOWN/OVERLAPPING:** a small evidence
  navigator could connect claim -> implementation -> exact source/data hashes ->
  experiment -> checker outcome -> later correction. It would answer which
  results remain supported for which revision and assumptions. Existing assets:
  `memory/AUTHORITY.md`, `memory/BINDING_SHA256`, `scripts/check_memory_docs.sh`,
  ER-001 manifests and `research/experiments/entity_resolution/independent_audit.py`.
  These are components, not a released generic memory product or proof of demand.
  [Graphiti](https://github.com/getzep/graphiti) already provides temporal facts,
  provenance, invalidation and MCP; that broad feature combination is not novel.
  The proposed narrower distinction is executable evidence applicability to code
  and experiment versions. Whether it beats Graphiti plus simple scripts is UNKNOWN.
- **GH-03, NOVELTY, NARROWED:** reviving FTL is not entering an empty MCP/Wasm
  field. [Wasmcp](https://github.com/wasmcp/wasmcp) already composes polyglot MCP
  components on component-capable runtimes. [Spin's own walkthrough](https://github.com/spinframework/spin-docs/blob/main/content/blog/mcp-with-wasmcp.md)
  describes this combination. This defeats the broad unoccupied-opportunity
  premise, not the possibility of a differentiated runtime. Portability of arbitrary
  native/GPU tools and actual isolation were not tested.
- **GH-04, PRODUCT, UNSUPPORTED:** combining Reor + FTL + Vexify + Graft produces
  a valuable Agent OS. There is no matched baseline, integrated prototype, customer
  evidence or verified cost estimate. NO-GO for a large build from this survey.
- **GH-05, RESEARCH, INCONCLUSIVE:** using the Ising optimizer for memory/context
  selection or reconciling claims. A consistent selection is not a truth proof.
  Source authority and explicit supersession need deterministic rules; contradictions
  should be retained, not erased to minimize energy. ER-001 found no added search
  benefit on its small objectives. Do not force QUBO into this product.

Top risks: unsupported summaries becoming authoritative memory; spending on an
integration platform before a useful workflow exists; and product claims derived
from a single development repository or author benchmarks. The source checks above
are the executed cheap checks. No new empirical product kill-test was run, so
absence of advantage is not a demonstrated negative result.

### Three directions and the smallest justified next action

1. **Immediate infrastructure:** qualify Graft as a replaceable navigation adapter.
   Preserve existing canonical documents. Test on an isolated snapshot, including
   staged/unstaged/untracked and ignored-evidence fixtures; do not auto-promote
   generated Markdown into the memory catalogue.
2. **New product candidate:** a CLI/MCP evidence navigator for research/coding teams.
   First deliverable would answer “what supports this claim now?” with source
   spans, artifact hashes, scope, failed checks and a reproduction command. Start
   read-only; no desktop UI, account connectors or runtime platform required.
3. **AI/solver research continuation:** learned entity scores with checked joint
   decisions, following the already identified ER-001 scorer/blocking gate. This
   uses the optimizer more directly, but semantic benefit and search advantage
   remain separate hypotheses. No new training follows from this survey.

**Recommended design-only next step for GH-01/GH-02:** freeze a small diagnostic
suite of 20 questions with manually verified evidence chains, using historical
contradictions, dirty-tree changes and invalidated artifacts. Ising Engine cases
are development data, never an independent generalization test. Compare (A) current
catalogue + `rg`, (B) Graft, (C) Graft plus a minimal evidence adapter with identical
model, tools, context budget, stopping and update conditions. Include an existing
memory system plus verification scripts before any competitive novelty claim.

Proposed go/no-go gate to freeze before measurements: all explicit deterministic
hash/authority failure fixtures detected; at least 18/20 evidence-correct answers
without lower correctness or hidden abstention; and >=25% fewer input tokens than
the strongest baseline, including amortized index/update costs over a stated use
count. These are proposed engineering criteria, not a preregistered experiment or
observed outcomes. Unknown answers count separately from correct/incorrect ones.
Use paired repeated runs for stochastic agent comparisons; 20 questions cannot
establish broad statistical superiority. Stop expansion if the simple workflow is
equally useful. Before a product investment, validate on additional repositories
and observe external users completing the workflow without its author.

**Theory:** no new theorem or solver guarantee. **Reproducibility:** pinned source
links and local base above; installed versions/runtime behavior were not tested.
**Stop:** opportunity triage ends here. Only a bounded compatibility/design check
is justified; no automatic Agent OS build, dependency installation or pivot.


## Graft local integration — 2026-09-27

User explicitly authorized this installation after the opportunity triage.
**Operational compatibility: SUPPORTED within the tests below. Productivity or
research advantage: UNVERIFIED.** This is an optional structural CLI, not a new
source of repository authority or an automatic agent/MCP integration.

Pinned [trailhq/Graft](https://github.com/trailhq/Graft/tree/80692e5ad0bc8e8f7e1edea648247d90b9f76820)
0.20.0; archive SHA256
`15b611f363cea321649de3b8a570d82da891a81dd22f83a2f19b73e28e84dad6`.
Installed with `npm ci` on Node v24.18.0 under
`.cache/graft-toolchain/80692e5ad0bc8e8f7e1edea648247d90b9f76820`.
[Adapter](../scripts/graft.sh) sets telemetry opt-out and hash-based refresh;
structural commands use no configured model. Network isolation was not measured.
No global package, hook, agent configuration, model weights, production Rust or
Cargo files changed. The cache is regenerable and remains Git-ignored.

Qualification uses [20 fixed development probes](graft_qualification/cases.json)
on a disposable Git snapshot, not an independent holdout. Sixteen exact-symbol
lookups and four document probes are scored by expected-path presence; returned
spans are checked for existence/bounds, not semantic correctness. Graft found
15/16 code targets in its first eight results; declaration `rg` found 16/16 over
all matches and 14/16 in alphabetically sorted first eight. Document results:
Graft 0/4; explicit-file literal `rg` 4/4 (3/4 in sorted first eight). QuboModel's
multiple definitions account for the missed code target. These distinct ranking
rules are descriptive diagnostics, not an equal-cost superiority comparison.
The earlier proposed 18/20 evidence-QA and token-cost gate was NOT executed.

All **12 compatibility checks PASS**: `.sol` exclusion (including an adjacent
`foo.rs.sol` fixture), scope-override rejection without graph mutation, existing
pointers, same-size/same-mtime content refresh, staged edits, untracked additions,
ignored-code exclusion, deletion, snapshot preservation, no unexpected visible
files, temporary-file cleanup, and original-checkout preservation. The fixture
indexed 440 files, 6,237 nodes and 10,165 approximate edges. LSP was not enabled.
Live symbol/skeleton/callers commands also ran successfully. `check --json`
reports graph freshness OK but missing concept context, as expected for this
structural-only installation; it is not a whole-repository evidence check.

```bash
bash scripts/graft.sh setup
python3 research/graft_qualification/check.py
bash scripts/graft.sh ask "UltimateSolver" --in src/solver/ultimate.rs --source
```

[Final raw result](graft_qualification/results.json) embeds the executed runner
and adapter plus their hashes; [source manifest](graft_qualification/source_manifest.json)
records the tested dirty snapshot based on c8391a6. Each reproduction creates a
new cache run and does not overwrite published results.

Negative attempts are retained: [attempt 01](graft_qualification/attempt01.json)
exposed fixture Git-ignore omissions, invalid bare-pointer rejection and the
upstream structural `--extensions` limitation; [attempt 02](graft_qualification/attempt02.json)
still indexed 717 solution files despite directory scoping. Explicit source-file
whitelisting fixes this. [Attempt 03](graft_qualification/attempt03.json) passed
10 tests before review requested scope enforcement; the final run added rejection
and cleanup coverage. Whitelist matching upstream is equality or slash-bounded
prefix, not an unbounded string prefix. Every call recomputes membership.

Independent read-only review found a whitelist bypass via forwarded `--only-dir`.
It is fixed and covered by the final run. An initial re-review was blocked by
a reviewer service limit; the subsequent independent re-review **PASS** checked
reserved flags, root scoping, cleanup, retained hashes, embedded sources and
reported counts. This is integration review, not a productivity experiment.
No performance claim, scientific result, or external publication follows from
this integration. Next: use scoped navigation
in ordinary work and retain `rg`/catalogue as fallbacks.

## Scientific ecosystem and remote-audit reconciliation — 2026-09-27

**Question:** which proposed external capabilities address a demonstrated local
gap, rather than recreating an existing module or replacing the project with an
unvalidated physics platform? **Decision: NARROW.** Prefer external comparative
baselines and a bounded model-compilation capability. No external performance,
new training method, record, or universal solver claim is supported by this pass.

Scope: primary repository/docs inspection, targeted local dependency mapping,
Git-history checks and three unchanged regression suites on local base c8391a6.
The user's remote audit does not identify an exact remote SHA, so it is a set of
claims to check, not evidence about current local behavior. The local tree is
newer in functionality; its date alone is not a correctness argument.
[Source hashes, test log, source URLs and verified bibliography](literature/scientific_ecosystem_20260927.json)
are retained. No external solver benchmark was run.

### Local claims reconciled

| Supplied claim | Current evidence | Conclusion / remaining scope |
|---|---|---|
| Direct QUBO coefficient copy plus XOR minimizes another polynomial | `ultimate.rs:147` copies coefficients, but `engine.rs:366` now uses binary products, also for cubic/quartic terms; basis suite 4/4 PASS | This specific XOR criticism is stale. This is not certification of every input/API/offset convention. |
| One slice receives spurious Trotter flip penalty | `engine.rs:343,589` force coupling to zero for one slice; Trotter suite 5/5 PASS | Specific single-slice bug is covered and not reproduced. |
| Returns only hot lane zero | `ultimate.rs:349–365` enumerates populations, temperatures, slices and all 64 lanes; incumbent/polishing exists | Claim is stale for this path. Best at checkpoints is not a claim to capture every intermediate flip. |
| Population selection uses hottest level | `ultimate.rs:729–735` uses `cold = nt - 1` during PA reweighting | Claim is stale for current PA path. |
| Multi-slice physics is validated by drift tests | `engine.rs:412` adds `+j_tau*(1-parity)`, while `ultimate.rs:282` sets positive coupling | Unresolved physical interpretation: aligned pair costs +J and opposite pair costs 0, favoring anti-alignment. Energy/delta consistency is not ferromagnetic SQA validation. Keep SQA physics claims blocked; classical single-slice path is separate. |
| No portfolio/selection capability exists | `engine_v2/ir.rs`, `decision.rs`, `registry.rs` provide IR, structural features, plan/backend selection and operators including Tabu/PA/cluster/elite/restart | A foundation already exists. Missing standalone `PortfolioSolver` does not establish missing functionality; equal-cost advantage remains unproven. The legacy scalar `tabu.rs` is still not exported. |
| Bayesian Autopilot has no Bayesian model | `solver/autopilot.rs` samples randomly then perturbs current best; no posterior/acquisition implementation in this path | Criticism remains applicable. Do not describe this implementation as Bayesian optimization. |
| Official GSET is generated and Gurobi timing is hardcoded | `bin/gset_official_benchmark.rs:9–10,109` explicitly generates a graph and sets 45.0 seconds | Criticism remains applicable. This binary is not valid official-G1/commercial-speed evidence. Preserve historical artifacts but quarantine these claims before publication. |

Reproduction: `cargo test --release --test test_basis_correctness --test test_trotter_correctness --test test_schedule_and_incumbent`:
**13 passed, 0 failed** (4 + 5 + 4). This was a targeted rerun, not the entire
Cargo gate. Source was not edited. The multi-slice polarity finding is a direct
reading/algebra check, not an executed physical-reference comparison.

### External capabilities ranked for this repository

| Priority | Source / capability | What is useful here | Falsification or scope limit |
|---|---|---|---|
| 1 | [MQLib](https://github.com/MQLib/MQLib) — published MaxCut/QUBO heuristics and ML hyper-heuristic, MIT | Independent CPU competitors and algorithm-complementarity measurements for existing engine_v2 | First compare single best fixed method, uniform allocation and an optimistic per-instance oracle. If even oracle complementarity is negligible, another selector cannot justify its overhead on that corpus. Selection itself is established prior art. |
| 2 | [Simulated Bifurcation](https://github.com/bqth29/simulated-bifurcation-algorithm) — PyTorch CPU/GPU | A different search family; initially an external process adapter, not a Rust/CUDA rewrite | Match wall time AND resources; charge transfer, startup and parameter selection consistently. Dense GPU behavior says little about sparse CPU tasks. No implemented SB operator found in the inspected local registry. |
| 3 | [PyQUBO](https://github.com/recruit-communications/pyqubo), [qubovert](https://github.com/jtiosue/qubovert) | Modeling/transform reference for a constrained Boolean-polynomial frontend; retain native HUBO where useful | Generic DSL, quadratization and dispatch already exist elsewhere. Value must be demonstrated via fewer errors, lower compilation cost or better end-to-end outcomes; check offsets, auxiliary-variable projection and penalty conditions independently. |
| 4 | [D-Wave samplers](https://github.com/dwavesystems/dwave-samplers), [hybrid](https://github.com/dwavesystems/dwave-hybrid) | SA/Tabu/descent baselines and a decomposition reference; avoid reimplementing all baselines | CPU-only participation does not require QPU. [qbsolv](https://github.com/dwavesystems/qbsolv) explicitly names successors; forking the retired tool is unnecessary. |
| Compare, do not assume reusable source | [QUBO++](https://qubo-plus.github.io/) / ABS3 | Important competing model/solver API, including HUBO and heterogeneous solvers | Official [installation](https://qubo-plus.github.io/en/INSTALL) requires license activation. A public docs repository is not permission to fork its solver core. Availability and benchmark terms must be resolved before integration. |
| Later application | [JAX-FEM](https://github.com/deepmodeling/jax-fem), [FEMcy](https://github.com/mo-hanxuan/FEMcy) | Discrete material/actuator/design choices around a validated continuous simulator | Start one small inverse-design question. Compete with continuous relaxation/rounding and established optimization; include every simulation in total cost. JAX-FEM documents GPL-3.0, not a blanket permissive license. |
| Later formal methods | [IHeartLA](https://github.com/iheartla/iheartla), [SciLean](https://github.com/lecopivo/SciLean) | Typed mathematical expressions and verified transformations as design references | SciLean calls itself an early proof of concept. Lean mathematics does not automatically verify our floating-point implementation, discretization or whole solver. Do not make it a mandatory dependency now. |
| Separate domain investment | [DQC](https://github.com/diffqc/dqc) | Differentiable HF/DFT can be useful for a specific molecular objective | Neither a classical Ising optimizer nor differentiable chemistry provides an automatic quantum-dynamics simulator. No current molecular question/data/reference target justifies a pivot. |
| Optional later uncertainty model | [ProbNum](https://github.com/probabilistic-numerics/probnum) | Probabilistic numerical methods for approximated continuous computations | Numerical posterior uncertainty is not a certified combinatorial optimality gap or calibrated solver success probability. |

For geometry/physics archaeology, [PyMesh](https://github.com/PyMesh/PyMesh),
[Jet](https://github.com/doyubkim/fluid-engine-dev),
[DiffCloth](https://github.com/omegaiota/DiffCloth),
[Tiny Differentiable Simulator](https://github.com/erwincoumans/tiny-differentiable-simulator),
[PlasticineLab](https://github.com/hzaskywalker/PlasticineLab),
[clifford](https://github.com/pygae/clifford) and
[RIVET](https://github.com/rivetTDA/rivet) are references for different workloads,
not interchangeable solver modules. No local integration question or baseline
currently supports prioritizing their resurrection. Jet describes computer-
graphics fluids: this alone does not validate engineering aerodynamics.
[SQAOD](https://github.com/shinmorino/sqaod) is a CPU/CUDA SQA reference, but a GPU
port is premature while our multi-slice physical convention remains unresolved.
PathSim, Geomstats and PDEBench were not deeply audited in this pass; no dormant
status, freshness or integration recommendation is inferred for them.

Two corrections to the opportunity premise: the archived scikit-optimize origin
has a [continuation](https://github.com/holgern/scikit-optimize) that explicitly
publishes to the same PyPI package; [SICMUtils](https://github.com/sicmutils/sicmutils)
explicitly moved development to [Emmy](https://github.com/mentat-collective/emmy).
For Bayesian experimental design, [BoTorch](https://github.com/meta-pytorch/botorch)
is an existing modular baseline, not an empty market. Last commit dates, stars
and inactivity durations in the supplied prose were not adopted as verified
facts; they do not establish an algorithmic or product opportunity.

### Bounded proposal and stop rules

My recommended research/product hypothesis is **a checked discrete-model
compiler and solver evaluation layer**, built around existing Ising Engine
modules. Input is an explicit constrained Boolean polynomial; output includes
the transformed model, assumptions, decoded candidate, independent feasibility/
objective checks and provenance. A heuristic candidate is not called optimal
without a bound/proof. Later, an AI model may propose scores, encodings or solver
allocation, while checks and held-out comparisons evaluate its contribution.
A compiler plus AI plus checks is not, by itself, a new learning paradigm.

1. **P0 (evidence):** close the pending independent Graft review; isolate the
   misleading GSET/Gurobi claims and specify a separate multi-slice physics
   regression before any SQA comparison. Do not silently change a scientific
   Hamiltonian inside navigation work.
2. **P1 (next bounded investigation):** define an external MQLib baseline adapter
   with exhaustive small-model energy/sign/offset validation and original input
   hashes. Inspect the existing external-comparison protocol amendment requirement
   before a new campaign; do not launch it under a conflicting handoff.
3. **RESEARCH:** preregister a family-separated, held-out comparison of existing
   methods against a fixed best baseline, uniform portfolio and selector; include
   feature extraction, preprocessing, compilation and tuning costs. Use the
   development oracle only to estimate room for improvement, never as a deployable
   competitor. If there is no room, stop selector expansion on that corpus.
4. **P2:** only then test external SB for complementary failures, or one checked
   compilation transformation. Require an ablation to distinguish improvements
   from a larger compute budget. Reuse engine_v2 instead of adding a second
   unrelated portfolio architecture.
5. **Deferred:** a single mixed discrete/continuous physical-design application
   with a domain collaborator and independently recomputed physical objective.
   DQC/FEM/CFD/GA/TDA do not become a universal solver merely by sharing an API.

No solver rearchitecture, training, heavy scientific dependency, GPU purchase or
record-hunting run was performed. Final independent review is still unavailable
because of the reviewer service limit; findings retain their stated verification
scope rather than claiming a completed whole-repository audit.
