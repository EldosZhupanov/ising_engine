//! RC-021 — host and timing-instrument qualification.
//!
//! Implements `research/PREREG_RC021_HOST_INSTRUMENT.md` as amended by
//! `PREREG_RC021_AMENDMENT_1.md` and `PREREG_RC021_AMENDMENT_2.md`.
//!
//! **This binary is under construction.** Commit 3 of the §12 plan adds the
//! host, manifest and provenance layers on top of the journal. There is still
//! no CLI, no measurement, no seed use, no protocol, no control, no finalize
//! and no verify path, and none may be added outside its own commit.

mod host;
mod journal;
mod manifest;
mod provenance;

fn main() {
    println!(
        "RC-021 host qualification — journal, host, manifest and provenance layers\n\
         (commit 3 of the §12 plan). No CLI, no measurement, no data."
    );
}
