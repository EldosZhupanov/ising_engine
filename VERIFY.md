# VERIFY.md

## Verification Checklist

Before every merge:

[ ] cargo check

[ ] cargo test --release

[ ] cargo build --release --bins

[ ] cargo clippy --release -- -D warnings

---

Performance changes

[ ] LLVM vectorized loops

[ ] objdump ymm instructions

[ ] cargo bench

[ ] gset benchmark

---

Architecture

[ ] Scalar family unchanged

[ ] MSC family verified

[ ] Public API unchanged

[ ] No ownership violations

---

Correctness

[ ] Numerical results verified

[ ] No energy drift

[ ] Fixed seed comparison passed

