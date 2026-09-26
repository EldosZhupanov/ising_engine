# LABS-Q002 — memetic hunter NOT QUALIFIED

Date: 2026-09-26. Status: completed, integrity-valid. The registered operational
qualification hypothesis failed. This is a scoped empirical result, not a proof
that this search can never improve a best-known value at a larger N.

The full [protocol](protocol.md) was committed at `524b61d`; the reviewed
instrument was frozen at `6668e76024365d5aeb4b67f50cb4caf4fe5d55ef`.
All raw qualification and smoke witnesses/logs were preserved at `7449724`.
No registered seed was rerun, replaced or excluded, and no budget was extended.

## Result at the registered budget

Each arm used one search worker and the same external ten-second deadline,
including initialization and logging. All ten seeds 740001..740010 were run
at each of N=40,50,60, alternating arm order. Known optima were 108,153,218.

| N | Hunter optimum hits | lMAts optimum hits | Hunter median E (range) | lMAts median E (range) | Hunter wins / ties / losses |
|---|---:|---:|---:|---:|---:|
| 40 | 4/10 | 10/10 | 116 (108–116) | 108 (108–108) | 0 / 4 / 6 |
| 50 | 0/10 | 8/10 | 173 (161–201) | 153 (153–161) | 0 / 0 / 10 |
| 60 | 0/10 | 0/10 | 280 (258–290) | 238 (222–258) | 0 / 0 / 10 |

Qualification required >=8/10 hits at **each** length. The hunter fails all
three screens; lMAts also fails the N60 optimum-hit screen. Across 30 paired
endpoints the hunter has 0 wins, 4 ties and 26 losses. No candidate-superiority
gate is met. This does not establish which component caused the gap.

| N / arm | Mean E | Sample SD | Q25 / Q75 | IQR | Wilson 95% interval for hit rate |
|---|---:|---:|---:|---:|---|
| 40 / hunter | 112.8 | 4.1312 | 108 / 116 | 8 | [0.1682, 0.6873] |
| 40 / lMAts | 108.0 | 0.0000 | 108 / 108 | 0 | [0.7225, 1.0000] |
| 50 / hunter | 175.4 | 14.0095 | 163 / 183 | 20 | [0.0000, 0.2775] |
| 50 / lMAts | 154.6 | 3.3731 | 153 / 153 | 0 | [0.4902, 0.9433] |
| 60 / hunter | 278.4 | 10.7414 | 274 / 288 | 14 | [0.0000, 0.2775] |
| 60 / lMAts | 239.6 | 11.8058 | 234 / 248 | 14 | [0.0000, 0.2775] |

The secondary exact Wilcoxon two-sided p-values are 0.03125, 0.001953125,
0.001953125 for N40/50/60; Holm-adjusted values are 0.03125, 0.005859375,
0.005859375. Median paired relative gains for the hunter are negative:
−7.41%, −10.46%, −19.32%. These tests have the symmetry/sign-exchangeability
assumption stated in the protocol; the registered candidate-superiority
criterion is not reversed post hoc into a general claim about lMAts.

Every non-hit is right-censored at ten seconds. The registered fixed-budget
restart TTS99 plug-in estimates are 100s for hunter/N40, 10s for lMAts/N40 and
30s for lMAts/N50. All zero-hit cells have undefined/infinite estimates
(`null` in JSON). These small-sample estimates are not 99% success guarantees,
not censored mean runtimes and not a hardware-independent speed comparison.
All energies, exact hit timestamps, relative gaps and distributions are in
`run/summary.json`.

## Integrity and provenance

- 60/60 planned cells present; zero launch, witness, checker or deadline errors.
- All 881 emitted incumbents passed independent direct integer autocorrelation
  evaluation; all were received before their applicable deadline/stop.
- All 60 final saved sequences passed the separate retained QOBLIB checker:
  22 exit 0 (optimal), 38 exit 20 (valid suboptimal), with matching energy.
- Maximum recorded deadline-stop time: 10.001953652s, within the 0.1s
  termination tolerance. No late witness received credit.
- Four excluded smoke cells (N20, seeds 820001/820002) all reached E26 and
  passed their checker. `smoke/environment.json` records the separate design.
- `SHA256SUMS` covers 132 raw artifacts (run and smoke rows, witnesses and
  environment/completion manifests); derived `run/summary.json` is excluded.
- Current analysis reproduced the saved summary byte-for-byte. It checks
  source blobs against the evaluated Git commit and requires completion to
  match the same freeze.
- Independent read-only review repeated direct energy evaluation of all 881
  main and 26 smoke incumbents, reran all 64 final-witness checkers, verified
  source/baseline provenance and independently recomputed statistics. Verdict:
  PASS for integrity, NOT_QUALIFIED for the candidate. No search was rerun.
- lMAts logging neutrality was independently rerun: four seeded returns and
  evaluation counts matched its uninstrumented source; eleven incumbents were
  logged before the first return. See `reference_validation.json`.
- Pre-data gates: seven Rust hunter tests (including exhaustive flip deltas,
  fixed-step observation neutrality and checkpoint tests), eight new Python
  tests, eight retained supervisor tests and all required root Cargo gates
  passed. The source instrument passed independent review before freeze.

The observed host was WSL2 Linux on the exposed AMD Ryzen 7 170 CPU, four
logical CPUs; each search used one worker with no affinity. Initial one-minute
load was 0.2505, with per-cell starting load ranging to 1.5034. Root release
flags included native CPU/+avx2,+fma; environment RUSTFLAGS was unset.
`run/environment.json` retains full CPU/compiler/config provenance and binary
SHA-256 hashes. Other host activity and virtualization limit timing transfer.

## Reproduce the audit, without rerunning the frozen campaign

From the repository root:

```bash
(cd research/experiments/labs_q002 && sha256sum --check SHA256SUMS)
python3 research/experiments/labs_q002/campaign.py analyze research/experiments/labs_q002/run
python3 -m unittest discover -s research/experiments/labs_q002 -p 'test_*.py'
cargo test --release --bin labs_record_hunter
```

Each cell's JSON contains its exact command, stdout/stderr, events and checker
output. The frozen invocation was `campaign.py run --memetic
target/release/labs_record_hunter --lmats
/tmp/labs-q002-baseline-20260926/solvers/lMAts-lRRts/src/lMAts --checker
target/release/check_labs --output research/experiments/labs_q002/run`.
The reference can be rebuilt from the retained archive with
`research/labs_qualification/build_reference.py` into a fresh directory.

## Decision and next question

**NO-GO for recommending a long record campaign from this prototype's current
evidence.** Preserve the implementation and the negative result. Do not describe
the 16-point N74 gap as evidence of likely record success, or infer that adding
more named operators will close it.

The next useful step is a separately scoped diagnosis of where the candidate
spends its search budget and which operator, if any, gives a benefit at matched
cost on new seeds. A profile or ablation requires its own question and protocol;
these opened qualification seeds must not become a tuning/test feedback loop.
No production solver, MSC/scalar family, Laya or earlier LABS-Q001 source was
changed by this campaign. Longer budgets, multiple workers, latest-specialist
comparisons and mechanism attribution remain untested.
