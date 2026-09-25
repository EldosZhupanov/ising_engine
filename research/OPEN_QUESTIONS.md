# Open Scientific Questions — Ising Engine

Authority: Active Research Frontiers  
Status: Live Agenda

---

## 1. Algorithmic & Complexity Frontiers

### Q1: Can Direct Degree-4 HUBO Evaluation Beat Ancilla Binarization at Equal Accuracy?
- **Background**: Standard QUBO solvers binarize cubic ($x_i x_j x_k$) and quartic ($x_i x_j x_k x_l$) terms using auxiliary variables (Freedman 2005, Ishikawa 2011), requiring large penalty coefficients that distort landscape geometry. Our engine evaluates degree-4 terms directly in AVX2.
- **Open Question**: Does direct HUBO evaluation yield faster Time-To-Solution (TTS) than optimal Ishikawa binarization solved by exact MIP/QUBO solvers across dense and sparse polynomial instances?
- **Falsification Metric**: Measure TTS_99 and wall-clock time across degree-3 and degree-4 benchmark problems.

### Q2: Is Breaking Knauer 2004 LABS World Records ($N \ge 67$) Tractable on Classical CPUs via Memetic VNS?
- **Background**: For $N=74$, the gap to the 22-year-old world record ($E=341$) is down to +16 ($E=357$). For $N=70$, gap is +24 ($E=319$).
- **Open Question**: Can an enhanced Variable Neighborhood Search (VNS) with 3-flip / 4-flip perturbation bridges break any world record ($E < E_{\text{BKV}}$) on commodity CPU hardware within 24–48 hours of compute?
- **Success Criterion**: Official ZIB checker `check_labs` outputting $E < E_{\text{BKV}}$.

### Q3: What is the Exact Applicability Boundary of Roof Duality Presolve on QOBLIB?
- **Background**: QPBO achieves 100% persistency on submodular instances, but fixes 0% of variables on dense anti-ferromagnetic graphs (e.g. SK spin glass).
- **Open Question**: For which official QOBLIB problem classes (01 Market Split, 03 Portfolio, 04 Network Design, 06 MaxCut, 07 MIS, 08 QAP) does QPBO eliminate $> 10\%$ of variables in $< 50$ ms?

### Q4: Does Quantum Transverse Field SQA (Trotter Slices) Provide Higher Valley Escape Probabilities than Thermal PT on Finite Graphs?
- **Background**: Simulated Quantum Annealing (SQA) simulates quantum tunneling via Trotter slices.
- **Open Question**: In equal-time CPU comparisons, does SQA (multi-slice PIMC) achieve higher ground-state hit rates on tall-narrow energy barriers than classical Parallel Tempering with feedback-tuned ladders?
