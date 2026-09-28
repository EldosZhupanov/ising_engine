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


## GitHub publication correction — 2026-09-27

The user authorized updating the GitHub repository after discovering that its
`master` still pointed to March commit `7df051b`. Local c8391a6 is 253 commits
ahead of that ancestor. Publish by ordinary fast-forward only; do not rewrite
remote history. This record prospectively supersedes the corresponding current-
code findings above; historical audit bytes and benchmark artifacts remain.

- The historical `gset_official_benchmark` binary now explicitly labels its input
  synthetic, seeds graph generation, prints actual graph size, and reports only
  measured solve time and recomputed cut/energy. No G1 optimum, fictitious Gurobi
  time, speedup ratio, GNN or quantum comparison is emitted. It still does not
  load an original Gset file.
- A real formulation defect was also found: each sampled edge previously reduced
  only the first endpoint's linear coefficient. Both now receive `-w`, restoring
  `E(x) = -cut(x)`. A complete three-vertex graph is exhaustively checked against
  the independent `k*(3-k)` cut formula; a seeded sparse six-vertex graph checks
  all states plus repeatability.
- `RandomSearchOrchestrator` names the actual random-search/local-perturbation
  method; `BayesianOrchestrator` remains a public compatibility alias. Descriptions
  no longer imply posterior modeling, gradients, or certified optimal parameters.
  The wrapper uses canonical model energy, fixing omission of `energy_offset`;
  a regression checks a nonzero-offset model through the public solve method.
- Solver kernels, family boundaries, public solve signatures, and existing
  research protocols are unchanged. Multi-slice SQA validation remains open.
  No new speed claim or external comparison run is made by these fixes.

The README now routes GitHub/connector readers to START_HERE, NOW, projects and
research evidence, including negative results and the remaining SQA limitation.
Graft integration has since passed independent re-review. Publication gates and
remote verification are recorded in NOW and the Git history, not inferred from
older reports.

Publication validation: full workspace release tests **787 passed, 0 failed,
5 intentionally ignored profiling cases**; check, release binaries, strict
all-target Clippy, formatting and Rustdoc PASS. Rustdoc initially rejected one
bare URL and two `StepEvent[0]` comment references in historical binaries; only
Markdown markup was corrected. [Retained commands/logs](publication_checks/20260927.json)
preserve the failures and successful rerun. Frozen scientific records were not
edited. These checks establish the stated engineering scope, not performance.

---

## Solver market triage — 2026-09-28, base `255af15`

**Mode A; decision NARROW.** Product hypothesis P-MKT-001: a carefully improved
Ising Engine could be useful to customers as a standalone QUBO/Ising solver.
There is demonstrated demand for *optimization outcomes*, but no evidence yet
that customers need this particular solver or would pay for it. No market-size,
revenue or product-market-fit estimate is inferred from solver availability,
GitHub interest or vendor case studies. This is a source and repository-evidence
triage, not customer discovery or a matched product trial.

### Evidence and strongest simple competitor

