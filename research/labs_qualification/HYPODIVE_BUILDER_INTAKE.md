# LABS-Q001 intake and dependency map

Mode C: qualification evidence. Objective: determine repeatable known-optimum
recovery at an equal finite budget against an established specialist.

Read-only dependency map: src/bin/qoblib_labs_challenge.rs owns LabsState (line50),
local_search_1opt (128), solve_labs_pt (148), and demo main (251).
Targeted src/tests search found no external callers. Existing binary alone uses
rand/rand_chacha/rayon/serde_json already in root Cargo.toml. No public library
types change. benchmarks/qoblib/check_labs.rs is the independent upstream CLI.
External GA.c updateBestIndividual -> RunGA -> TabuSearch; LABS.c owns scratch
energy; timer.c owns internal timing. Qualification bypasses internal time limits
with a common external deadline; logging patch changes no search decision.

Plan: 1 freeze protocol; 2 fix swap sign, add observer/CLI and numerical tests;
3 add immutable external source snapshot plus logging patch/build instructions;
4 implement supervisor, independent verifier and analysis; 5 run gates and
independent review; 6 freeze and execute once; 7 publish evidence and handoff.

Do not touch production solver families, core/compiler, Laya, other research,
Cargo dependencies, existing immutable protocols, or external solver search math.
The only prototype algorithm change is the documented acceptance-sign bug fix.
Success means completing all60 cells with transparent positive or negative outcome.
No claim of a world record, universal advantage, or latest-best specialist.
