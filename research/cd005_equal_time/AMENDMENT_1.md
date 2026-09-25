---
id: cd005-q-amendment-1
kind: research-protocol-amendment
status: binding
authority_scope: CD005-Q equal-time MIS qualification
created: 2026-09-25
immutable: true
supersedes: PROTOCOL.md blob IDs only
---

# CD005-Q2: correct source metadata after aborted fetch

The first CD005-Q1 fetch under frozen instrument commit `e535c93` stopped
before any solver run. `C125-9.gph` and `brock200-2.gph` downloaded and passed
their blob checks. The third file, `hamming6-4.gph`, was downloaded into memory
but rejected before being written: expected blob ID in the original protocol
was incorrectly prefixed by its decimal byte size. No graph content was
printed, parsed or inspected; no solver outcome, graph feature or analysis row
was produced. The failed command and error were disclosed in the conversation
before this amendment. This is prior access to bytes, not an untouched-data
claim.

Requerying the QOBLIB API at the *same pinned upstream commit*
`2b400f43c197bb0eb9bc9802efa2b28b818ab63c` showed that four entries in
the protocol had transcription errors. The corrected Git blob SHA-1 values are:

| Filename | Correct blob SHA-1 |
|---|---|
| `hamming6-4.gph` | `b16bfe4b459aa5e4643545d0083f410707aeb798` |
| `sloane_1zc_128.gph` | `8cc7d2f84a5a77c9920cac476138883efb63a30b` |
| `johnson8-4-4.gph` | `5c6ac520d524519794578501c1331afb9a65925c` |
| `football.gph` | `8413a17349b56abacbc9035aab65712e856fae62` |

The two other blob IDs, six graph names, instance order, model, five-second
budget, 18 paired cells, seeds, arm order, checks, endpoints and GO/NO-GO gate
remain as in the original [protocol](PROTOCOL.md). The aborted Q1 fetch is not
an experimental run. The first complete evaluation must be labeled **CD005-Q2**
in raw rows and result. Fetch/analysis code will be revised and separately
frozen before running the solver. No algorithm or threshold may be tuned from
the downloaded bytes or subsequent results; any further instrument failure
requires another disclosed iteration.
