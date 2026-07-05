# Ising Engine — Roadmap

`C:\Users\Asus TUF\.gemini\antigravity\scratch\ising_engine` | Rust 1.95 stable/MSVC | QUBO/Ising solver: MSC (64 replicas/u64) + Parallel Tempering + Population Annealing.

## Status

| Модуль | Статус |
|---|---|
| engine.rs (MC core, delta_e, step, fast_exp, PT swap) | ✅ работает, но скалярный |
| ultimate.rs (UltimateSolver: MSC+PT+QPA+GNN warm-start) | ✅ |
| parallel_tempering.rs / adaptive.rs / cluster.rs / tabu.rs / autopilot.rs | ✅ скалярные, не трогать |
| CsrMatrix, HuboModel (2/3/4-body), QuboModel legacy | ✅ |
| LogicBuilder (AND/XOR/adder/multiplier→QUBO) | ✅ |
| server_api.rs (Axum, spawn_blocking+rayon, Semaphore, CSR dedup) | ✅ |
| 22 теста, Criterion benches, G-Set (G1/G22/G39/G55) | ✅ |
| ultimate_patch.rs | ⚠️ мёртвый код, дублирует ultimate.rs |
| ethers 2.0.14 (только base_scanner.rs, mempool_scanner.rs) | ⚠️ ~100 лишних зависимостей |

## Блокеры

1. **Windows: `STATUS_ACCESS_VIOLATION` при cargo build** — не баг кода, окружение (Defender/toolchain).
   Fix: `rustup toolchain install stable --force` → exclude `%USERPROFILE%\.cargo`, `%USERPROFILE%\.rustup` из Defender → фолбэк `wsl -d Ubuntu` → `cargo check -j 1`.
2. **Архитектурный скаляр**: `(spin_i >> bit) & 1`, `flip_mask |= 1<<bit`, `f64::from_bits(...)` в hot loops — variable shift/bitcast блокируют LLVM auto-vectorize.

## Roadmap

| # | Фаза | Приоритет | Speedup | Файл |
|---|---|---|---|---|
| 0 | Fix compile | БЛОКЕР | — | env |
| 1 | Byte-per-replica SIMD layout | КРИТИЧЕСКИЙ | 4-7× | types.rs, engine.rs, ultimate.rs |
| 2 | FlatHuboModel (CSR edges) | ВЫСОКИЙ | 1.3-1.8× | core/hubo.rs |
| 3 | Branchless acceptance | СРЕДНИЙ | 1.2-1.5× | engine.rs |
| 4 | Vectorizable fast_exp | СРЕДНИЙ | 1.1× | engine.rs |
| 5 | Bulk RNG pre-gen | НИЗКИЙ | 1.1× | engine.rs |
| 6 | .cargo/config.toml (target-cpu=native+avx2+fma) | ВЫСОКИЙ | бесплатно | .cargo/ |
| 7 | Удалить мёртвый код + ethers gate | НИЗКИЙ | — | cleanup |

### Фаза 1 — детали (единственная нетривиальная)

`types.rs`: `SpinWord=u64` packed-bits → `spins: Vec<i8>` (64 contiguous i8/var = 1 cache line, AVX2-ready).
```rust
pub const NUM_REPLICAS: usize = 64;
pub struct QuantumField { pub spins: Vec<i8>, /* var_base(v,s,t,p) -> index */ }
```
`engine.rs` hot loops: заменить `for bit in 0..64 { (spin_i>>bit)&1 }` на прямой доступ `spins[base+r]` для `r in 0..64` — LLVM векторизует в `vfmadd231pd`.

`ultimate.rs`: init `field.spins[base+r] = rng.gen_range(0..=1) as i8` (вместо `rng.gen::<u64>()`); extract — читать `i8` напрямую, без bit shift.

**Не ломать**: parallel_tempering/adaptive/cluster/tabu — остаются на скалярном QuboModel. Только UltimateSolver+engine.rs используют QuantumField.

### Фазы 2-6 — код (только суть)

```rust
// Ф2: core/hubo.rs — FlatHuboModel
pub struct FlatHuboModel {
    pub linear: Vec<f64>, pub num_vars: usize,
    pub edge2_targets: Vec<usize>, pub edge2_weights: Vec<f64>, pub edge2_offsets: Vec<usize>,
    pub edge3_targets_j: Vec<usize>, pub edge3_targets_k: Vec<usize>, pub edge3_weights: Vec<f64>, pub edge3_offsets: Vec<usize>,
    pub edge4_targets_j: Vec<usize>, pub edge4_targets_k: Vec<usize>, pub edge4_targets_l: Vec<usize>, pub edge4_weights: Vec<f64>, pub edge4_offsets: Vec<usize>,
}
impl FlatHuboModel { pub fn from_hubo(m: &HuboModel) -> Self { /* flatten Vec<Vec<EdgeN>> -> CSR */ } }

// Ф3: branchless accept
let accept = (total_delta <= 0.0) as i8 | (rng_buf[r] < exp_val) as i8;
spins[base_v + r] ^= accept;
energy_slot[r] += total_delta * (accept as f64);

// Ф4: fast_exp без bitcast
fn fast_exp(x: f64) -> f64 {
    if x < -20.0 { return 0.0; } if x > 0.0 { return 1.0; }
    let r = x * 0.0625; let r2 = r*r; let r3 = r2*r;
    let p = 1.0 + r + 0.5*r2 + r3/6.0 + r2*r2/24.0;
    let p2=p*p; let p4=p2*p2; p4*p4
}

// Ф5: pre-gen RNG buffer перед sweep-циклом, индекс v*NUM_REPLICAS+r

// Ф6: .cargo/config.toml
// [target.x86_64-unknown-linux-gnu] / [target.x86_64-pc-windows-msvc]
// rustflags = ["-Ctarget-cpu=native", "-Ctarget-feature=+avx2,+fma", "-Copt-level=3"]
```