- Broad need is concrete: [Google OR-Tools](https://developers.google.com/optimization/introduction)
  supports routing, scheduling, bin packing, CP and MIP, and its
  [routing guide](https://developers.google.com/optimization/routing) identifies
  capacity, time-window and resource constraints. The same guide distinguishes
  its free solver from a paid route-optimization service. This supports demand
  for integrated, reliable decisions more directly than demand for a raw QUBO API.
- [Kaneka's production-planning case](https://www.gurobi.com/resources/case-studies/kaneka-production-planning-optimization)
  describes a deployed optimizer embedded in a cutting-plan application.
  [Fujitsu's Baptist Health case](https://info.archives.global.fujitsu/global/about/resources/news/press-releases/2023/0914-01.html)
  reports a trial across 14 operating rooms and a 37% increase in available
  prime-time minutes. These are vendor-reported cases, not independently audited
  ROI estimates and not a QUBO-only market sample.
- Specialized supply exists: [MQLib](https://github.com/MQLib/MQLib) includes
  many Max-Cut/QUBO heuristics and an instance-aware selector;
  [Toshiba SQBM+](https://news.toshiba.com/press-releases/press-release-details/2023/Toshiba-Brings-SQBM-its-Quantum-Inspired-Optimization-Solution--to-AWS-Marketplace/default.aspx)
  was distributed as an embeddable commercial optimization module. The strongest
  simple competitor for a proposed customer workflow is the existing domain
  stack (for example OR-Tools/CP-SAT or a MIP solver); for a native QUBO API it
  is an existing QUBO heuristic such as MQLib. Both must be compared at equal
  delivered-result wall time, hardware, feasibility rules and integration cost.
- Local evidence is narrower than a product claim: [MQ-QUALITY-002](experiments/mqlib_coarse_quality/closure/RESULT.md)
  audited 960 synthetic qualification cells, found no registered complementary
  winners and had 60/240 Ultimate fallback-only cells. [MQ-FIRST-CHUNK-001](experiments/mqlib_first_chunk/RESULT.md)
  identified a delivered-output cadence issue on exposed dense inputs; it did
  not isolate search speed. [LABS-Q002](experiments/labs_q002/RESULT.md) failed
  its operational gate against lMAts. The premise that 41 Market Split inputs
  lack solutions was [falsified](EXP005_MARKETSPLIT_TRIAGE.md). No held-out
  customer dataset, paid pilot, replicated edge or record supports a product
  superiority claim.

### Cheap falsifier, risks and decision

The core commercial claim would fail if a real task owner will not supply a
repeatable problem/data/decision metric, or if a standard domain solver produces
equally feasible decisions at equal total cost. A benchmark-only energy win
without a feasible decoded business decision is insufficient. No such customer
trial has been run; **P-MKT-001 remains UNSUPPORTED**, not disproved. No
scientific novelty is claimed: algorithm selection and QUBO heuristics have
clear prior art in MQLib. Worlds/seeds/leakage and abstention are N/A to this
market-source screen; they become binding in any future solver comparison.

Top risks are (1) confusing the large operations-research market with the much
narrower QUBO solver niche; (2) undercounting modeling, constraints, integration
and support while comparing only kernel time; (3) choosing easy or previously
exposed benchmark instances and converting a local win into a general claim.

Recommended bounded validation, not a new active research protocol: select one
specific decision workflow with an external task owner; obtain historical
instances and an independently recomputed operational objective; compare the
existing workflow, a strong domain solver and Ising Engine with all modeling,
feasibility, delivery and support costs counted. Separately, qualify the native
QUBO component on untouched held-out instances against MQLib and another
relevant implementation. A pilot or product claim needs both a verified
practical advantage and credible buyer interest; otherwise keep the engine as an
open research/evaluation component and stop investing in generic-solver marketing.
The current engineering next step remains the budget-aware Ultimate output
interface map in [NOW](../memory/NOW.md); this triage does not authorize a new
solver campaign or modify any frozen experiment.

---

## New-product discovery after owner pause — 2026-09-28, base `da8981b`

**Mode A; decision NARROW / NO BUILD YET.** The owner explicitly paused Ising
Engine development and requested a broadly useful, measurable, potentially
breakthrough product. This changes the live task, not the historical mission or
the status of frozen scientific results. The strongest candidate claim is:
"an independent verification layer for computational claims made by coding and
research agents could reduce false success reports at acceptable cost." This is
a **HYPOTHESIS**, not an invention, customer-validated product or established
advantage. Assumption for the screen: one small team, software-first, no large
training budget, and no access yet to external customer traces.

### Why investigate and what already exists

[Anthropic's measured agent use](https://www.anthropic.com/research/measuring-agent-autonomy)
shows growth in the longest Claude Code turns, while typical turns were much
more stable; it does not quantify this product's demand.
[SWE-Cycle](https://arxiv.org/abs/2605.13139) reports a drop in code-agent solve
rates on end-to-end tasks and builds an execution-capable judge because simpler
evaluation misses errors. A [2026 survey of AI research agents](https://arxiv.org/abs/2608.05179)
reports incomplete release of seeds/traces and verification methods in its coded
sample. These are primary research signals of a verification problem, not market
size, willingness to pay, or proof that our proposed method solves it.

| Candidate | Strongest simple competitor / direct overlap | Triage |
|---|---|---|
| General agent memory / repository context | [Graft](https://github.com/trailhq/Graft), [Graphiti](https://github.com/getzep/graphiti), [Letta](https://github.com/letta-ai) already address code context or persistent/temporal agent memory. | **NO-GO as a generic entry point now**; no local evidence of a missing measurable capability. |
| General agent tracing, evals and dashboards | [Langfuse](https://github.com/langfuse/langfuse), [LangSmith](https://docs.langchain.com/langsmith/evaluation-types), and [OpenTelemetry GenAI conventions](https://github.com/open-telemetry/semantic-conventions-genai) already provide substantial tracing/evaluation infrastructure. | **NO-GO for another generic observability platform**; no differentiated baseline result. |
| Independent claim-to-evidence verification | [GitHub CI and artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations), Langfuse custom/code evaluators, and newer direct overlaps [Tessera](https://github.com/robert-vetter/tessera), [AgentTrial](https://github.com/tang-vu/agenttrial), [Agent Evidence Levels](https://github.com/luckyPipewrench/agent-evidence-levels), and [data-to-paper](https://github.com/Technion-Kishony-lab/data-to-paper). The latter projects' self-descriptions are existence/prior-art evidence, not independently validated performance. | **NARROW / OVERLAPPING**: candidate only for claims with an external, machine-checkable oracle and a measured gap against these tools. |

The proposed narrow workflow is `claim -> pinned input/code/artifact -> fresh
independent verifier -> PASS / FAIL / UNKNOWN with reproduction receipt`. It
must re-evaluate the *claimed result*, not merely preserve a producer-signed
trace. A verifier can check objective values, deadlines, test outcomes and source
hashes; it cannot generally prove arbitrary natural-language or scientific
novelty claims. A missing oracle must be `UNKNOWN`, never an invented PASS. The
closest known concepts include CI, artifact provenance, execution-based
benchmarks and evidence-gated agents. **Novelty is UNKNOWN/OVERLAPPING.** The
only possible differentiated claim would be an empirically better coverage vs
cost/error frontier on a specific workflow.

### Kill-test before implementation or product claim

First obtain a prospective corpus of at least 30 real coding/research-agent
deliverables from more than one workflow, with final claims, artifacts and
independently established outcomes; disclose exclusions and any overlap with
this repository. Freeze claim classes and verifier rules before labels are
inspected. Compare the *same* cases under ordinary CI plus available
trace/evaluator/provenance tooling, a human checklist, and the proposed
claim-bound verifier. Primary endpoint: materially false `DONE` reports missed
by each approach. Also count correct-work rejections, UNKNOWNs, setup effort,
reviewer minutes and verification cost. Equal data access, stopping and
abstention are required. A seeded synthetic-fault set is only a control, not a
substitute for real failures. No such experiment has been run.

**Falsifiers / stop conditions:** if the ordinary stack catches the same
material failures at similar or lower total cost, or if no external team has
the problem and budget to pilot it, do not build a generic product. A positive
local replay in Ising Engine would validate an internal need only. Two separate
gates are required: a matched technical advantage and credible external buyer
interest. The former may be tested without contacting anyone; the latter cannot
be inferred from repository searches. No third-party contact, purchase, new
project repository, solver run, model training or code implementation occurred
in this screen.

Top risks: (1) the apparent gap is already covered by CI/custom evaluators or
direct evidence-layer products; (2) verification plugins remain domain-specific
and cannot support a truly universal truth claim; (3) the team values a result
but will not pay for another integration layer. Seed/trajectory independence,
cost-normalized success and leakage controls are N/A until a comparison corpus
is frozen. The current conclusion is a **testable path**, not a revolutionary
result. Ising Engine remains preserved and paused at its recorded resume point
in [NOW](../memory/NOW.md).

---

## Dual-track discovery update — 2026-09-28, base `d173c30`

**Mode A; decision NARROW on verification, NO-GO for an unqualified 4–5x
mathematical claim.** This is a literature and artifact-availability check, not
a new solver implementation, speed measurement, buyer interview or proof of
novelty. The owner keeps Ising Engine paused and asked to continue product
research while seeking an independent mathematical direction.

### Track V — false-success verification

[Advani, 2026](https://arxiv.org/abs/2606.09863), an arXiv preprint accepted to
the FAGEN@ICML2026 workshop, studies false completion claims against
programmatic environment outcomes. In its own tau2-bench and AppWorld samples,
lightweight text/action detectors outperform LLM judges; this is **author-reported
evidence**, not our reproduction. It directly falsifies a broad novelty claim
for detecting confident but false agent completion by inspecting a trajectory.
Its paper also reports 50% precision at a 10% flag rate and warns that its
AppWorld judge comparison distinguishes false from honest *failures*, without a
true-success class. The 3% dual-control observation has one confounded domain
and is not a causal estimate of what an independent verifier would achieve.

The possible contribution is narrower: on a specified workflow with an
independent state oracle, can a claim-bound verifier catch material false
`DONE` statements **that ordinary CI, benchmark verdicts, provenance and a
cheap classifier do not catch**, at acceptable false-reject/UNKNOWN rates and
total reviewer time? [SWE-bench's public experiment registry](https://github.com/swe-bench/experiments)
links predictions, evaluation reports, test logs and sometimes trajectories.
That establishes a candidate source, not yet a qualified claim/outcome corpus:
public trajectories may lack explicit final claims or complete artifacts.
Its documented `swebench submit verify` already re-derives verdicts from
recorded test output, so merely repeating this regrade would add no value.
One local read-only attempt to list current submissions through the GitHub API
failed with temporary DNS resolution; no individual trajectory was qualified
and no missing artifact is inferred from that failure.
The preceding 30-case prospective-corpus requirement remains; exposed cases
from this search are for qualification only. Real deployment also requires
independent buyer evidence. Strongest competing explanation: a simple rule
reading the official verdict or database state already solves the problem.

**Next cheap kill-test:** inspect a small, predeclared sample of public run
artifacts for explicit final claims, pinned patch/artifact, independent verdict,
and legal/reproducible access. Reject this corpus if those fields cannot be
joined reliably. If joinable, freeze a new evaluation split and compare the
oracle rule, ordinary CI/eval/provenance, a cheap detector, and any proposed
verifier at matched access and cost. Do not train/tune on its holdout.

### Track M — repeated sparse linear systems (independent of Ising Engine)

Candidate question: can selective reuse of numerical information accelerate a
*sequence* of changing sparse linear systems at the same true-residual tolerance
and lower total time? This is a worthwhile applied-mathematics area, but the
generic method is **NOT NOVEL**. [Soodhalter, de Sturler and Kilmer (2020)](https://doi.org/10.1002/gamm.202000016)
survey subspace recycling; [Carr, de Sturler and Gugercin (2021)](https://doi.org/10.1137/20M1331123)
already recycle preconditioners for parametrized sequences. An adjacent
[Parth study (2025)](https://doi.org/10.1145/3731179) reports up to 14x faster
*reordering* but about 2x end-to-end Cholesky solves on its studied workloads:
accelerating a stage does not imply a 4–5x application gain.

For a hypothesized fivefold speedup, if baseline mean end-to-end time per
system is `T0`, unchanged mandatory work costs `F` per system, new shared
setup costs `S` over `K` solves and new numerical work costs `I1` per system,
the necessary condition is `F + S/K + I1 <= T0/5`. This is an
accounting condition, not a speedup result. A simple competing method is
warm-start plus a tuned standard preconditioner; strong controls include
[PETSc KSP](https://petsc.org/main/manual/ksp/) and
[hypre BoomerAMG](https://hypre.readthedocs.io/en/stable/solvers-boomeramg.html).
Any numerical proposal must report setup, solve, memory, true residual,
matrix-family shift and failures, and beat the best applicable strong control
on untouched sequences. Plain unpreconditioned CG alone is an inadequate
comparison. Published success on particular PDE families would not establish
universal superiority or a new theorem.

**Next cheap kill-test:** identify one real open sequence with stable provenance
and an application owner, then measure the fraction of baseline time spent in
mandatory work, setup and iterations using existing libraries. If mandatory
work alone costs more than `T0/5`, retire the 5x target for
that workload before inventing an algorithm. No such workload or measurement
has been obtained here; competitive potential and mathematical novelty remain
UNKNOWN. A different future contribution would need a precise theorem or a
replicated end-to-end advantage over recycling/AMG, not a renamed warm start.

Top risks across the two tracks: (1) comparison to weak baselines; (2) leaked
or incomplete public agent trajectories and incompatible outcome labels;
(3) stage-only speedups mistaken for end-to-end gains. No new code, dataset,
benchmark or customer contact occurred. Verified bibliographic metadata and
qualification limits are indexed in
[the literature record](literature/dual_track_20260928.json).

---

## Problem-first cross-domain screen — 2026-09-28

The owner asked for a deeper search beyond known solver/agent niches: which
customary representation or intermediate step could be the wrong tool for the
actual job? The [cross-domain map](PROBLEM_FIRST_DISCOVERY_20260928.md) records
nine jobs, closest primary-source prior art, cheap falsifiers and a ranked
qualification path. This is **Mode A / NARROW**, not a build handoff. The direct
scan-to-G-code idea is already published; generic test selection, notebook-state
repair and figure-to-code also have direct prior art. The three retained
qualification tracks are domain-specific computational-claim verification,
physical validity of direct scan-to-print, and one target-specific scientific
computation workload. None has user demand, novelty, or speedup evidence yet.
The next task is to qualify joinable public claim/evidence/outcome artifacts
before writing a new verifier; Ising Engine remains paused.

---

## SWE-bench artifact-source intake: selection freeze — 2026-09-29

**Mode B, read-only artifact audit.** Question: do public SWE-bench Verified
submissions yield real, joinable `(agent final claim, submitted patch,
independent outcome)` cases for a future false-`DONE` study? This is an intake
check, not a benchmark of an agent or a new verifier. Stop after six selected
cases; do not substitute favorable cases for missing artifacts.

Selection was frozen before opening the chosen submissions' metadata, results,
trajectories or test output. Source registry:
[`SWE-bench/experiments`](https://github.com/swe-bench/experiments) at Git commit
`40f164d5b8f1d249bf95a6df8b74b577fd8e519d`. The GitHub contents API
listing of `evaluation/verified` at that ref had 14 directories whose names
begin `2026`; raw JSON SHA-256 was
`bc68cf6441d086c5a9f741b0d96a7d836b6ef996f5685e2a609c1b52e515a157`.
Rank those 14 names by binary SHA-256 of their UTF-8 name and take the first
two, without replacement:

1. `20260219_mini-v2.0.0_gpt-5-2-codex`
2. `20260217_mini-v2.0.0_claude-4-5-haiku-high`

For each entry, read `metadata.yaml` and follow only its pinned public asset
repository. From its `all_preds.jsonl`, rank unique `instance_id` values by
binary SHA-256 of UTF-8 ID; take the first three. A missing/inaccessible asset
or fewer than three predictions is recorded as failure, with no replacement.
The selection script may parse patches to obtain IDs but must print only IDs
before cases are frozen; it must not use outcomes to select IDs.

For each of six cases record whether the following are present and joinable by
the same instance ID: (C) an explicit final agent success/completion claim in
the original trajectory, not an inferred claim from a patch; (P) the actual
submitted patch; (O) official pass/fail outcome plus test output sufficient for
the official artifact regrader; (I) original task identity/version; (L) public
access and stated reuse terms. Record URL, content hash, missingness and exact
type of each field. An official result replaying its own test log is weaker
than fresh independent execution; report this distinction. Do not label a
case "false DONE" merely because O fails if C is absent or ambiguous.

**Source-admission rule:** at least four of six must have C, P, O and I; L is
reported separately. Otherwise this source is NOT QUALIFIED for the planned
30-case false-`DONE` experiment. Even 6/6 would show only corpus feasibility,
not a product gap: [the official `swebench submit verify`](https://github.com/swe-bench/experiments)
already regrades recorded test output. The strongest simple baseline is reading
the official verdict; any future verifier must catch an additional material
failure at acceptable cost. This intake neither runs a new agent nor searches
for a favorable false-success example.
