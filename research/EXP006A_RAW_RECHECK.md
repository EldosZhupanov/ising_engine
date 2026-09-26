# EXP-TEN-006A-R raw-data recheck (prospective audit, 2026-09-26)

This record audits the untracked local corpus without editing its protocol,
report, source or raw TSV. **Decision: NO-GO for a claimed active reasoning or
exact-solve advantage on this evaluated synthetic setting.** It does not prove
that all learned energy methods fail.

## Claim, competing explanation and decisive check

The local `research/fundamental_ai/EXP_TEN_006_NOVELTY_REVIEW.md` concludes that
the candidate is mostly inert and classical synchronization is stronger. Its
headline says 4,410 evaluations; the TSV contains **3,360 data rows**. The
cheapest competing explanation for the candidate's apparent advantage over
recurrent attention is that attention degrades while the candidate stays close
to the noisy input. We re-read the TSV independently and reran the exact current
`exp_ten_006_audit` source in an isolated temporary working directory.

The source loops over 3 cycle lengths × 1 noise level × 7 step counts × 20
seeds × 8 methods = **3,360** rows. The current source produced 3,360 rows;
the header and all **14 non-timing fields matched in all 3,361 lines** against
the retained TSV. `wall_time_us` varied as expected. This links the *current*
source to the raw observations. Both source and TSV remain untracked, so their
historical September 12 freeze and evaluated commit are **UNKNOWN**.

| K=16, noise 0.20, T=16; 20 paired seeds | Mean bit accuracy | Rescue | Damage | Exact graphs |
|---|---:|---:|---:|---:|
| Noisy input / zero-interaction ablation | 0.79746 | 0 | 0 | 0/20 |
| Learned-energy candidate | 0.79649 | 0.00716 | 0.00299 | 0/20 |
| Spectral synchronization | 0.89357 | 0.55044 | 0.01840 | 0/20 |
| Loopy min-sum | 0.95098 | 0.84985 | 0.02263 | 0/20 |

The candidate minus zero-interaction paired mean bit-accuracy difference is
−0.000975: 2 seed wins, 11 ties, 7 losses. No practical superiority is shown.
The classical methods improve **partial bit accuracy** substantially; calling
the K=16 graphs solved would be false. Across the entire TSV there are five
positive `exact_solve` rows, all the *same* K=4 spectral seed/graph repeated at
T=2,4,8,16,32. No other method/graph has an exact success in this audit.

## Protocol and reporting limits

The earlier `EXP_TEN_006_PROTOCOL.md` specifies 500 training epochs and a
larger evaluation design. The separate `006A-R` executable trains for five
epochs and evaluates only K=4,8,16 at noise 0.20. Treat this as a later
diagnostic, **not** a completed execution of every original preregistered
condition. The protocol's "mean node accuracy" is described as cosine
similarity; `TrajectoryAuditor::audit_state` computes the fraction of correct
sign bits in `mean_accuracy`. The audit report's formulas and several printed
percentages are damaged. Its central qualitative inert-versus-classical
ordering survives this recheck; its row count and exact-solve implication do
not. No matched wall-time or parameter-count advantage is established here.

## Reproduction and provenance

- Raw TSV SHA-256: `04d317f0a197e063443501476a8f8b0214bf246d288db11b522b61147dceab82`.
- Current audit source SHA-256: `83a52b02c7760a62fa3768e3f886127ff30e3fdbde4447a34e2a54f37e6120b3`.
- Run `python3 scripts/audit_exp006_raw.py` for row integrity, means and unique
  exact successes. Run it on a replayed TSV to compare scientific fields.
- Build with `cargo build --release --manifest-path research/fundamental_ai/Cargo.toml --bin exp_ten_006_audit`.
  Execute the built binary from a fresh temporary directory: it writes
  `EXP_TEN_006_AUDIT_RAW.tsv` into its current working directory. Do not run it
  in `research/fundamental_ai/`, because that would overwrite the retained raw
  evidence.

**Stop condition:** no further model-building iteration on this same synthetic
split based on the failed active-reasoning claim. Reopen only under a separately
frozen protocol with a new assumption, verified source commit, and an exact
success or meaningful partial-recovery endpoint against the strong classical
baselines at matched cost.
