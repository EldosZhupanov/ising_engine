# REPRO-Bench public-claim source intake — frozen selection

Date: 2026-09-29. Status: preregistered **source qualification**, not a
reproduction experiment, product validation, or comparison of agents. Ising
Engine remains paused. This protocol must be committed before opening selected
case labels, papers, code, data, or outcomes.

## Question and source freeze

Can a small public REPRO-Bench sample supply joined artifacts for a future
study of independent computational-claim checking: original paper, runnable
code/data, a concrete numerical claim, and an independent outcome more
specific than a binary reproducibility label?

Source revisions: Hugging Face dataset
[`chuxuan/REPRO-Bench`](https://huggingface.co/datasets/chuxuan/REPRO-Bench)
at commit `74958fba32daeacff51f2e1fd37be3916595648f` and official
[`uiuc-kang-lab/REPRO-Bench`](https://github.com/uiuc-kang-lab/REPRO-Bench)
at commit `75f112f9671be5b076f757403081a7fd2dda0c24`. Before selection, only
repository-level metadata and filenames were read: the dataset API lists
112 numbered task directories and 112 `should_reproduce.txt` files, and the
official repository exposes `ground_truth.json`. No selected-case content or
outcome was read. The Hugging Face card reports approximately 183 GB total;
the dataset viewer has a format error, so metadata/API reads are the access
path for this intake.

Eligible IDs are decimal strings `1` through `112`. Sort by binary SHA-256 of
the UTF-8 ID; take the first five without replacement. Frozen sample:
`55`, `51`, `39`, `49`, `109`. No replacement for unavailable or unfavorable
cases. The ordering rule, not the displayed list, governs any replay.

## Fields and stop rule

For each case record URL, content hash or pinned-revision path, and status:

- **A:** article PDF publicly fetchable; title/DOI can be identified.
- **D:** original data and at least one analysis script publicly fetchable.
- **C:** an exact numerical claim/table/figure in A can be linked to a
  computation in D; a general paper title is insufficient.
- **O:** a public independent reproduction report or equivalent claim-level
  outcome that names the same result. `should_reproduce.txt`, aggregate gold
  labels, and agent self-reports alone do **not** satisfy O.
- **E:** enough environment and run instructions to attempt an ordinary clean
  rerun, including required software and entry point.
- **L:** stated reuse terms for the benchmark and original case materials,
  reported separately; public download alone is not a license.

First inspect pinned root gold/schema and the five case directory listings
for paths to independent reports. A report may be external, but it must be
linked unambiguously to the selected case. If **zero of five** have plausible
O, stop the intake without downloading case PDFs/data or executing third-party
code: this source is **NOT QUALIFIED for a claim-level independent-outcome
corpus**. Otherwise inspect all five cases with at most 20 MB per downloaded
file and 100 MB total, recording oversized files as inaccessible under this
budget rather than absent. Do not execute case code in this source-intake
stage. A future reproduction study needs a separate protocol and environment.

Admission threshold, if stage two is reached: at least three of five have
A/D/C/O/E joined on one exact claim. L is an additional deployment/reuse gate,
not silently assumed from accessibility. A failure is about this sampled
source and cost envelope, not proof that research reproduction is solved or
that a different domain has no product opportunity.

The strongest cheap incumbent is clean execution of the supplied scripts plus
the existing reproduction report/checklist. A new verifier would have to
detect a material missed discrepancy at comparable access and total cost;
this intake does **not** measure such a gain. Negative/missing evidence stays
in the result with the frozen IDs; no substitute source or handpicked success
may rescue this sample.
