# Scientific Literature Map — Ising Engine

Authority: Verified Bibliographic Landscape  
Purpose: Ground every algorithmic component in established mathematical physics and computer science literature.

---

## 1. Ground State Foundations & Statistical Physics

1. **Sherrington & Kirkpatrick (1975)**: "Solvable model of a spin-glass", *Phys. Rev. Lett.* 35, 1792.
   - Introduced the infinite-range Ising spin glass (SK model).
2. **Parisi (1979, 1980)**: "Infinite number of order parameters for spin-glasses", *Phys. Rev. Lett.* 43, 1754.
   - Replica symmetry breaking (RSB); established asymptotic ground-state energy limit $e_0 \approx -0.76321...$ (proven rigorously by Talagrand 2006).
3. **Bernasconi (1987)**: "Low autocorrelation binary sequences", *Journal de Physique* 48, 559.
   - Formulated the LABS problem as a 4-spin long-range interaction model with golf-course energy landscape.
4. **Mertens (1996)**: "Exhaustive search for low-autocorrelation binary sequences", *J. Phys. A: Math. Gen.* 29, L473.
   - Branch-and-bound and exhaustive limits for LABS up to $N=48$.
5. **Knauer (2004)**: "Heuristische Suche nach binären Sequenzen mit niedriger Autokorrelation", *Dissertation, Universität Bayreuth*.
   - Established the 20-year unproven world records for LABS ($N = 67 \dots 100$).

---

## 2. Exact Reductions & Presolve Theory

1. **Hammer, Hansen & Simeone (1984)**: "Roof duality, complementation and persistency in quadratic 0-1 optimization", *Mathematical Programming* 28, 121–155.
   - Established weak persistency via max-flow duality on quadratic pseudo-boolean posiforms.
2. **Boros & Hammer (2002)**: "Pseudo-Boolean optimization", *Discrete Applied Mathematics* 123, 155–225.
   - Detailed implication network $N_f$ formulation and strongly connected component reduction used in our `qpbo.rs`.
3. **Kolmogorov & Rother (2007)**: "Minimizing non-submodular functions with graph cuts — a review", *IEEE TPAMI* 29, 1274–1288.
   - Modern algorithmic treatment of QPBO and QPBOP probing.
4. **Ishikawa (2011)**: "Transformation of general binary MRF energies to first-order submodular and non-submodular ones", *IEEE TPAMI* 33, 1234–1249.
   - Exact polynomial reduction of higher-order binary terms to quadratic forms.

---

## 3. Stochastic & Population Metaheuristics

1. **Swendsen & Wang (1986)**: "Replica Monte Carlo simulation of spin-glasses", *Phys. Rev. Lett.* 57, 2607.
   - Original replica exchange formulation.
2. **Hukushima & Nemoto (1996)**: "Exchange Monte Carlo method and application to spin glass simulations", *J. Phys. Soc. Jpn.* 65, 1604–1608.
   - Standard Parallel Tempering for spin systems.
3. **Katzgraber, Trebst, Huse & Troyer (2006)**: "Feedback-optimized parallel tempering Monte Carlo", *J. Stat. Mech.* P03018.
   - Round-trip local fraction ladder tuning (implemented in `UltimateSolver::FeedbackOptimized`).
4. **Machta (2010)**: "Population annealing with weighted averages", *Phys. Rev. E* 82, 026704.
   - Population Annealing framework combining resampling with thermal sweeps.
5. **Wang, Machta & Katzgraber (2015)**: "Population annealing: Theory and application in spin glasses", *Phys. Rev. E* 92, 063307.
   - Systematic resampling and post-resample decorrelation invariants implemented in our solver.
6. **Houdayer (2001)**: "A cluster Monte Carlo algorithm for 2-dimensional spin glasses", *Eur. Phys. J. B* 22, 479–484.
   - Isoenergetic Cluster Moves (ICM) implemented in `src/solver/cluster.rs` and `UltimateSolver`.

---

## 4. Benchmark Suites & Competitive Solvers

1. **QOBLIB (2026)**: "Quantum Optimization Benchmarking Library", *Nature Computational Science* (ZIB-AOPT / IBM Quantum Working Group).
   - 10 problem classes establishing standard instances and official solution checkers.
2. **Helmberg & Rendl (2000)**: "A spectral bundle method for semidefinite programming", *SIAM J. Optim.* 10, 673–696.
   - Established the Biq Mac library and semidefinite relaxations for MaxCut.
3. **Lamm et al. (2017)**: "Accelerating local search for maximum independent set", *ALENEX*, 124–137.
   - The KaMIS solver framework (state of the art classical MIS heuristic).
