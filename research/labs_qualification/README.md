# LABS qualification

Canonical design: [PROTOCOL.md](PROTOCOL.md). Dependency map and scope:
[intake](HYPODIVE_BUILDER_INTAKE.md). No claim that this older specialist is the
best available algorithm in 2026.

The prototype bug was reversed replica-exchange acceptance. For cold/hot inverse
temperatures beta_c > beta_h and energies E_c < E_h, the probability of accepting
the swap is exp(-(beta_c-beta_h)*(E_h-E_c)), not one. The corrected exchange alone
does not imply the entire optimization heuristic is an equilibrium sampler.

The existing demo remains available. Qualification uses the same search function
with a read-only incumbent callback; it runs serial restarts instead of the demo's
parallel restart pool. Both measured methods have one search worker. C defaults
and search logic remain unchanged; the external patch only prints witnesses.

Build from the frozen source checkout:

```bash
cargo build --release --bin qoblib_labs_challenge
python3 research/labs_qualification/build_reference.py /tmp/labs-reference-build
rustc --edition 2021 -O benchmarks/qoblib/check_labs.rs -o /tmp/labs-checker
python3 -m unittest discover -s research/labs_qualification -p 'test_*.py'
```

The reference source archive includes its GPL-2.0 license and RNG library
licenses; it is compiled as a separate process, not linked into Ising Engine.
Archive and logging patch integrity are checked against [reference.json](reference.json).
The original author implementation is from [borkob/git_labs](https://github.com/borkob/git_labs),
commit 1aa123407636e48630174e0bf122037e369c1d68.
Known-optimum targets are listed by [QOBLIB](https://github.com/ZIB-AOPT/QOBLIB/tree/main/02-labs/solutions).

Run once after source freeze (new output directory required):

```bash
python3 research/labs_qualification/campaign.py run \
  --pt target/release/qoblib_labs_challenge \
  --lmats /tmp/labs-reference-build/solvers/lMAts-lRRts/src/lMAts \
  --checker /tmp/labs-checker --output /tmp/labs-q001-run
python3 research/labs_qualification/campaign.py analyze /tmp/labs-q001-run
```

No model downloads or additional dependencies are required. The common supervisor
includes process startup in each 10-second budget, validates integer energies,
retains stdout/stderr and received incumbent histories, kills the process group,
and verifies each final saved witness with the separate QOBLIB checker.
It reports final quality and optimum-hit frequency, not a speedup or a
hardware-independent algorithm ranking. Tiny smoke runs use separate N20 seeds.
