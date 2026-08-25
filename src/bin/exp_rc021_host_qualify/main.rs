//! RC-021 — host and timing-instrument qualification.
//!
//! Implements `research/PREREG_RC021_HOST_INSTRUMENT.md` as amended by
//! `PREREG_RC021_AMENDMENT_1.md` and `PREREG_RC021_AMENDMENT_2.md`.
//!
//! **This binary is under construction.** Commit 6 of the §12 plan adds the
//! twelve controls, both markers and the control journal on top of the journal,
//! host, manifest, provenance, seeds, sentinel, diagnostics, protocol and gap
//! layers. There is still no decision, finalize or verify path (commit 7) and
//! **no public CLI** (commit 8); neither may be added outside its own commit.

mod controls;
mod host;
mod journal;
mod manifest;
mod protocol;
mod provenance;
mod session;

/// §C11.4: the hidden P2 child, and **only** that.
///
/// This is not the CLI. It is a single internal branch whose argument is
/// deliberately unadvertised, because P2 needs a real process that dies after a
/// durable journal write and only this executable can be that process. The five
/// public modes arrive in commit 8 and will parse their arguments separately;
/// this branch takes precedence over nothing, because nothing else parses
/// arguments yet.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == controls::P2_CHILD_ARG {
        p2_child(std::path::Path::new(&args[2]), args.clone());
    }
    println!(
        "RC-021 host qualification — journal, host, manifest, provenance, seeds,\n\
         sentinel, diagnostics, protocol, gap enforcement and controls\n\
         (commit 6 of the §12 plan). No decision, no finalize, no verify, no CLI."
    );
}

/// Build the child's journal metadata from the run directory's manifest, then
/// hand over to the control. Never returns: the control ends in `abort()`.
fn p2_child(dir: &std::path::Path, args: Vec<String>) -> ! {
    // The child's own start instant, in RFC 3339.
    let utc_now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    // §C10.3 names the manifest `run.json`; `read_manifest` takes the file.
    let m = match manifest::read_manifest(&dir.join("run.json")) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("rc021 p2 child: manifest: {e}");
            // A child that cannot even read the run it belongs to must still
            // die abnormally, so the parent's signal check stays unambiguous
            // and P2 fails on the journal it did not leave.
            std::process::abort();
        }
    };
    let meta = journal::Metadata {
        run_uuid: m.run_uuid.clone(),
        boot_id: m.boot_id.clone(),
        run_start_uptime_ms: m.run_start_uptime_ms,
        repo_commit: m.repo_commit.clone(),
        prereg_commit: m.prereg_commit.clone(),
        amendment_commits: m.amendment_commits.clone(),
        instrument_birth_commit: m.instrument_birth_commit.clone(),
        host_fingerprint: m.host_fingerprint.clone(),
        cpu_set: m.cpu_set.clone(),
        thread_count: m.thread_count,
        timer_resolution_ms: m.timer_resolution_ms,
        cpu_time_unit: m.cpu_time_unit.clone(),
        // §C11.4: this invocation's own provenance, not the manifest's.
        command_line: args,
        utc_start: utc_now,
        session: journal::MetaSession::P2,
        diag_availability: journal::DiagAvailability {
            cpu_time: m.diag_availability.cpu_time,
            ctx_switches: m.diag_availability.ctx_switches,
            freq: m.diag_availability.freq,
        },
    };
    let ctx = journal::RowContext {
        run_uuid: m.run_uuid.clone(),
        repo_commit: m.repo_commit.clone(),
        prereg_commit: m.prereg_commit.clone(),
        host_fingerprint: m.host_fingerprint.clone(),
        timer_resolution_ms: m.timer_resolution_ms,
        cpu_set: m.cpu_set.clone(),
        thread_count: m.thread_count,
    };
    // §C11.4: the production source. The hidden child's argv carries only the
    // run directory, so this path is not user-selectable.
    controls::p2_child_main(
        dir,
        &meta,
        &ctx,
        m.run_start_uptime_ms,
        std::path::Path::new("/proc/loadavg"),
    );
}
