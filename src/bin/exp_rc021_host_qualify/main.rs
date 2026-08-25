//! RC-021 — host and timing-instrument qualification.
//!
//! Implements `research/PREREG_RC021_HOST_INSTRUMENT.md` as amended by
//! `PREREG_RC021_AMENDMENT_1.md` and `PREREG_RC021_AMENDMENT_2.md`.
//!
//! **This binary is under construction.** Commit 4 of the §12 plan adds the
//! seeds, the sentinel and the operational `/proc` diagnostics on top of the
//! journal, host, manifest and provenance layers. There is still no CLI, no
//! phase protocol, no warmup or load block, no session, no control, no journal
//! orchestration, no finalize and no verify path, and none may be added
//! outside its own commit.

mod host;
mod journal;
mod manifest;
mod protocol;
mod provenance;
mod session;

fn main() {
    println!(
        "RC-021 host qualification — journal, host, manifest, provenance,\n\
         seeds, sentinel and diagnostics (commit 4 of the §12 plan).\n\
         No CLI, no protocol, no session, no data."
    );
}
