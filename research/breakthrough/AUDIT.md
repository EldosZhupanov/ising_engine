# Ising Engine: mathematical and algorithmic audit

Scope: tracked source and locally available branch history at `fdec0df`; September 2026 user-authorized expansion beyond RC-027. This is an algorithm-surface audit, not a claim of line-by-line verification of all 223 Rust and 25 Python files. Declarations, imports, actual call sites and focused implementations were checked; generated data and unrelated website/MEV applications were excluded. Cached remote refs are not a fresh remote census. The declaration maps are [source_files.txt](source_files.txt), [declarations.txt](declarations.txt), and [platform_declarations.txt](platform_declarations.txt). Confidence: High for command-confirmed declarations/call sites, Medium for mathematical interpretation, explicit Unknown for unmeasured costs.

## Objective and representations

The live `core::QuboModel` is **`src/core/hubo.rs:165`**, re-exported by `src/core/mod.rs:5`; `src/core/qubo_model.rs` is not the live declaration. For x in {0,1}, E=c+sum h_i x_i+sum_(i<j) q_ij x_i x_j. Symmetric CSR stores the full pair coefficient twice, and canonical scoring uses 1/2. Flip direction d_i=1-2x_i gives Δ_i=d_i(h_i+sum_j q_ij x_j). Under s=2x-1, J_ij=q_ij/4, b_i=h_i/2+sum_j q_ij/4, and c_Ising=c+sum h_i/2+sum q_ij/4. A MaxCut QUBO has b=0 despite nonzero QUBO linear coefficients. Offset must survive every reduction.

HUBO carries degree 2,3,4 product terms. `HuboModel` is coefficient truth; `FlatHuboModel` is the construction-time contiguous projection (`hubo.rs:37,70,100`). MSC `QuantumField` owns byte-per-replica spins with dimensions variable × 64 lanes × Trotter slices × temperatures × populations, plus f64 energy ledgers (`types.rs:28`). It is no longer packed-u64 spin coding despite historical names. There are also research backends, with a different ownership contract, below `SpinState`.

## Production and scalar solvers

| Surface and code anchor | Mathematical mechanism / execution | Important qualification |
|---|---|---|
| `ultimate.rs:129–225` | QPBO strong labels + first-order/probing weak persistency; disconnected free components solved independently | Existing exact reductions fix values. No conditional degree-2 elimination found. |
| `presolve/mod.rs:54,126,283` | Bounds L_i=h_i+sum min(0,q), U_i=h_i+sum max(0,q); substitute fixed values; probe branches | Re-running identical unconditional persistency is not a new dynamic. |
| `ultimate.rs:225–373` | Random initial configurations; geometric temperature ladder; 64 lanes per cell; swaps; incumbent; 1-opt polish | `num_replicas` field does not change physical lane count. Defaults: 10 temperatures, 1 population, 1 slice. |
| `engine.rs:218,573` | f64 local deltas; p=min(1,approx_exp(-βΔ)); for slices>1 add Trotter neighbor coupling; for one slice quantum delta zero | Numerical fast_exp is approximate. No new exact Boltzmann-sampling claim. Classical energy must be rescored separately from Trotter energy. |
| `engine.rs:605,720` | Rayon outer temperature/population cells, SIMD-friendly 64-lane update; masked swaps with exp((β_i-β_j)(E_i-E_j)) | CPU code; no tracked CUDA/OpenCL/WGSL kernels found. Cell RNG xoshiro seeded from ChaCha; fixed draw ordering matters. |
| `ultimate.rs:572`; `population_annealing.rs` | PA over PT ladders: w∝exp(-Δβ E_cold), ESS=1/sum w², systematic resampling, post-clone relaxation | Optional population mode; tuning overhead must count. |
| `population_annealing.rs:128`; `ultimate.rs:9` | Acceptance-uniform and round-trip-feedback ladder tuning, freeze tuned ladder before production | Temperature feedback is already implemented. |
| `icm.rs:91,161`; `elite.rs` | Houdayer disagreement components conserve pair energy; diverse elite path relinking follows best prefixes | Pairwise/single-slice gating. Both optional in Ultimate, off by default. |
| `local_search.rs:19` | Incremental steepest single-flip descent to a free-variable 1-opt minimum | It does not establish 2-opt or global optimality. |
| `ultimate.rs:91`; `tts.rs:95` | Luby-scaled independent restarts; TTS(q)=t log(1-q)/log(1-p) | p=0 means censored/infinite estimate, not zero time. |
| `parallel_tempering.rs:35`; `adaptive.rs:33`; `cluster.rs:86`; `tabu.rs:10` | Separate per-replica scalar PT, ladder adaptation, bond clusters, tabu | Independent family. No edits authorized as incidental MSC experimentation. |
| `autopilot.rs:48,85` | Random hyperparameter search and multiplicative perturbation around best observations | Not a Gaussian-process Bayesian optimizer despite name. |

