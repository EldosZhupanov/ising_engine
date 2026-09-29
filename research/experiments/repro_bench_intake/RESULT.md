# REPRO-Bench public-claim source intake — bounded result (2026-09-29)

**Decision: QUALIFIED as a source index (4/5), but NOT READY for a matched
verifier experiment.** Four frozen cases join an original numerical result,
its script/data paths, run entry point, and an independent claim-level report.
Only two of those four have their *claim-specific* data beneath this intake's
20 MB/file ceiling. No third-party analysis was executed and no claimed number
was independently reproduced here. This is source qualification, not evidence
that an AI verifier is useful, novel, faster, or wanted by customers.

The immutable [protocol](PROTOCOL.md) was committed as `737c2d4` before case
content inspection. Source revisions: Hugging Face dataset
`74958fba32daeacff51f2e1fd37be3916595648f`; GitHub code
`75f112f9671be5b076f757403081a7fd2dda0c24`. The pinned HF API metadata
response had SHA-256
`a0ba8f1571b197e8f2374e3c00ee23890a2c62d8969b3bfec9261847df91257a`;
the pinned GitHub `ground_truth.json` had SHA-256
`b862c18712eb8da08d5a71387a3b9490b5b56d4053e733234f76cb7cabf2d70a`.
The gold values for IDs 55, 51, 39, 49, 109 were respectively `3, 2, 2, 1,
2`. These aggregate scores were **not** treated as independent claim-level
outcomes. The official [paper](https://arxiv.org/html/2507.18901v1) says
cases were selected for public independent reproduction reports; those reports
are generally external to the HF package.

## Frozen-case field audit

`Y` means sufficient *source evidence* for the protocol field, not successful
execution. `P` means partial/unverified. All five `paper.pdf` files were
downloaded and identified by title/DOI; all five packages had at least one
downloadable data file and analysis script. Each row names a specific claim
and the script that computes it. The HF paths below are relative to the
[pinned dataset revision](https://huggingface.co/datasets/chuxuan/REPRO-Bench/tree/74958fba32daeacff51f2e1fd37be3916595648f).

| ID | A/D/C/O/E | Claim, computation, independent outcome | Practical limit |
| --- | --- | --- | --- |
| **55** | Y/Y/Y/Y/Y | Original AER Table 3: fourth-grade score effect **−0.082**. `Data and Code/Programs/Analysis/Analysis.do`, Table 3 block, reads `s_MainData_StdtLevel2007_2013_MunicSchools.dta`; [I4R DP39](https://www.econstor.eu/bitstream/10419/272772/1/I4R-DP039.pdf), its Table 1, reports **−0.070** from new code. Package README names Stata 16.1 and setup/entry instructions. | Claim-specific data responds to HEAD at **2,832,870,051 bytes**; not downloaded under the cap. The small downloaded Stata file proves package access, not access to these exact observations within budget. |
| **51** | Y/Y/Y/Y/Y | Original AJPS Table 2, “Young” RD effect **−0.084**; `brazil-RD-analysis.R` explicitly builds Table 2 from `brazil-RD-data.dta`. [I4R DP54](https://www.rwi-essen.de/fileadmin/user_upload/RWI/Publikationen/I4R_Discussion_Paper_Series/054_I4R_Kelly_Odermatt_Metson.pdf), Table 5, reproduces **−0.084**. README specifies R 4.0.3, Stata-MP 16.1, packages and script roles. | Brazil data **494,704 bytes** downloaded and hashed; no R/Stata run. |
| **39** | Y/Y/Y/Y/Y* | Original QJE Table I/abstract: post-rally probability of a Black-driver stop **+5.74%**; `Do/Table1.do` estimates `TRUMP_POST_1_30` from `data/stoplevel_data.dta`. The independent report identified in the [I4R report compendium](https://eprints.whiterose.ac.uk/id/eprint/240013/1/I4R-DP287.pdf) reproduces that result but reports **+2.46%**, no longer statistically significant, after excluding multiple-rally counties. `Do/Main.do` and `Readme.pdf` identify the entry point; `.do` and Stata syntax identify the required software. | Claim-specific data responds to HEAD at **10,761,954,340 bytes**; not downloaded. `*` The README does not pin Stata version or package versions, so E supports an *attempt*, not a cleanly fixed environment. The linked OSF full report was not independently fetched; the public I4R compendium supplies the claim-level outcome. |
| **49** | Y/Y/P/P/Y* | `multiracial_jop_final2020.R` reads `multiracial.sav` and computes tables. The [I4R report compendium](https://eprints.whiterose.ac.uk/id/eprint/240013/1/I4R-DP287.pdf) identifies an independent report about software-version breakage, but its accessible abstract gives **no exact numerical result** to join to a paper table. | **Fails C/O join.** The script names R 3.6.2 and archived Zelig versions, but no separate README or locked environment; `*` E is merely attempt-level. Data **11,764,724 bytes** downloaded and hashed. |
| **109** | Y/Y/Y/Y/Y | Original QJE Table III, Panel A, column 3: **0.0191**. `Do Files/CJLR_GreenBooks_QJE_Rep.do` has the Table 3 block reading `Data Files/county_gb_main.dta`; [I4R DP140](https://econpapers.repec.org/paper/zbwi4rdps/140.htm) reports reproduction **0.0263**. README gives the master `.do` entry point and path adjustment. | Claim data **19,735,542 bytes** downloaded and hashed, just below the per-file cap. Stata and user-written packages remain prerequisites; no run. |

The claim-specific data for 55 and 39 are **publicly addressable**, not absent:
HTTP HEAD returned 200 and the byte sizes above. The protocol's source-field D
is therefore Y, but content and actual executability of those exact files
remain unverified in this bounded intake. If D is instead operationalized as
*download the entire claim-specific dataset under 20 MB*, only IDs 51 and 109
pass all five fields (2/5), below the three-case threshold. That stricter
operational gate blocks the proposed cheap matched experiment. No favorable
case was substituted for a large one.

## Checksums and reuse gate

Selected original paper PDF SHA-256 values: 55
`afd24ebe6221689295018d4b34aeb0633024b2ff72f27c56f6ba98bc6395f91a`;
51 `e7406b34dcfe8991ea60d163f96af1b0538629c5bc3f064950389f702908ffc7`;
39 `aafb2d39f391bab3089a8e02d16c4661a689243939626b446abbc6d8b0c0d355`;
49 `dcb002defd85f72d5c5fe443a544dcb28947ca8b58ed17663484fab1afebec21`;
109 `8ff9d21d207b363ebbdca62cb37d6fa50a18475cf65b0c77387ceb004ed69e84`.

Claim scripts SHA-256: 55 `Analysis.do`
`e84ec58e9768d5b51560ada010161496632fa314cd930bcb55bc2f9b4de4cf0f`;
51 `brazil-RD-analysis.R`
`bff6b239278a2b0d4a149a14e26697903507a28edbc39845e221706f8681978c`;
39 `Table1.do`
`e90387aa36cd27e3badc6021c2f4ee53be5d2d27caf2af064c624395cfb74c2b`;
49 `multiracial_jop_final2020.R`
`cd5c741fad30d70e57316b2d1d344445e3a2becf134941f8f17428d186f350f2`;
109 `CJLR_GreenBooks_QJE_Rep.do`
`ca64452b0f29f34ba0808465da29b13d70e15fe56130ad3da66ba1194289ef68`.
Downloaded data SHA-256: 55 ancillary test-score data
`c183a1193548b0ec5a8db95947c8906642ff20d39b7ac9d825d42cab875ff99f`;
51 claim data `2decb7907415fabcd9652d59a5005d086fd2b0f9c1738bf0fd6b90a4d9202cae`;
39 ancillary rally data
`19870c082619ff3bf1b4deda021f508b0b7d9dafffdc856603b35c2d85fd0a67`;
49 claim data `cd6ec9c74cd085fb9c4eeafb52af7d4c051848fbd66684e57717ae8386149fc2`;
109 claim data `e0ebe11b2ca3f288e67d38e0577642c5706ef5c7d89e186abffc0376ae0bae5c`.
Downloaded case PDFs, readmes, scripts, and data stayed below 100 MB total;
no selected file above 20 MB was downloaded. The extracted PDF text and data
bytes were kept in memory, not copied into the repository.

**L remains UNKNOWN/RESTRICTED.** Neither the checked HF dataset card nor the
GitHub root gives a blanket reuse license for the assembled cases. Individual
articles, original-data packages, and independent reports can have different
terms. EconStor's copy of DP39 expressly restricts public/commercial reuse
unless an open-content license applies. HTTP availability and the benchmark's
research use do not establish a right to redistribute or train a product on
the bundled materials. This is a separate deployment gate, not a retroactive
change to A/D/C/O/E.

## Falsification and next gate

The simple incumbent is the original scripts, a clean environment, and the
already published independent reports. This sample shows that claim-level
discrepancies can be indexed, including both reproduced results and
specification-sensitive findings. It does **not** show that another agent can
find a discrepancy the incumbent missed. Manual join work, proprietary Stata,
multi-gigabyte data, and uncertain reuse terms may dominate any automation
benefit.
On this host, `command -v` found neither `stata`, `stata-se`, `stata-mp`,
`R`, nor `Rscript` on `PATH` at intake time. This is a local availability
observation, not proof the packages cannot run elsewhere.

**Next smallest justified test:** on the two under-cap claims (51, 109), first
cost and permission-check an ordinary clean rerun against the published report.
Only with a new frozen protocol and a declared target discrepancy should a
novel verifier be compared at equal total human, compute, software, and data
access cost. If the routine rerun/checklist gives the needed answer at lower
cost, stop this product direction. Buyer demand remains untested; Ising Engine
development remains paused.
