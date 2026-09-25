---
name: statistics
description: Rigorous statistical testing, multi-seed aggregation, and survival analysis for stochastic solvers.
---

# Statistical Analysis Protocol

This skill enforces mathematically sound statistical methods for evaluating stochastic optimization algorithms.

## Multi-Seed Mandate

1. **No Single-Seed Comparisons**:
   - Stochastic metaheuristics (Simulated Annealing, Parallel Tempering, Tabu Search) exhibit heavy-tailed run-time distributions. A single seed provides zero scientific evidence.
   - Every cell must be evaluated across multiple independent seeds (recommended: $N \ge 10$ for initial screening, $N \ge 30$ for definitive claims).

2. **Mandatory Distribution Metrics**:
   For every experiment arm, report:
   - Median and Mean;
   - Standard Deviation;
   - Minimum and Maximum;
   - Interquartile Range (IQR) / 25th and 75th percentiles;
   - Empirical Success Rate: $P_{\text{succ}} = \frac{N_{\text{hits}}}{N_{\text{runs}}}$.

3. **Censored Data & Time-To-Solution (TTS)**:
   - When solvers time out, runs are right-censored. Treating timeout as $t = T_{\text{limit}}$ systematically underestimates the true mean runtime.
   - For reaching target energy within confidence $1 - \alpha$ (standard: $\alpha = 0.01 \implies 99\%$):
     $$\text{TTS}_{99} = t_{\text{run}} \cdot \frac{\ln(1 - 0.99)}{\ln(1 - P_{\text{succ}})}$$
     where $P_{\text{succ}} > 0$. If $P_{\text{succ}} = 0$, TTS is strictly undefined ($\infty$) and must be reported as `> T_budget` or `FAIL`.

4. **Hypothesis Testing**:
   - Use paired non-parametric tests (e.g. Wilcoxon signed-rank test) for paired runs on identical instances and seeds.
   - Use bootstrap confidence intervals (e.g. 95% BCa bootstrap) to demonstrate equivalence or effect size.
   - Never claim superiority when $p \ge 0.05$ or when the effect size falls below the preregistered threshold.