`UltimateSolver` re-synchronizes energy every 1000 steps, checks incumbent at exchange boundaries and polishes at return. It exposes no per-flip time-to-target callback; a caller can honestly measure only return-time bounds or independent budget-rung runs. An experiment must not invent fine-grained trajectories through this API.

## engine_v2, including its research platform

`ProblemIR` (`ir.rs:14,35`) owns canonical offset, linear coefficients and ordered deduplicated pair CSR. `SpinState` (`state.rs:75`) exposes Δ ledgers, masked flips, energies, overlap, replica extraction/copy and topology; it does not expose coupling weights. Weight-aware experiments should initially be IR passes, not silently break this interface.

`DenseByteState` (`backends/dense_byte.rs:27`) uses n² f64 coefficients and replica-major byte/field planes; routing cutoff n=4096. `SparseBitSlice` (`sparse_bitslice.rs:20`) packs spins in u64 but retains i64 fields and energy per replica, so memory is not just one bit per spin. Both have reference-backend equivalence tests. Sparse flip update switches strategy by flip density; nominal shape-only work does not measure this effect (RC-005). `boxed_state` chooses the reference fallback when the dense matrix is prohibitive.

The standard registry (`registry.rs:43`) currently registers **20** factories, verified in code rather than copying historical counts:

| Operators | Dynamics already available |
|---|---|
| Metropolis / GibbsColorSweep | Δ-based thermal acceptance / logistic conditional bit probability; chromatic order |
| Greedy / SteepestDescent | deterministic first/steepest local descent |
| RandomFlipSweep / RandomRestartWorst | noise moves / rerandomize worst quarter |
| ExtremalOptimization / ExtremalMetropolis | rank-biased always-move EO (bias 2.5); then neighborhood-restricted thermal repair |
| ReplicaExchange | temperature-indexed exchange |
| HoudayerClusterMove / IsoenergeticClusterMove | disagreement clusters |
| PopulationResample / EliteBroadcast | Boltzmann systematic resampling / copy elite to worst quarter |
| HistoryFieldSweep | accept using Δ+0.5 bias, decay 0.98; reported objective unchanged |
| TabuSweep | 10-sweep prohibition with aspiration on canonical personal best |
| consensus_freeze / consensus_seek | T_eff(i,r)=T_r exp(λ abs(mean_r s_i)); two signs of λ |
| synth_population / synth_dynamics | edge-restricted covariance or temporal co-flip scores; union-find clusters; exact accumulated collective Δ; period 5, up to 8 moves, size≤min(64,n/4) |
| PathRelinkSweep | endpoint-guided best-prefix relinking with incremental deltas |

Runtime (`runtime.rs:418–508`) has acceptance-EMA heating/cooling, UCB operator choice with reward improvement/declared-work, and entropy/diversity-triggered phase skipping. These are not new hypotheses. `scheduler.rs` sequences phases, budgets and repeats. `decision.rs:37,115` measures density, degree CV, clustering, integrality; sparse/integral routing threshold 0.05. Plan synthesis and evolution (`decision.rs:285`, `evolution.rs`) search sequences, budgets and temperature endpoints. They do not currently select exact elimination orders or optimize a multilevel coarse model.

