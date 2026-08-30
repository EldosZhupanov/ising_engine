# RC-021 — instrument-invalid closure record

**Status:** RC-021 is **closed without a scientific verdict**. The allocated run
reached the mandatory control phase, control `C10` failed, and the canonical
closure classified the run as **Class I `INSTRUMENT-INVALID`, exit 3**. No
qualification session began and no Class II statement about the host is
licensed.

This record is the durable publication of the abort required by
`PROJECT_PLAN.md` Stage S2. It does not amend the protocol, reinterpret the
failure as a null, or permit a retry. Any repaired or replacement instrument
requires a fresh pre-registration.

---

## 1. Frozen identity and chronology

| field | value |
|---|---|
| run UUID | `0d9588e6eef8e7b3afe880da93fd5121` |
| run directory | `/home/eldos/ising_engine/experiments/rc021` |
| instrument commit | `e9a0450829c553d95e023fa8173ff58683c97d59` |
| pre-registration | `b09092334e4c1c4505f931484680049c216658f3` |
| amendments | `c895e812b94263e035ec20b89033e601ed416190`, `76853829df368d413cebd89a722f31c611448418` |
| instrument birth | `d8848831b8714b724df5404359aebcf5d4ea0967` |
| boot ID | `252682b9-5979-45da-87c5-56b68ca2c28e` |
| host fingerprint | `85d19fd3ab892a47f25d95281088de376dfdf60e34fae47bf6b2c38325d9427b` |
| run start | `2026-08-30T05:48:07Z` |
| finalized | `2026-08-30T05:48:58Z` |

The run was initialized once, entered `--controls`, and was finalized after the
terminal control failure. The closure is complete. `controls_complete.json` is
absent by design because the control sequence did not pass.

## 2. Control outcome

Controls `P6`, `P5`, `P4`, `P3`, `P2`, `P8`, `N2`, `N1`, `N3`, `P1`, and `P7`
passed. The twelfth control failed:

```text
C10  FAIL  diagnostic overhead 0.010802221586322025 exceeds 0.01;
           diagnostics are removed, not tolerated
```

The terminal mapping is the frozen §C11.25 rule: a mandatory non-P2 control
failure is `INSTRUMENT-INVALID`, exit 3. The closure therefore carries null
`verdict_rules`, `failures_total`, `failures_by_session`, and `leave_one_out`.
All six `session_states` are `NOT STARTED`; `rc021_observations.tsv` and
`RC021_RESULTS.md` are absent, as §C13.8 requires for Class I.

## 3. Durable evidence inventory

The canonical run directory is gitignored operational evidence. These digests
bind this immutable record to the exact surviving bytes:

| path | bytes | SHA-256 |
|---|---:|---|
| `run.json` | 1343 | `40c80bedd1e5df57049b5fe1eb037ab06f7c68ce596f3e6b4ec0b6263152161f` |
| `controls_started.json` | 373 | `1f292813a7385a3019f90ca24703cddae27cea9431e8df3a264ef3d3f42e98b2` |
| `control/rc021_control_journal.tsv` | 2456 | `e4cd7067192b457e834437f80555be41330d4271f642a1f4f2b24cd2ab724aeb` |
| `control/rc021_journal_p2.tsv` | 2005 | `679be2ac61d1953c5332f8927df5095c035fcba8c05ec87504258c6baaf1b17d` |
| `control/rc021_journal_n3.tsv` | 6511 | `9afa421dea6118cb7300e82e1d830c594be0c09a308c30d048f399cf513175c9` |
| `rc021_closure.json` | 5856 | `d9566133a13e4477f2da04906b03354d7e0ba8a97dedd2dbc9161a9169e4975f` |

The closure's own fifteen-entry integrity inventory records the first five
digests and every expected `MISSING` path. The closure cannot contain its own
hash; the final row above supplies that external binding.

## 4. What survives

- The instrument implementation and its synthetic test suite survive as an
  engineering asset.
- The control failure is evidence that this instrument/configuration did not
  qualify as a wall-time measuring station.
- No RC-021 paired-spread datum exists: the six qualification sessions never
  started.
- No claim about `UltimateSolver`, operator quality, marginal cost, or the
  host's true pass probability follows.
- RC-021 is not retried. A future wall-time qualification is a fresh cycle with
  a prospective protocol.

## 5. What this record does not do

It changes no threshold, control, status mapping, seed, or binding document. It
does not call the Class I failure `HOST-NOT-QUALIFIED`, does not publish a
scientific results Markdown, and does not license performance claims from later
quality-only research.