## Верификация

```bash
cargo test --release                        # 22/22 pass
RUSTFLAGS='-C target-cpu=native -C llvm-args=-pass-remarks=loop-vectorize' \
  cargo build --release --lib 2>&1 | grep "vectorized loop"
objdump -d target/release/libising_engine.rlib | grep -c ymm
cargo bench --bench benchmark                # ultimate_solver_200_vars ≥2×
cargo run --release --bin gset_benchmark
```

Baseline (WSL, до слома Windows-компиляции): `ultimate_solver_200_vars` 291ms→143.9ms (2.02×), vectorized loops 0→30, ymm refs 0→5967. ⚠️ Текущий код в репо — старая scalar-архитектура, SIMD не закоммичен.

## Файлы (справка)

| Файл | LOC | Роль |
|---|---|---|
| solver/types.rs | 66 | 5D memory layout `[vars][slices][temps][pops][replicas]` |
| solver/engine.rs | 353 | delta_e, step (Rayon par_chunks_mut → PT swap → resampling), fast_exp |
| solver/ultimate.rs | 163 | Qubo→Hubo, init QuantumField, temp schedule, sweep loop, best-of-64 extract |
| core/hubo.rs | 73 | Edge2/3/4, HuboModel (scattered Vec<Vec>), QuboModel legacy |
| bin/server_api.rs | 210 | Axum :8080, POST /api/v1/solve, num_vars≤50000 |
| solver/parallel_tempering.rs | 123 | скалярный PT, sequential Metropolis |
| solver/adaptive.rs | 141 | +Rayon, target accept-rate ~23% |
| solver/cluster.rs | 178 | +Wolff cluster flip каждый 10-й шаг, BFS weight>1.0 |

## Deps (Cargo.toml)

axum 0.8.8, chrono 0.4.44, **ethers 2.0.14 ⚠️→ feature-gate/вынести в отдельный workspace member**, futures 0.3.32, rand 0.8.5, rand_chacha 0.3.1, rayon 1.11.0, serde 1.0.228, serde_json 1.0.149, tokio 1.49.0, tokio-stream 0.1.18.

## После SIMD

- **T1 продукт**: WASM (wasm-pack), PyO3 bindings, импорт .qubo/.bq/DIMACS, CLI (`ising solve --file p.qubo --time-limit 10s`)
- **T2 алгоритмы**: Simulated Bifurcation, multi-start seeds, реальный GNN warm-start
- **T3 бенчи**: G-Set реальные прогоны, сравнение с D-Wave / SimCIM / SB

---

## Промпт для Claude Code

```
Проект: Ising/QUBO solver, Rust 1.95. Путь: C:\Users\Asus TUF\.gemini\antigravity\scratch\ising_engine
Если не компилируется на Windows (STATUS_ACCESS_VIOLATION) — это окружение, не код: cargo check -j 1 или WSL.

Задача: главное ядро (src/solver/engine.rs) полностью скалярное — `(spin>>bit)&1` variable-shift блокирует LLVM auto-vectorize. Нужен byte-per-replica layout (SIMD-first).

По приоритету:
1. types.rs: SpinWord=u64 → spins: Vec<i8>, 64 contiguous i8/переменную. Обновить var_base/get/get_mut.
2. engine.rs: calculate_delta_e_local/calculate_replica_energies_local — прямой доступ spins[base+r] вместо bit-shift; step() — branchless accept + pre-gen RNG buffer; fast_exp — pure f64 polynomial вместо bitcast.
3. ultimate.rs: init/extract через i8 напрямую.
4. core/hubo.rs: добавить FlatHuboModel (CSR-style flat arrays edge2/3/4) + from_hubo().
5. .cargo/config.toml: rustflags target-cpu=native +avx2,+fma для linux-gnu и windows-msvc.
6. Удалить src/solver/ultimate_patch.rs (мёртвый код).

НЕ трогать: ParallelTemperingSolver/AdaptiveTemperingSolver/ClusterSolver (скалярный QuboModel, отдельный путь). Только UltimateSolver+engine.rs используют QuantumField. server_api.rs зовёт UltimateSolver.solve(). Все bin/* должны продолжать компилироваться.

Верификация: cargo test --release (22/22) + loop-vectorize remarks показывают vectorized loops + cargo bench ≥2× на ultimate_solver_200_vars.
```
