---
id: cd005-q-protocol
kind: research-protocol
status: binding
authority_scope: CD005-Q equal-time MIS qualification
created: 2026-09-25
immutable: true
---

# CD005-Q: held-out equal-time MIS qualification

## Prior access and source

Before this protocol, we inspected existing CD005 and QOBLIB code, the names,
byte sizes and Git blob IDs of QOBLIB's instance directory, and found no local
references to the six selected names. We did not read their graph contents or
run either solver on them. The four locally evaluated QOBLIB MIS graphs and SK
benchmark cases are calibration only. Upstream source is QOBLIB commit
`2b400f43c197bb0eb9bc9802efa2b28b818ab63c`, path
`07-independentset/instances/`. Fetch exact bytes and verify the Git blob SHA-1
(`sha1("blob <length>\0" + bytes)`) before use. Do not substitute changed files.

| Index | Filename | Git blob SHA-1 |
|---:|---|---|
| 0 | `C125-9.gph` | `8a4d512bc71a16a458b6aae3f2820cf6bd04127d` |
| 1 | `brock200-2.gph` | `d1141099281fdd4ba084cee7c7ec7962d9e6cc96` |
| 2 | `hamming6-4.gph` | `5448b16bfe4b459aa5e4643545d0083f410707aeb798` |
| 3 | `sloane_1zc_128.gph` | `9388d86f8183ceb0baf24e76fb09192f84bf7d89f` |
| 4 | `johnson8-4-4.gph` | `14378e8d32d4e0d04349a1e72a94c447cebe329` |
| 5 | `football.gph` | `a71393c17349b56abacbc9035aab65712e856fae62` |

## Model and arms

Parse DIMACS `p edge N M` / `e u v`, reject malformed, duplicate or out-of-range
edges. Use the repository's existing MIS QUBO convention: linear `-1`,
undirected edge penalty `+2`, represented symmetrically in `CsrMatrix`. Both
arms call `UltimateSolver::new(2.5, 0.05, 10, 5, Some(seed)).solve(model, &[])`;
candidate adds `.with_2opt(true)`, baseline `.with_2opt(false)`. Do not add
repair, tabu, new incumbent, or different preprocessing to either arm.

For each graph run three independent campaign indices `0..2`. For graph index
`g`, campaign `c`, solve number `k` starting at zero, use seed
`5_000_000 + 100_000*g + 10_000*c + k`; arms share this seed sequence. Run
arms sequentially; candidate first iff `(g+c)%2 == 1`. Parse/build the model
before timing. Each arm has 5.000 seconds from its first solve start. Start a
solve only while time remains; count its solution only if it finishes at or
before the deadline, and record a late completion separately. If an arm has no
counted solution, record the cell as failure, not an exclusion. Record every
cell and all counted solution bitstrings, sizes, model energies, durations,
completed solve counts and overrun. No retries or instance exclusions.

## Primary decision and secondary measures

The independent MIS checker counts selected vertices and collisions for each
solution. Every counted solution must have 0 collisions, valid binary spins,
and model energy exactly `-size` within `1e-9`. Primary paired outcome is
`best_candidate_size - best_baseline_size` among solutions returned inside the
same deadline in each of 18 graph-campaign cells. Report wins/ties/losses,
per-graph differences and all raw rows. The pilot earns a practical **GO** only
if all 18 cells are valid, candidate wins at least five cells across at least
two graph names, and loses at most one cell. Otherwise the decision is NO-GO
for a demonstrated equal-time advantage in this pilot; a tie is not proof of
equivalence. The gate is deliberately practical, not a p-value or universal
algorithm-selection claim.

Secondary: best size, number of completed solves and late completions by arm;
elapsed wall time and signed QUBO energy; whether same-seed candidate solutions
improve on the corresponding baseline when both complete. No threshold or
graph rule will be tuned from these rows. Calibration may check parsing,
feasibility, run duration and output format on existing local QOBLIB graphs,
but must not access the six holdout graph contents. Any required change after
calibration needs a committed prospective amendment before holdout access.

Freeze the protocol before instrument code/data. Freeze code and analysis
before downloading or running the six graph files. Save raw outcomes separately
from the evaluated commit, then report environment, commands, hashes and any
failure. The result is about CD005 as a finishing operator in this solver,
not tensor-network quantum dynamics, LABS, or Laya residual search.
