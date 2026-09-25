---
id: cd003-market-residual-result
kind: research-result
status: closed
authority_scope: cd003-market-residual-structural-gate
created: 2026-09-25
immutable: true
---

# CD003-MR1 result: no boundary-field compression on the fixed Market Split set

**Decision: NO-GO for field-equivalence caching on this ten-instance corpus and
the preregistered split.** The [binding protocol](protocol.md) required at
least three instances with eight or more free variables and at most half as
many distinct boundary fields as raw boundary assignments. All ten rows were
valid; **zero** met the compression threshold. There were no timeouts or
excluded rows. This does not refute the CD003 conditional-response theorem,
nor does it compare solver running times.

| Official instances | Count | Fixed/free after `full_presolve` | Boundary assignments | Distinct fields |
|---|---:|---|---:|---:|
| `ms_03_*`, m=3, n=20 | 6 | 0 / 20 each | 1024 each | 1024 each |
| `ms_04_*`, m=4, n=30 | 4 | 0 / 30 each | 4096 each | 4096 each |
| **Total** | **10** | **0 fixed** | **22,528** | **22,528** |

The existing presolver left a nonempty residual in every case, so the previous
LAYA-001 presolve-saturation problem did not recur. But every tested boundary
assignment produced a different integer cross-field vector for the internal
variables. Grouping boundary contexts by identical fields therefore saves no
conditional internal solves in this exact setup. The presolve timer ranged
from 0.104 to 0.264 ms (median 0.157 ms); these descriptive times are not an
end-to-end speed comparison.

## What was checked

The Rust probe used the unchanged production `full_presolve` on the QUBO
`sum_k(A_k x-b_k)^2`. It checked direct and QUBO energies on eight deterministic
assignments per instance. A separate Python analyzer parsed the official
integer matrices, checked both endpoint energies, validated every fixing and
free-variable index, and enumerated every boundary mask under the frozen
split. The analyzer did not optimize conditional responses. Consequently,
`N_field = 2^b` says that *field equality* gives no compression; different
fields might still share an optimal response. The cost of computing any
conditional optimum also remains unmeasured.

An independent read-only reviewer recomputed all field vectors by direct
integer conditional-energy differences without importing the analyzer,
checked all ten input hashes, protocol and binary hashes, source chronology,
every raw row, and the run manifest. The independent result was 10/10 valid,
0/10 compression hits, with no HIGH/MEDIUM findings. The run metadata reports
`git_dirty=true` because unrelated untracked drafts pre-existed; the evaluated
source paths were clean and frozen.

## Provenance and replay

- Preregistration and scope correction: `62a7d40`.
- Frozen instrument: `e4ea5e262d58f1bc3bf70f4fc79766e3526d9e2b`.
- Raw artifact commit: `ed40229`; one campaign run.
- [Raw output](results/run001/raw.jsonl), [independent analysis](results/run001/analysis.json),
  [metadata](results/run001/metadata.json), [run README](results/run001/README.md),
  and [hash manifest](results/run001/SHA256SUMS).
- Protocol SHA-256: `2129e7de5e091149efbfd3be0862de9f3b68bf9d17066b940b6c86b925afd46f`.
- Raw SHA-256: `7ed2afef803555549fa9409f999daf9bfd6e89dc99334693358662e48dae0ad2`.
- Analysis SHA-256: `00f820fc526f606607dd9e83315ae72b0b1aabb9adbe95b4a0900161d8b6d412`.
- Binary SHA-256: `1af437b303592f7993bdc919d325754f3a0443b4efc35b8c7c358a69cf3e9a99`.

```bash
python3 research/experiments/cd003_market_residual/study.py analyze \
  research/experiments/cd003_market_residual/results/run001/raw.jsonl
cd research/experiments/cd003_market_residual/results/run001
sha256sum -c SHA256SUMS
```

The first command re-analyzes the frozen raw result; it requires no solver
execution. This result closes the Market Split field-cache mechanism on the
selected ten files and fixed partition. Testing a different domain, partition,
or actual conditional response compression requires a new protocol and an
independent equal-cost gate before any production integration.
