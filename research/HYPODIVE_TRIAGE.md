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
