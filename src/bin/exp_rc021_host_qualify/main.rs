//! RC-021 — host and timing-instrument qualification.
//!
//! Implements `research/PREREG_RC021_HOST_INSTRUMENT.md` as amended by
//! `PREREG_RC021_AMENDMENT_1.md` and `PREREG_RC021_AMENDMENT_2.md`.
//!
//! **This binary is under construction.** Commit 2 of the §12 plan provides the
//! journal and row-codec layer only. No CLI, manifest, provenance, host
//! measurement, seed, protocol, control, decision, finalize or verify path
//! exists yet, and none may be added outside its own commit.

mod journal;

fn main() {
    println!(
        "RC-021 host qualification — journal layer only (commit 2 of the §12 plan).\n\
         No CLI, no measurement, no data. Nothing is run by this binary yet."
    );
}