`ai_scientist/executor.rs:124` initializes all replicas to zero, unlike Ultimate. Scientist/theory/proposal/novelty modules search hypotheses and operator compositions; Predictor, Dynamics, World and Policy models rank plans, predict remaining improvement or emit operator sequences. Knowledge graph stores weighted evidence triples; memory OS stores durable experiment records. These are **models of solver behavior**, not a learned conditional distribution over solution variables. See [platform_declarations.txt](platform_declarations.txt) for the inspected dependency surface. RC-010 through RC-020 include audits of leakage, rank invariance, unreachable early-stop behavior, cost identification and invalid instruments; model existence is not a replicated equal-cost advantage over Ultimate.

## Historical ideas and misleading names

Independent read-only Explorer examined workspace research binaries and cached branch differences. `bayesian_scaling_lab.rs:247` already uses per-variable success/attempt ratios to thin proposals. v2 adds burn-in, decay and neighbor activation; v3 attempts memory blending. `ml_hybrid_benchmark.rs:183` and `math_breakthrough.rs:211` already update proposal probabilities using downhill/thermal/rejection rewards. An unspecified “analytical learned-like policy” would repeat these.

`core/anls.rs:20` implements shifted nonnegative matrix factorization by multiplicative updates, called in tests; no solver integration was found. It is not spectral move selection. `src/bin/research_platform.rs:334` computes an absolute-weight spectral diagnostic. `gnn_hybrid_demo.rs` supplies hardcoded probabilities to an empty objective. `gset_official_benchmark.rs` generates a mock graph and hardcodes a Gurobi time; its outgoing-only linear coefficients do not encode its advertised MaxCut graph. Do not use it as a benchmark oracle. The cached HUBO-3SAT branch uses an all-odd XOR instance family satisfied by all-ones; this does not establish a hard SAT regime.

Python files implement benchmark parsers, external adapter/orchestration, source patching and application bridges; there are no tracked GPU kernel files. Non-solver applications and website are inventory exclusions, not unexamined alternative optimizer evidence. Untracked external datasets, remote updates, unpublished vendor results and neural training weights are not audited sources.

## Evidence that must not be repeated or overstated

RC-001 refuted consensus cooling at its registered criterion. RC-002 separated initialization quality from diversity; deterministic descent from equal states collapses replicas. RC-003 and RC-024 found tiny collective-move gains, not compute-normalized breakthroughs. RC-023 found hard vs soft memory indistinguishable. RC-025 and RC-026 examined corpus and embedding depth; additional relinking work explains why naive quality gains cannot be interpreted as speed. RC-027 has frozen code/protocol and pre-existing untracked output: no data were inspected or rerun in this audit.

The immutable [RELATIONAL_PRIMITIVE.md](../RELATIONAL_PRIMITIVE.md) overextends a correct Gibbs-symmetry theorem to every marginal-based optimizer. A prospective derivation is [GAUGE_SCOPE.md](GAUGE_SCOPE.md); the old evidence is untouched. The immutable [AXIOMS_OF_OPTIMIZATION.md](../AXIOMS_OF_OPTIMIZATION.md) §8.9 already proposes elimination/coarse hierarchies and identifies boundary growth as the obstruction; it explicitly does not refute bounded-boundary elimination. The present proposals develop this open mathematical direction rather than rename covariance cluster flips.

## Bottlenecks and discarded information

Measured historical bottlenecks: flip-density-sensitive field maintenance (RC-005), memory locality (RC-013), superlinear relink work (RC-026), and calibration noise (RC-021 diagnosis). Newly identified *candidates*, not measured timing findings: repeated reconstruction of conditionally optimal small subgraphs; failure to use exact two-variable cross-curvature in the 1-opt finisher; absence of gauge-aware basin-conditioned distributions; shape-only allocation ignoring residual eliminable dimension.

The next question is not “which thermostat?” but “how many degrees of freedom truly require stochastic search after exact conditional responses have been removed?” For arbitrary dense frustrated QUBO the answer may still be n. That negative case is deliberately included.
