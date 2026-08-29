//! RC-021 — host and timing-instrument qualification.
//!
//! Implements `research/PREREG_RC021_HOST_INSTRUMENT.md` as amended by
//! `PREREG_RC021_AMENDMENT_1.md` and `PREREG_RC021_AMENDMENT_2.md`.
//!
//! Commit 8 of the §12 plan exposes the five frozen modes. `main.rs` owns only
//! argument parsing and orchestration; every scientific rule and durable codec
//! remains in its private module.

mod controls;
mod decision;
mod host;
mod journal;
mod manifest;
mod protocol;
mod provenance;
mod session;

fn main() {
    let args = std::env::args_os()
        .map(|arg| {
            arg.into_string()
                .map_err(|_| "an argument is not valid UTF-8")
        })
        .collect::<Result<Vec<_>, _>>();
    let code = match args {
        Ok(args) => {
            if args.len() == 3 && args[1] == controls::P2_CHILD_ARG {
                p2_child(std::path::Path::new(&args[2]), args.clone());
            }
            match parse_mode(&args) {
                Ok(mode) => dispatch(mode, args),
                Err(why) => {
                    eprintln!("rc021: {why}\n{USAGE}");
                    controls::RunStatus::RefusedBeforeMeasurement.exit_code()
                }
            }
        }
        Err(why) => {
            eprintln!("rc021: {why}\n{USAGE}");
            controls::RunStatus::RefusedBeforeMeasurement.exit_code()
        }
    };
    std::process::exit(code);
}

const USAGE: &str = "usage:\n  exp_rc021_host_qualify --init-run --dir D\n  \
exp_rc021_host_qualify --controls --dir D --run-id R\n  \
exp_rc021_host_qualify --session N --dir D --run-id R\n  \
exp_rc021_host_qualify --finalize --dir D --run-id R\n  \
exp_rc021_host_qualify --verify --dir D";

#[derive(Clone, PartialEq, Eq, Debug)]
enum Mode {
    InitRun {
        dir: std::path::PathBuf,
    },
    Controls {
        dir: std::path::PathBuf,
        run_id: String,
    },
    Session {
        session: u8,
        dir: std::path::PathBuf,
        run_id: String,
    },
    Finalize {
        dir: std::path::PathBuf,
        run_id: String,
    },
    Verify {
        dir: std::path::PathBuf,
    },
}

fn parse_mode(args: &[String]) -> Result<Mode, String> {
    let path = |s: &String| std::path::PathBuf::from(s);
    match args {
        [_, mode, dir_flag, dir] if mode == "--init-run" && dir_flag == "--dir" => {
            Ok(Mode::InitRun { dir: path(dir) })
        }
        [_, mode, dir_flag, dir, id_flag, run_id]
            if mode == "--controls" && dir_flag == "--dir" && id_flag == "--run-id" =>
        {
            require_run_id(run_id)?;
            Ok(Mode::Controls {
                dir: path(dir),
                run_id: run_id.clone(),
            })
        }
        [_, mode, session, dir_flag, dir, id_flag, run_id]
            if mode == "--session" && dir_flag == "--dir" && id_flag == "--run-id" =>
        {
            require_run_id(run_id)?;
            let session = session
                .parse::<u8>()
                .ok()
                .filter(|n| (1..=session::SESSIONS).contains(n))
                .ok_or_else(|| format!("session must be in 1..={}", session::SESSIONS))?;
            Ok(Mode::Session {
                session,
                dir: path(dir),
                run_id: run_id.clone(),
            })
        }
        [_, mode, dir_flag, dir, id_flag, run_id]
            if mode == "--finalize" && dir_flag == "--dir" && id_flag == "--run-id" =>
        {
            require_run_id(run_id)?;
            Ok(Mode::Finalize {
                dir: path(dir),
                run_id: run_id.clone(),
            })
        }
        [_, mode, dir_flag, dir] if mode == "--verify" && dir_flag == "--dir" => {
            Ok(Mode::Verify { dir: path(dir) })
        }
        _ => Err("arguments do not match one frozen public mode".to_string()),
    }
}

fn require_run_id(run_id: &str) -> Result<(), String> {
    if host::is_run_uuid(run_id) {
        Ok(())
    } else {
        Err("--run-id must be exactly 32 lower-case hexadecimal digits".to_string())
    }
}

/// Both measuring modes require an optimised build.
///
/// The guard existed only inside P6's provenance detail, which runs only under
/// `--controls`. Its own message says "RC-021 measuring modes require a release
/// build" — plural — but `--session N`, the mode that produces all ninety
/// scientific timings, had no check at all. An operator could pass `--controls`
/// from a release build and then run the six sessions from a debug one: every
/// journal would bind, every header validate, and `--finalize` would publish a
/// verdict over ninety unoptimised measurements with nothing in the record
/// saying so. The schema is frozen at 24 columns, so the build cannot be
/// recorded per row — it has to be refused up front.
fn require_release_build() -> Result<(), String> {
    if controls::is_optimised_build() {
        return Ok(());
    }
    Err(format!(
        "RC-021 measuring modes require an optimised build ({})",
        controls::build_profile_fact()
    ))
}

/// One wall-clock reading per invocation, so every header this process writes
/// carries the same `utc_start`.
fn invocation_utc() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn report_status(context: &str, status: controls::RunStatus) -> i32 {
    eprintln!("rc021 {context}: {}", status.as_str());
    status.exit_code()
}

fn dispatch(mode: Mode, args: Vec<String>) -> i32 {
    match mode {
        Mode::InitRun { dir } => init_run_mode(&dir, args),
        Mode::Controls { dir, run_id } => controls_mode(&dir, &run_id, args),
        Mode::Session {
            session,
            dir,
            run_id,
        } => session_mode(&dir, session, &run_id, args),
        Mode::Finalize { dir, run_id } => finalize_mode(&dir, &run_id),
        Mode::Verify { dir } => verify_mode(&dir),
    }
}

#[derive(Clone)]
struct LiveFacts {
    host_fields: host::HostFields,
    boot_id: String,
    uptime_ms: u64,
    provenance: provenance::ProvenanceSnapshot,
    thread_count: u32,
}

fn repository_root() -> Result<std::path::PathBuf, String> {
    // The CLI has no working-directory precondition. Anchor provenance to the
    // repository that built this instrument, so invoking the absolute binary
    // from a run directory cannot silently change which Git tree is checked.
    let build_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(build_root)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("git rev-parse --show-toplevel: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let text = String::from_utf8(out.stdout).map_err(|e| e.to_string())?;
    std::fs::canonicalize(text.trim()).map_err(|e| e.to_string())
}

fn live_facts(repo: &std::path::Path) -> Result<LiveFacts, String> {
    let sources = host::HostSources {
        osrelease: std::path::Path::new("/proc/sys/kernel/osrelease"),
        kernel_version: std::path::Path::new("/proc/version"),
        sys_cpu_dir: std::path::Path::new("/sys/devices/system/cpu"),
        meminfo: std::path::Path::new("/proc/meminfo"),
        self_status: std::path::Path::new("/proc/self/status"),
        cpuinfo: std::path::Path::new("/proc/cpuinfo"),
    };
    let host_fields = sources.read().map_err(|e| e.to_string())?;
    let boot_id = host::parse_boot_id(
        &std::fs::read_to_string("/proc/sys/kernel/random/boot_id").map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let uptime_ms =
        host::read_uptime_ms(std::path::Path::new("/proc/uptime")).map_err(|e| e.to_string())?;
    let provenance = provenance::check(repo).map_err(|e| e.to_string())?;
    let thread_count = u32::try_from(rayon::current_num_threads())
        .map_err(|_| "rayon thread count does not fit u32".to_string())?;
    Ok(LiveFacts {
        host_fields,
        boot_id,
        uptime_ms,
        provenance,
        thread_count,
    })
}

fn invocation_observations(cpu_set: &str) -> Result<(f64, host::DiagProbe), String> {
    let cpu_ids = host::parse_cpu_list(cpu_set).map_err(|e| e.to_string())?;
    let reader = host::FsReader;
    let paths = host::DiagPaths {
        self_stat: std::path::Path::new("/proc/self/stat"),
        self_status: std::path::Path::new("/proc/self/status"),
        sys_cpu_dir: std::path::Path::new("/sys/devices/system/cpu"),
    };
    Ok((
        host::timer_resolution_ms(),
        host::probe_availability(&reader, &paths, &cpu_ids),
    ))
}

#[cfg(test)]
fn journal_metadata(
    manifest: &manifest::RunManifest,
    args: Vec<String>,
    session: journal::MetaSession,
) -> journal::Metadata {
    journal_metadata_observed(
        manifest,
        args,
        session,
        manifest.timer_resolution_ms,
        manifest.diag_availability,
        invocation_utc(),
    )
}

/// `utc_start` is the **invocation's** stamp, passed in rather than read here.
///
/// Reading the clock inside meant one `--controls` run could stamp the control
/// journal from one reading and the N3/P2 headers from another, and nothing in
/// `metadata_binds` or `ControlsContext::bind` compares `utc_start`, so the
/// disagreement was silent. Same doctrine as `FinalizeStamp`: one act, one
/// timestamp.
fn journal_metadata_observed(
    manifest: &manifest::RunManifest,
    args: Vec<String>,
    session: journal::MetaSession,
    timer_resolution_ms: f64,
    diag_availability: host::DiagProbe,
    utc_start: String,
) -> journal::Metadata {
    journal::Metadata {
        run_uuid: manifest.run_uuid.clone(),
        boot_id: manifest.boot_id.clone(),
        run_start_uptime_ms: manifest.run_start_uptime_ms,
        repo_commit: manifest.repo_commit.clone(),
        prereg_commit: manifest.prereg_commit.clone(),
        amendment_commits: manifest.amendment_commits.clone(),
        instrument_birth_commit: manifest.instrument_birth_commit.clone(),
        host_fingerprint: manifest.host_fingerprint.clone(),
        cpu_set: manifest.cpu_set.clone(),
        thread_count: manifest.thread_count,
        timer_resolution_ms,
        cpu_time_unit: manifest.cpu_time_unit.clone(),
        command_line: args,
        utc_start,
        session,
        diag_availability: journal::DiagAvailability {
            cpu_time: diag_availability.cpu_time,
            ctx_switches: diag_availability.ctx_switches,
            freq: diag_availability.freq,
        },
    }
}

#[cfg(test)]
fn row_context(manifest: &manifest::RunManifest) -> journal::RowContext {
    row_context_observed(manifest, manifest.timer_resolution_ms)
}

fn row_context_observed(
    manifest: &manifest::RunManifest,
    timer_resolution_ms: f64,
) -> journal::RowContext {
    journal::RowContext {
        run_uuid: manifest.run_uuid.clone(),
        repo_commit: manifest.repo_commit.clone(),
        prereg_commit: manifest.prereg_commit.clone(),
        host_fingerprint: manifest.host_fingerprint.clone(),
        timer_resolution_ms,
        cpu_set: manifest.cpu_set.clone(),
        thread_count: manifest.thread_count,
    }
}

fn init_run_mode(dir: &std::path::Path, args: Vec<String>) -> i32 {
    let canonical = match manifest::preflight_run_dir(dir) {
        Ok(d) => d,
        Err(e) => {
            return report_status(
                &format!("init refused: {e}"),
                controls::RunStatus::RefusedBeforeMeasurement,
            )
        }
    };
    let repo = match repository_root() {
        Ok(r) => r,
        Err(e) => {
            return report_status(
                &format!("init provenance: {e}"),
                controls::RunStatus::RefusedBeforeMeasurement,
            )
        }
    };
    let facts = match live_facts(&repo) {
        Ok(f) => f,
        Err(e) => {
            return report_status(
                &format!("init host: {e}"),
                controls::RunStatus::RefusedBeforeMeasurement,
            )
        }
    };
    let (timer_resolution_ms, diag) =
        match invocation_observations(&facts.host_fields.cpus_allowed_list) {
            Ok(observations) => observations,
            Err(e) => {
                return report_status(
                    &format!("init observations: {e}"),
                    controls::RunStatus::RefusedBeforeMeasurement,
                )
            }
        };
    let run_dir = match canonical.to_str() {
        Some(s) => s.to_string(),
        None => {
            return report_status(
                "init path is not UTF-8",
                controls::RunStatus::RefusedBeforeMeasurement,
            )
        }
    };
    let manifest = manifest::RunManifest {
        run_uuid: host::new_run_uuid(),
        boot_id: facts.boot_id,
        run_start_uptime_ms: facts.uptime_ms,
        run_dir,
        repo_commit: facts.provenance.repo_commit,
        prereg_commit: facts.provenance.prereg_commit,
        amendment_commits: facts.provenance.amendment_commits,
        instrument_birth_commit: facts.provenance.instrument_birth_commit,
        host_fingerprint: facts.host_fields.fingerprint(),
        cpu_set: facts.host_fields.cpus_allowed_list.clone(),
        host_fields: facts.host_fields,
        thread_count: facts.thread_count,
        timer_resolution_ms,
        cpu_time_unit: journal::CPU_TIME_UNIT.to_string(),
        command_line: args,
        utc_start: invocation_utc(),
        diag_availability: diag,
    };
    match manifest::init_run(&canonical, &manifest) {
        Ok(_) => {
            println!("{}", manifest.run_uuid);
            controls::MODE_SUCCESS_EXIT_CODE
        }
        Err(e) => report_status(
            &format!("init refused: {e}"),
            controls::RunStatus::RefusedBeforeMeasurement,
        ),
    }
}

fn path_present(path: &std::path::Path) -> Result<bool, String> {
    match path.symlink_metadata() {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

fn closing_gate(dir: &std::path::Path) -> Option<i32> {
    match decision::closure_path_state(dir) {
        decision::ClosurePathState::Absent => None,
        decision::ClosurePathState::Complete(_) => Some(report_status(
            "closure already complete",
            controls::RunStatus::RefusedBeforeMeasurement,
        )),
        decision::ClosurePathState::Empty | decision::ClosurePathState::Partial(_) => {
            Some(report_status(
                "closure is empty or partial",
                controls::RunStatus::JournalInvalid,
            ))
        }
    }
}

fn read_bound_manifest(
    dir: &std::path::Path,
    run_id: &str,
    damaged_status: controls::RunStatus,
) -> Result<manifest::RunManifest, i32> {
    let manifest = manifest::read_manifest(&dir.join("run.json"))
        .map_err(|e| report_status(&format!("manifest: {e}"), damaged_status))?;
    if manifest.run_uuid != run_id {
        return Err(report_status(
            "run id does not match manifest",
            controls::RunStatus::RefusedBeforeMeasurement,
        ));
    }
    if let Err(e) = manifest.check_run_dir(dir) {
        return Err(report_status(
            &format!("run directory: {e}"),
            damaged_status,
        ));
    }
    Ok(manifest)
}

fn observed_config(facts: &LiveFacts) -> session::ObservedConfig {
    session::ObservedConfig {
        cpus_allowed_list: facts.host_fields.cpus_allowed_list.clone(),
        thread_count: facts.thread_count,
        host_fingerprint: facts.host_fields.fingerprint(),
        repo_commit: facts.provenance.repo_commit.clone(),
    }
}

fn load_g11(repo: &std::path::Path) -> Result<protocol::VerifiedSentinelInstance, String> {
    let path = repo.join("benchmark_suite/data/gset/G11");
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    protocol::load_sentinel_instance(&bytes).map_err(|e| e.to_string())
}

fn controls_mode(dir: &std::path::Path, run_id: &str, args: Vec<String>) -> i32 {
    if let Err(e) = require_release_build() {
        return report_status(&e, controls::RunStatus::RefusedBeforeMeasurement);
    }
    if let Some(code) = closing_gate(dir) {
        return code;
    }
    let run_invalid = session::classify_run_invalid(dir);
    match &run_invalid {
        session::RunInvalidState::Damaged(_) => {
            return report_status(
                "run_invalid is damaged",
                controls::RunStatus::JournalInvalid,
            )
        }
        session::RunInvalidState::Valid(_) | session::RunInvalidState::Absent => {}
    }
    let started = match path_present(&dir.join(controls::CONTROLS_STARTED)) {
        Ok(v) => v,
        Err(e) => return report_status(&e, controls::RunStatus::JournalInvalid),
    };
    let manifest = match read_bound_manifest(
        dir,
        run_id,
        if started || !matches!(&run_invalid, session::RunInvalidState::Absent) {
            controls::RunStatus::JournalInvalid
        } else {
            controls::RunStatus::RefusedBeforeMeasurement
        },
    ) {
        Ok(m) => m,
        Err(code) => return code,
    };
    if let session::RunInvalidState::Valid(record) = &run_invalid {
        return match session::bind_run_invalid(record, &manifest) {
            Ok(()) => report_status(
                "run_invalid already exists",
                controls::RunStatus::InstrumentInvalid,
            ),
            Err(why) => report_status(
                &format!("run_invalid does not bind to this run: {why}"),
                controls::RunStatus::JournalInvalid,
            ),
        };
    }
    for rel in [
        controls::CONTROLS_STARTED,
        controls::CONTROLS_COMPLETE,
        controls::CONTROL_JOURNAL,
    ] {
        match path_present(&dir.join(rel)) {
            Ok(true) => {
                return report_status(
                    "controls already began",
                    controls::RunStatus::InstrumentInvalid,
                )
            }
            Ok(false) => {}
            Err(e) => return report_status(&e, controls::RunStatus::JournalInvalid),
        }
    }
    let repo = match repository_root() {
        Ok(r) => r,
        Err(e) => return report_status(&e, controls::RunStatus::RefusedBeforeMeasurement),
    };
    let facts = match live_facts(&repo) {
        Ok(f) => f,
        Err(e) => return report_status(&e, controls::RunStatus::RefusedBeforeMeasurement),
    };
    if let Err(e) = session::check_configuration(&observed_config(&facts), &manifest) {
        return report_status(
            &format!("configuration: {e}"),
            controls::RunStatus::RefusedBeforeMeasurement,
        );
    }
    let (timer_resolution_ms, diag_availability) = match invocation_observations(&manifest.cpu_set)
    {
        Ok(observations) => observations,
        Err(e) => {
            return report_status(
                &format!("invocation observations: {e}"),
                controls::RunStatus::RefusedBeforeMeasurement,
            )
        }
    };
    let offset = match host::monotonic_offset_ms(
        &manifest.boot_id,
        &facts.boot_id,
        manifest.run_start_uptime_ms,
        facts.uptime_ms,
    ) {
        Ok(v) => v,
        Err(e) => {
            return report_status(
                &e.to_string(),
                controls::RunStatus::RefusedBeforeMeasurement,
            )
        }
    };
    let instance = match load_g11(&repo) {
        Ok(i) => i,
        Err(e) => return report_status(&e, controls::RunStatus::RefusedBeforeMeasurement),
    };
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            return report_status(
                &e.to_string(),
                controls::RunStatus::RefusedBeforeMeasurement,
            )
        }
    };
    // One reading for every header this invocation writes.
    let utc = invocation_utc();
    let meta = journal_metadata_observed(
        &manifest,
        args.clone(),
        journal::MetaSession::Control,
        timer_resolution_ms,
        diag_availability,
        utc.clone(),
    );
    let context = controls::ControlsContext {
        manifest: &manifest,
        command_line: args.clone(),
        meta,
    };
    // One read for the whole invocation. `journal_metadata_observed` stamps the
    // control journal and this value stamps the N3 and P2 headers; reading the
    // clock twice let one `--controls` run write two different `utc_start`
    // values, and nothing in `metadata_binds` or `ControlsContext::bind`
    // compares them. Same doctrine as `FinalizeStamp`.
    let mut live = match controls::LiveEnvironment::new(
        dir,
        &repo,
        &exe,
        &instance,
        &manifest,
        timer_resolution_ms,
        diag_availability,
        args,
        utc,
    ) {
        Ok(v) => v,
        Err(e) => {
            return report_status(
                &e.to_string(),
                controls::RunStatus::RefusedBeforeMeasurement,
            )
        }
    };
    let mut runner = controls::ProductionRunner::new(&mut live);
    let mut clock = controls::LiveClock::new(offset);
    let outcome = controls::run_controls(dir, &context, &mut runner, &mut clock);
    match outcome {
        controls::ControlsOutcome::Complete { .. } => controls::MODE_SUCCESS_EXIT_CODE,
        other => {
            // The same loss `SessionOutcome::describe` and `report_finalize`
            // exist to prevent. `RefusedBeforeMeasurement` writes nothing, so
            // an unprinted reason exists nowhere at all.
            eprintln!("rc021 controls: {}", other.describe());
            other.terminal_exit_code()
        }
        .unwrap_or_else(|| {
            report_status(
                "unclassified controls outcome",
                controls::RunStatus::JournalInvalid,
            )
        }),
    }
}

fn metadata_binds(
    meta: &journal::Metadata,
    manifest: &manifest::RunManifest,
    expected: journal::MetaSession,
) -> bool {
    meta.session == expected
        && meta.run_uuid == manifest.run_uuid
        && meta.boot_id == manifest.boot_id
        && meta.run_start_uptime_ms == manifest.run_start_uptime_ms
        && meta.repo_commit == manifest.repo_commit
        && meta.prereg_commit == manifest.prereg_commit
        && meta.amendment_commits == manifest.amendment_commits
        && meta.instrument_birth_commit == manifest.instrument_birth_commit
        && meta.host_fingerprint == manifest.host_fingerprint
        && meta.cpu_set == manifest.cpu_set
        && meta.thread_count == manifest.thread_count
        && meta.cpu_time_unit == manifest.cpu_time_unit
}

fn controls_anchor(dir: &std::path::Path, manifest: &manifest::RunManifest) -> Result<u64, String> {
    let started_text = std::fs::read_to_string(dir.join(controls::CONTROLS_STARTED))
        .map_err(|e| format!("controls_started: {e}"))?;
    let started = controls::ControlsStarted::parse(&started_text).map_err(|e| e.to_string())?;
    if started.run_uuid != manifest.run_uuid || started.boot_id != manifest.boot_id {
        return Err("controls_started does not bind to the manifest".to_string());
    }
    let complete_text = std::fs::read_to_string(dir.join(controls::CONTROLS_COMPLETE))
        .map_err(|e| format!("controls_complete: {e}"))?;
    let complete = controls::ControlsComplete::parse(&complete_text).map_err(|e| e.to_string())?;
    if complete.run_uuid != manifest.run_uuid || complete.boot_id != manifest.boot_id {
        return Err("controls_complete does not bind to the manifest".to_string());
    }
    let bytes = std::fs::read(dir.join(controls::CONTROL_JOURNAL))
        .map_err(|e| format!("control journal: {e}"))?;
    if host::sha256_hex(&bytes) != complete.control_journal_sha256 {
        return Err("controls_complete SHA does not match the control journal".to_string());
    }
    let read = controls::read_control_journal(&bytes).map_err(|e| e.to_string())?;
    let binding = controls::ControlsContext {
        manifest,
        command_line: vec!["session-preflight".to_string()],
        meta: read.metadata,
    };
    binding.bind().map_err(|e| e.to_string())?;
    if read.rows.len() != controls::CONTROL_COUNT as usize
        || read
            .rows
            .iter()
            .any(|r| r.status != controls::ControlStatus::Pass)
    {
        return Err("the central control journal is not twelve PASS rows".to_string());
    }
    Ok(complete.monotonic_offset_ms)
}

/// Why session `N`'s predecessor fails §C13's "sessions `1..N−1` are
/// `COMPLETED`" precondition.
///
/// The two arms carry different §8 classes and must not be collapsed. A
/// predecessor that has not run is an **operational** refusal: nothing is
/// wrong with the instrument, nothing has been measured, and the operator
/// simply has to run that session first — exactly the class Amendment 2's
/// state machine gives a short gap, "an operational refusal, **never a
/// terminal run-level verdict**". A predecessor whose journal is damaged or
/// does not bind to this run is a §C13.7 integrity fault instead.
///
/// Neither is ever `HOST-NOT-QUALIFIED`: exit 1 is a §7.2 **result**, derived
/// only by `--finalize` from the ledger and only alongside a closure record
/// (§C14.1). A `--session` invocation that measured nothing cannot publish one.
enum PredecessorFault {
    NotCompleted(String),
    JournalInvalid(String),
}

impl PredecessorFault {
    fn run_status(&self) -> controls::RunStatus {
        match self {
            PredecessorFault::NotCompleted(_) => controls::RunStatus::RefusedBeforeMeasurement,
            PredecessorFault::JournalInvalid(_) => controls::RunStatus::JournalInvalid,
        }
    }
    fn detail(&self) -> &str {
        match self {
            PredecessorFault::NotCompleted(why) | PredecessorFault::JournalInvalid(why) => why,
        }
    }
}

fn completed_session_close(
    dir: &std::path::Path,
    manifest: &manifest::RunManifest,
    session_id: u8,
) -> Result<u64, PredecessorFault> {
    let outcome = journal::read_journal(&dir.join(decision::session_journal_name(session_id)))
        .map_err(|e| PredecessorFault::JournalInvalid(format!("session {session_id}: {e}")))?;
    let journal::ReadOutcome::Present(read) = &outcome else {
        // §C13.7: an absent journal is NOT STARTED, not a fault.
        return Err(PredecessorFault::NotCompleted(format!(
            "session {session_id} is NOT STARTED"
        )));
    };
    let Some(meta) = read.typed_metadata.as_ref() else {
        return Err(PredecessorFault::JournalInvalid(format!(
            "session {session_id} metadata is absent"
        )));
    };
    if !metadata_binds(
        meta,
        manifest,
        journal::MetaSession::Qualification(session_id),
    ) {
        return Err(PredecessorFault::JournalInvalid(format!(
            "session {session_id} metadata does not bind to the manifest"
        )));
    }
    let classified = decision::classify_session(session_id, &outcome);
    if classified.journal_invalid {
        return Err(PredecessorFault::JournalInvalid(format!(
            "session {session_id} journal is invalid"
        )));
    }
    if classified.state != Some(decision::SessionState::Completed) {
        // ABORTED or NOT STARTED. The run may well be unqualifiable, but only
        // `--finalize` is entitled to say so, and only with a closure.
        return Err(PredecessorFault::NotCompleted(format!(
            "session {session_id} is not COMPLETED"
        )));
    }
    read.rows
        .iter()
        .find(|r| r.status == journal::Status::SessionCloseCompleted)
        .map(|r| r.monotonic_offset_ms)
        .ok_or_else(|| {
            // COMPLETED is defined by that row, so its absence is an internal
            // contradiction in the bytes, not a missing step.
            PredecessorFault::JournalInvalid(format!(
                "session {session_id} is COMPLETED with no close row"
            ))
        })
}

/// The host as it is **now** — `boot_id`, fingerprint and uptime, with no
/// provenance check and no `git`.
///
/// §C12.3's record documents a mismatch, so it must describe the host at the
/// moment the mismatch was seen. A snapshot taken by the preflight cannot: by
/// construction it already *passed* that preflight, so reusing it writes
/// `observed == expected` — "nothing changed" — beside a reason that says the
/// host changed, and recomputes `monotonic_offset_ms` from a stale uptime,
/// producing a number where the contract requires `null`.
fn reobserve_host() -> Option<(String, host::HostFields, u64)> {
    let sources = host::HostSources {
        osrelease: std::path::Path::new("/proc/sys/kernel/osrelease"),
        kernel_version: std::path::Path::new("/proc/version"),
        sys_cpu_dir: std::path::Path::new("/sys/devices/system/cpu"),
        meminfo: std::path::Path::new("/proc/meminfo"),
        self_status: std::path::Path::new("/proc/self/status"),
        cpuinfo: std::path::Path::new("/proc/cpuinfo"),
    };
    let host_fields = sources.read().ok()?;
    let boot_id =
        host::parse_boot_id(&std::fs::read_to_string("/proc/sys/kernel/random/boot_id").ok()?)
            .ok()?;
    let uptime_ms = host::read_uptime_ms(std::path::Path::new("/proc/uptime")).ok()?;
    Some((boot_id, host_fields, uptime_ms))
}

fn write_run_invalid(
    dir: &std::path::Path,
    manifest: &manifest::RunManifest,
    facts: &LiveFacts,
    args: Vec<String>,
    reason: String,
) -> i32 {
    // Re-observe. Only if the host cannot be read at all does the preflight
    // snapshot stand in, and then the record says so rather than passing stale
    // values off as current readings.
    let (boot_id, host_fields, uptime_ms, reason) = match reobserve_host() {
        Some((b, h, u)) => (b, h, u, reason),
        None => (
            facts.boot_id.clone(),
            facts.host_fields.clone(),
            facts.uptime_ms,
            format!("{reason}; host could not be re-observed, values are the preflight snapshot"),
        ),
    };
    let offset = host::monotonic_offset_ms(
        &manifest.boot_id,
        &boot_id,
        manifest.run_start_uptime_ms,
        uptime_ms,
    )
    .ok();
    let record = session::RunInvalid {
        schema_version: journal::SCHEMA_VERSION.to_string(),
        run_uuid: manifest.run_uuid.clone(),
        boot_id_observed: boot_id,
        boot_id_expected: manifest.boot_id.clone(),
        host_fingerprint_observed: host_fields.fingerprint(),
        host_fingerprint_expected: manifest.host_fingerprint.clone(),
        reason,
        command_line: args,
        monotonic_offset_ms: offset,
        utc: invocation_utc(),
    };
    match record.write(dir) {
        Ok(_) => controls::RunStatus::InstrumentInvalid.exit_code(),
        Err(e) => report_status(&e.to_string(), e.run_status()),
    }
}

/// The live world `--session` reads before it measures.
///
/// **Why this exists.** `live_facts` runs `provenance::check`, which refuses any
/// uncommitted tree. Every branch behind it — the §C12.3 clock fact, §C11.45's
/// short gap, an unreadable sentinel file — therefore returns the provenance
/// gate's own code during development and cannot be reached by a test. Three
/// tests written against those branches passed for the wrong reason and were
/// removed; a fourth mutation survived a test that looked like it covered it.
/// The rest of this instrument is built on injection (`FinalizeIo`,
/// `SessionEnvironment`, `ControlRunner`, `Clock`), and the absence of a seam
/// exactly here was the cause — not the branches.
trait SessionHost {
    fn repo_root(&mut self) -> Result<std::path::PathBuf, String>;
    fn facts(&mut self, repo: &std::path::Path) -> Result<LiveFacts, String>;
    fn observations(&mut self, cpu_set: &str) -> Result<(f64, host::DiagProbe), String>;
    fn sentinel(
        &mut self,
        repo: &std::path::Path,
    ) -> Result<protocol::VerifiedSentinelInstance, String>;
    fn exe(&mut self) -> Result<std::path::PathBuf, String>;
}

/// Production: the real repository, the real `/proc`, the real frozen G11.
struct RealSessionHost;

impl SessionHost for RealSessionHost {
    fn repo_root(&mut self) -> Result<std::path::PathBuf, String> {
        repository_root()
    }
    fn facts(&mut self, repo: &std::path::Path) -> Result<LiveFacts, String> {
        live_facts(repo)
    }
    fn observations(&mut self, cpu_set: &str) -> Result<(f64, host::DiagProbe), String> {
        invocation_observations(cpu_set)
    }
    fn sentinel(
        &mut self,
        repo: &std::path::Path,
    ) -> Result<protocol::VerifiedSentinelInstance, String> {
        load_g11(repo)
    }
    fn exe(&mut self) -> Result<std::path::PathBuf, String> {
        std::env::current_exe().map_err(|e| e.to_string())
    }
}

fn session_mode(dir: &std::path::Path, session_id: u8, run_id: &str, args: Vec<String>) -> i32 {
    session_mode_with(dir, session_id, run_id, args, &mut RealSessionHost)
}

fn session_mode_with(
    dir: &std::path::Path,
    session_id: u8,
    run_id: &str,
    args: Vec<String>,
    host_env: &mut dyn SessionHost,
) -> i32 {
    if let Err(e) = require_release_build() {
        return report_status(&e, controls::RunStatus::RefusedBeforeMeasurement);
    }
    if let Some(code) = closing_gate(dir) {
        return code;
    }
    let run_invalid = session::classify_run_invalid(dir);
    if matches!(&run_invalid, session::RunInvalidState::Damaged(_)) {
        return report_status(
            "run_invalid is damaged",
            controls::RunStatus::JournalInvalid,
        );
    }
    let started = match path_present(&dir.join(controls::CONTROLS_STARTED)) {
        Ok(v) => v,
        Err(e) => return report_status(&e, controls::RunStatus::JournalInvalid),
    };
    if !started && matches!(&run_invalid, session::RunInvalidState::Absent) {
        return report_status(
            "controls have not started",
            controls::RunStatus::RefusedBeforeMeasurement,
        );
    }
    let manifest = match read_bound_manifest(dir, run_id, controls::RunStatus::JournalInvalid) {
        Ok(m) => m,
        Err(code) => return code,
    };
    if let session::RunInvalidState::Valid(record) = &run_invalid {
        return match session::bind_run_invalid(record, &manifest) {
            Ok(()) => report_status(
                "run_invalid already exists",
                controls::RunStatus::InstrumentInvalid,
            ),
            Err(why) => report_status(
                &format!("run_invalid does not bind to this run: {why}"),
                controls::RunStatus::JournalInvalid,
            ),
        };
    }
    let current_path = dir.join(decision::session_journal_name(session_id));
    match path_present(&current_path) {
        Ok(true) => {
            return report_status(
                "this session journal already exists",
                controls::RunStatus::RefusedBeforeMeasurement,
            )
        }
        Ok(false) => {}
        Err(e) => return report_status(&e, controls::RunStatus::JournalInvalid),
    }
    let controls_complete_offset_ms = match controls_anchor(dir, &manifest) {
        Ok(v) => v,
        Err(e) => return report_status(&e, controls::RunStatus::InstrumentInvalid),
    };
    let mut previous_close_offset_ms = None;
    for preceding in 1..session_id {
        match completed_session_close(dir, &manifest, preceding) {
            Ok(offset) => previous_close_offset_ms = Some(offset),
            Err(fault) => return report_status(fault.detail(), fault.run_status()),
        }
    }
    // A missing `git`, an unresolvable repository root or a failed provenance
    // check says nothing about the journals and nothing about the host. It is
    // correctable and writes nothing, exactly as `controls_mode` and
    // `init_run_mode` classify the identical failures. Reporting exit 4 here
    // would declare a healthy run's evidence broken because a shell had no
    // `git` on its PATH.
    let repo = match host_env.repo_root() {
        Ok(r) => r,
        Err(e) => return report_status(&e, controls::RunStatus::RefusedBeforeMeasurement),
    };
    let facts = match host_env.facts(&repo) {
        Ok(f) => f,
        Err(e) => return report_status(&e, controls::RunStatus::RefusedBeforeMeasurement),
    };
    // §C12.3: a changed `boot_id` or an uptime below the run's start is a
    // terminal host mismatch that must be recorded, not folded into a zero
    // offset. Substituting 0 here would feed `session_preflight` a fabricated
    // "now" and risk reporting a correctable short gap for a host that had
    // already changed underneath the run — the exact confusion
    // `session_preflight`'s own contract warns against.
    let now_offset_ms = match host::monotonic_offset_ms(
        &manifest.boot_id,
        &facts.boot_id,
        manifest.run_start_uptime_ms,
        facts.uptime_ms,
    ) {
        Ok(v) => v,
        Err(e) => return write_run_invalid(dir, &manifest, &facts, args, e.to_string()),
    };
    let preflight = session::SessionPreflight {
        session: session_id,
        manifest: &manifest,
        observed: observed_config(&facts),
        observed_boot_id: facts.boot_id.clone(),
        now_offset_ms,
        controls_complete_offset_ms,
        previous_close_offset_ms,
        run_invalid,
    };
    match session::session_preflight(&preflight) {
        session::SessionOutcome::Proceed { .. } => {}
        session::SessionOutcome::Terminal { reason } => {
            return write_run_invalid(dir, &manifest, &facts, args, reason)
        }
        other => {
            eprintln!("rc021 session {session_id}: {}", other.describe());
            return other.terminal_exit_code().unwrap_or_else(|| {
                report_status(
                    "unclassified session preflight",
                    controls::RunStatus::JournalInvalid,
                )
            });
        }
    }
    // None of the three below is a host or provenance mismatch, so none may
    // write `run_invalid.json` — `SessionOutcome::writes_run_invalid` names the
    // only two outcomes that do, and its contract warns that "a caller that
    // decides this for itself would drift from the module". A one-off
    // unreadable `/sys/devices/system/cpu` or an EIO on the frozen G11 file
    // would otherwise poison the run permanently: once the record exists every
    // later `--controls`/`--session` is INSTRUMENT-INVALID and `--finalize`
    // derives Class I. `controls_mode` already treats the identical failures as
    // correctable refusals; `--session` now agrees.
    let (timer_resolution_ms, diag_availability) = match host_env.observations(&manifest.cpu_set) {
        Ok(observations) => observations,
        Err(e) => return report_status(&e, controls::RunStatus::RefusedBeforeMeasurement),
    };
    let instance = match host_env.sentinel(&repo) {
        Ok(i) => i,
        Err(e) => return report_status(&e, controls::RunStatus::RefusedBeforeMeasurement),
    };
    let exe = match host_env.exe() {
        Ok(p) => p,
        Err(e) => {
            return report_status(
                &e.to_string(),
                controls::RunStatus::RefusedBeforeMeasurement,
            )
        }
    };
    // One reading for every header this invocation writes.
    let utc = invocation_utc();
    let meta = journal_metadata_observed(
        &manifest,
        args.clone(),
        journal::MetaSession::Qualification(session_id),
        timer_resolution_ms,
        diag_availability,
        utc.clone(),
    );
    let ctx = row_context_observed(&manifest, timer_resolution_ms);
    // `args` is moved into the environment below; §C12.3's record needs it too.
    let args_for_record = args.clone();
    let mut live = match controls::LiveEnvironment::new(
        dir,
        &repo,
        &exe,
        &instance,
        &manifest,
        timer_resolution_ms,
        diag_availability,
        args,
        utc,
    ) {
        Ok(v) => v,
        // `LiveEnvironment::new` fails on exactly one thing — an unparsable
        // `cpu_set` — and nothing durable exists yet, so this is the same
        // correctable refusal `controls_mode` gives for the same cause.
        Err(e) => {
            return report_status(
                &e.to_string(),
                controls::RunStatus::RefusedBeforeMeasurement,
            )
        }
    };
    match session::execute_session(dir, session_id, &meta, &ctx, &manifest, &mut live) {
        Ok(()) => controls::MODE_SUCCESS_EXIT_CODE,
        // §C12.3: the host moved underneath a running session. The rows already
        // written stand as evidence, and the record of *why* the run stopped is
        // written here — the one thing the old `JournalWriteFailed` mapping
        // could never do.
        Err(session::SessionOutcome::HostChangedMidSession { why }) => {
            write_run_invalid(dir, &manifest, &facts, args_for_record, why)
        }
        Err(outcome) => {
            // Every one of these carries a `why`/`reason` the operator needs.
            // Dropping it leaves a bare exit code and no way to tell a refused
            // sentinel from a failed fsync.
            eprintln!("rc021 session {session_id}: {}", outcome.describe());
            outcome.terminal_exit_code().unwrap_or_else(|| {
                report_status(
                    "unclassified session execution",
                    controls::RunStatus::JournalInvalid,
                )
            })
        }
    }
}

/// §C13.6's whole precondition set — the terminal lock, the §C14.1 closure
/// obligation, identity reconstruction and the `--run-id` binding — is ordered
/// inside `decision::finalize` and must not be re-implemented here. A wrapper
/// that tested identity first put the run-id check ahead of the lock and
/// reported `JOURNAL-INVALID` where §C13.6 requires
/// `REFUSED-BEFORE-MEASUREMENT`.
fn finalize_mode(dir: &std::path::Path, run_id: &str) -> i32 {
    let outcome = decision::finalize(dir, run_id);
    report_finalize(&outcome);
    outcome.terminal_exit_code().unwrap_or_else(|| {
        report_status(
            "unclassified finalize outcome",
            controls::RunStatus::JournalInvalid,
        )
    })
}

fn report_finalize(outcome: &decision::FinalizeOutcome) {
    // `variant_name()` alone prints "Wrote" for every terminal status and
    // "DurabilityFailed" for every durability point — discarding the run's
    // verdict and which step failed. That is the exact loss `SessionOutcome::
    // describe` exists to prevent three functions above.
    eprintln!("rc021 finalize: {}", outcome.describe());
}

fn verify_mode(dir: &std::path::Path) -> i32 {
    let outcome = decision::verify(dir);
    if let Some(code) = outcome.mode_exit_code() {
        return code;
    }
    // The fourth mode. The other three were hardened to print their reason in
    // this same change; `--verify`'s `Mismatch { what }` names which of the
    // fifteen inventory entries failed, and without it exit 4 arrives with
    // empty stderr.
    if matches!(outcome, decision::VerifyOutcome::NoClosurePath) {
        // One line, not two: `report_status` already names the status, and the
        // variant adds nothing an operator can use here.
        return report_status(
            "closure path does not exist",
            controls::RunStatus::RefusedBeforeMeasurement,
        );
    }
    eprintln!("rc021 verify: {}", outcome.describe());
    outcome.terminal_exit_code().unwrap_or_else(|| {
        report_status(
            "unclassified verify outcome",
            controls::RunStatus::JournalInvalid,
        )
    })
}

/// Build the child's journal metadata from the run directory's manifest, then
/// hand over to the control. Never returns: the control ends in `abort()`.
fn p2_child(dir: &std::path::Path, args: Vec<String>) -> ! {
    // The child's own start instant, in RFC 3339.
    let utc_now = invocation_utc();
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
    let (timer_resolution_ms, diag_availability) = match invocation_observations(&m.cpu_set) {
        Ok(observations) => observations,
        Err(e) => {
            eprintln!("rc021 p2 child: invocation observations: {e}");
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
        timer_resolution_ms,
        cpu_time_unit: m.cpu_time_unit.clone(),
        // §C11.4: this invocation's own provenance, not the manifest's.
        command_line: args,
        utc_start: utc_now,
        session: journal::MetaSession::P2,
        diag_availability: journal::DiagAvailability {
            cpu_time: diag_availability.cpu_time,
            ctx_switches: diag_availability.ctx_switches,
            freq: diag_availability.freq,
        },
    };
    let ctx = journal::RowContext {
        run_uuid: m.run_uuid.clone(),
        repo_commit: m.repo_commit.clone(),
        prereg_commit: m.prereg_commit.clone(),
        host_fingerprint: m.host_fingerprint.clone(),
        timer_resolution_ms,
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

#[cfg(test)]
mod cli_tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(name: &str) -> TempDir {
            let path = std::env::temp_dir().join(format!(
                "rc021_cli_{}_{}_{}",
                std::process::id(),
                SEQ.fetch_add(1, Ordering::SeqCst),
                name
            ));
            std::fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn argv(words: &[&str]) -> Vec<String> {
        std::iter::once("exp_rc021_host_qualify".to_string())
            .chain(words.iter().map(|s| (*s).to_string()))
            .collect()
    }

    #[test]
    fn the_five_public_modes_have_one_exact_argument_grammar() {
        let id = "0".repeat(32);
        assert!(matches!(
            parse_mode(&argv(&["--init-run", "--dir", "/tmp/run"])),
            Ok(Mode::InitRun { .. })
        ));
        assert!(matches!(
            parse_mode(&argv(&["--controls", "--dir", "/tmp/run", "--run-id", &id])),
            Ok(Mode::Controls { .. })
        ));
        assert!(matches!(
            parse_mode(&argv(&[
                "--session",
                "6",
                "--dir",
                "/tmp/run",
                "--run-id",
                &id
            ])),
            Ok(Mode::Session { session: 6, .. })
        ));
        assert!(matches!(
            parse_mode(&argv(&["--finalize", "--dir", "/tmp/run", "--run-id", &id])),
            Ok(Mode::Finalize { .. })
        ));
        assert!(matches!(
            parse_mode(&argv(&["--verify", "--dir", "/tmp/run"])),
            Ok(Mode::Verify { .. })
        ));
        for bad in [
            argv(&[]),
            argv(&["--verify", "/tmp/run"]),
            argv(&["--session", "0", "--dir", "/tmp/run", "--run-id", &id]),
            argv(&["--session", "7", "--dir", "/tmp/run", "--run-id", &id]),
            argv(&["--controls", "--run-id", &id, "--dir", "/tmp/run"]),
            argv(&["--finalize", "--dir", "/tmp/run", "--run-id", "ABC"]),
        ] {
            assert!(parse_mode(&bad).is_err(), "accepted {bad:?}");
        }
    }

    fn fixture_manifest(dir: &Path) -> manifest::RunManifest {
        let host_fields = host::HostFields {
            kernel_release: "6.6.0".to_string(),
            kernel_version: "#1 SMP".to_string(),
            available_processors: "4".to_string(),
            mem_total_kb: "10185860".to_string(),
            cpus_allowed_list: "0-3".to_string(),
            cpu_model: "Test CPU".to_string(),
        };
        manifest::RunManifest {
            run_uuid: "0".repeat(32),
            boot_id: host::parse_boot_id(
                &std::fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap(),
            )
            .unwrap(),
            run_start_uptime_ms: 0,
            run_dir: std::fs::canonicalize(dir)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            repo_commit: "a".repeat(40),
            prereg_commit: "b".repeat(40),
            amendment_commits: vec!["c".repeat(40), "d".repeat(40)],
            instrument_birth_commit: "e".repeat(40),
            host_fingerprint: host_fields.fingerprint(),
            cpu_set: host_fields.cpus_allowed_list.clone(),
            host_fields,
            thread_count: 1,
            timer_resolution_ms: 0.00002,
            cpu_time_unit: journal::CPU_TIME_UNIT.to_string(),
            command_line: argv(&["--init-run", "--dir", "fixture"]),
            utc_start: "2026-08-27T00:00:00Z".to_string(),
            diag_availability: host::DiagProbe {
                cpu_time: false,
                ctx_switches: false,
                freq: false,
            },
        }
    }

    fn write_synthetic_complete_run(dir: &Path) -> manifest::RunManifest {
        write_synthetic_complete_run_with(dir, |_| {})
    }

    /// The same fixture, with `mutate` applied to the manifest **before**
    /// anything is derived from it, so the markers, the control journal and
    /// every header stay bound to each other. Only the *live* observation can
    /// then disagree — which is the only way to reach the terminal host and
    /// clock branches without touching a real host.
    fn write_synthetic_complete_run_with(
        dir: &Path,
        mutate: impl FnOnce(&mut manifest::RunManifest),
    ) -> manifest::RunManifest {
        let mut manifest = fixture_manifest(dir);
        mutate(&mut manifest);
        manifest::init_run(dir, &manifest).unwrap();
        controls::ControlsStarted {
            schema_version: journal::SCHEMA_VERSION.to_string(),
            run_uuid: manifest.run_uuid.clone(),
            boot_id: manifest.boot_id.clone(),
            monotonic_offset_ms: 1,
            utc: "2026-08-27T00:00:01Z".to_string(),
            command_line: argv(&[
                "--controls",
                "--dir",
                "fixture",
                "--run-id",
                &manifest.run_uuid,
            ]),
        }
        .write(dir)
        .unwrap();
        let meta = journal_metadata(
            &manifest,
            argv(&[
                "--controls",
                "--dir",
                "fixture",
                "--run-id",
                &manifest.run_uuid,
            ]),
            journal::MetaSession::Control,
        );
        let mut control =
            controls::ControlJournal::create(&dir.join(controls::CONTROL_JOURNAL), &meta).unwrap();
        for id in controls::CONTROL_ORDER {
            control
                .append(&controls::ControlRow {
                    control_id: id,
                    ordinal: id.ordinal(),
                    status: controls::ControlStatus::Pass,
                    monotonic_offset_ms: u64::from(id.ordinal()) + 1,
                    detail: journal::json_string_encode("synthetic PASS"),
                })
                .unwrap();
        }
        drop(control);
        let control_bytes = std::fs::read(dir.join(controls::CONTROL_JOURNAL)).unwrap();
        controls::ControlsComplete {
            schema_version: journal::SCHEMA_VERSION.to_string(),
            run_uuid: manifest.run_uuid.clone(),
            boot_id: manifest.boot_id.clone(),
            monotonic_offset_ms: 20,
            utc: "2026-08-27T00:00:02Z".to_string(),
            control_journal_sha256: host::sha256_hex(&control_bytes),
            control_count: controls::CONTROL_COUNT,
            all_pass: true,
        }
        .write(dir)
        .unwrap();

        let ctx = row_context(&manifest);
        let p2_meta = journal_metadata(
            &manifest,
            argv(&[controls::P2_CHILD_ARG, "fixture"]),
            journal::MetaSession::P2,
        );
        let mut p2 = journal::Journal::create(&dir.join(controls::P2_JOURNAL), &p2_meta).unwrap();
        p2.append(&ctx.session_open(0, 30, 0.1)).unwrap();
        p2.append(&ctx.lost(0, journal::Phase::A, 1, None, 31, "synthetic abort"))
            .unwrap();
        drop(p2);

        let n3_meta =
            journal_metadata(&manifest, argv(&["synthetic-n3"]), journal::MetaSession::N3);
        let mut n3 = journal::Journal::create(&dir.join(controls::N3_JOURNAL), &n3_meta).unwrap();
        n3.append(&ctx.session_open(0, 40, 0.1)).unwrap();
        for coordinate in session::plan_session(1).unwrap() {
            let mut row = ctx.lost(
                0,
                coordinate.phase,
                coordinate.block,
                None,
                40 + u64::from(coordinate.measurement_index),
                "",
            );
            row.status = journal::Status::Ok;
            row.sentinel_first_ms = Some(1.0);
            row.sentinel_last_ms = Some(1.01);
            row.paired_spread = protocol::paired_spread(1.0, 1.01);
            row.load_avg_start = Some(0.1);
            row.load_avg_end = Some(0.1);
            n3.append(&row).unwrap();
        }
        n3.append(&ctx.session_close(0, 99, true, 0.1)).unwrap();

        for session_id in 1..=session::SESSIONS {
            let meta = journal_metadata(
                &manifest,
                argv(&[
                    "--session",
                    &session_id.to_string(),
                    "--dir",
                    "fixture",
                    "--run-id",
                    &manifest.run_uuid,
                ]),
                journal::MetaSession::Qualification(session_id),
            );
            let path = dir.join(decision::session_journal_name(session_id));
            let mut out = journal::Journal::create(&path, &meta).unwrap();
            out.append(&ctx.session_open(session_id, 100 * u64::from(session_id), 0.1))
                .unwrap();
            for coordinate in session::plan_session(session_id).unwrap() {
                let first = 1.0;
                let last = 1.01;
                let mut row = ctx.lost(
                    session_id,
                    coordinate.phase,
                    coordinate.block,
                    Some(coordinate.measurement_index),
                    100 * u64::from(session_id) + u64::from(coordinate.measurement_index),
                    "",
                );
                row.status = journal::Status::Ok;
                row.sentinel_first_ms = Some(first);
                row.sentinel_last_ms = Some(last);
                row.paired_spread = protocol::paired_spread(first, last);
                row.load_avg_start = Some(0.1);
                row.load_avg_end = Some(0.1);
                out.append(&row).unwrap();
            }
            out.append(&ctx.session_close(session_id, 100 * u64::from(session_id) + 99, true, 0.1))
                .unwrap();
        }
        manifest
    }

    fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
        fn walk(root: &Path, here: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
            for entry in std::fs::read_dir(here).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    walk(root, &path, out);
                } else {
                    out.insert(
                        path.strip_prefix(root)
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                        std::fs::read(path).unwrap(),
                    );
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(dir, dir, &mut out);
        out
    }

    #[test]
    fn finalize_and_verify_complete_a_synthetic_run_end_to_end() {
        let dir = TempDir::new("end_to_end");
        let manifest = write_synthetic_complete_run(dir.path());
        assert_eq!(finalize_mode(dir.path(), &manifest.run_uuid), 0);
        assert!(dir.path().join(decision::CLOSURE_FILE).exists());
        assert!(dir.path().join(decision::OBSERVATIONS_FILE).exists());
        assert!(dir.path().join(decision::RESULTS_FILE).exists());
        let results = std::fs::read_to_string(dir.path().join(decision::RESULTS_FILE)).unwrap();
        for required in [
            "### Invocation headers",
            "### Frozen diagnostic seeds",
            "`RC021_SENTINEL_SEED`: `31001`",
            "### Six measured idle intervals",
            "| 6 |",
            "### Durable control provenance",
            "`C10`: synthetic PASS",
            "### Integrity inventory before the results file",
            "`exit_status`: `0`",
        ] {
            assert!(results.contains(required), "missing {required:?}");
        }
        let before = snapshot(dir.path());
        assert_eq!(verify_mode(dir.path()), controls::MODE_SUCCESS_EXIT_CODE);
        assert_eq!(
            snapshot(dir.path()),
            before,
            "--verify must be byte-read-only"
        );
        assert_eq!(finalize_mode(dir.path(), &manifest.run_uuid), 2);
        assert_eq!(
            snapshot(dir.path()),
            before,
            "terminal lock must write nothing"
        );
    }

    /// §C13.6 and §C14.1: a directory holding no run at all has not begun, so
    /// finalize refuses it as `REFUSED-BEFORE-MEASUREMENT`, exit 2 —
    /// correctable, and no closure path is claimed. Exit 4 here would assert
    /// the run is *broken* rather than *absent*, and would contradict
    /// `decision`'s own §C13.6 gate.
    #[test]
    fn unmet_public_preconditions_create_no_rc021_bytes() {
        let dir = TempDir::new("refusals");
        let before = snapshot(dir.path());
        assert_eq!(verify_mode(dir.path()), 2);
        assert_eq!(
            finalize_mode(dir.path(), &"0".repeat(32)),
            controls::RunStatus::RefusedBeforeMeasurement.exit_code()
        );
        assert_eq!(snapshot(dir.path()), before);
    }

    /// The precondition **order**, which is the defect this test exists for.
    /// A wrong `--run-id` must never turn a locked, broken run's exit 4 into a
    /// correctable exit 2, and identity must never be tested before the
    /// §C14.1 obligation.
    #[test]
    fn the_finalize_precondition_order_is_lock_then_obligation_then_run_id() {
        let dir = TempDir::new("finalize_order");
        let manifest = fixture_manifest(dir.path());
        manifest::init_run(dir.path(), &manifest).unwrap();
        let foreign = "9".repeat(32);

        // 1. No run began: the run-id is irrelevant, right or wrong.
        for id in [&manifest.run_uuid, &foreign] {
            assert_eq!(
                finalize_mode(dir.path(), id),
                controls::RunStatus::RefusedBeforeMeasurement.exit_code(),
                "§C13.6: an absent run is refused, not diagnosed"
            );
        }
        assert!(!dir.path().join(decision::CLOSURE_FILE).exists());

        // 2. The run began and the run-id is wrong: still exit 2, still no
        //    reservation — §C13.6's binding, nothing written.
        std::fs::write(dir.path().join(controls::CONTROLS_STARTED), b"{}\n").unwrap();
        let before = snapshot(dir.path());
        assert_eq!(
            finalize_mode(dir.path(), &foreign),
            controls::RunStatus::RefusedBeforeMeasurement.exit_code()
        );
        assert_eq!(
            snapshot(dir.path()),
            before,
            "a mistyped argument writes nothing"
        );

        // 3. The closure path is claimed but empty: the run is locked and
        //    broken. §C14.2 makes that exit 4 **whatever** the run-id says.
        std::fs::write(dir.path().join(decision::CLOSURE_FILE), b"").unwrap();
        let before = snapshot(dir.path());
        for id in [&manifest.run_uuid, &foreign] {
            assert_eq!(
                finalize_mode(dir.path(), id),
                controls::RunStatus::JournalInvalid.exit_code(),
                "§C14.2: the terminal lock outranks the run-id test"
            );
        }
        assert_eq!(snapshot(dir.path()), before, "a locked run writes nothing");
    }

    fn run_invalid_record(manifest: &manifest::RunManifest) -> session::RunInvalid {
        session::RunInvalid {
            schema_version: journal::SCHEMA_VERSION.to_string(),
            run_uuid: manifest.run_uuid.clone(),
            boot_id_observed: "different-boot".to_string(),
            boot_id_expected: manifest.boot_id.clone(),
            host_fingerprint_observed: "f".repeat(64),
            host_fingerprint_expected: manifest.host_fingerprint.clone(),
            reason: "synthetic host mismatch".to_string(),
            command_line: argv(&[
                "--session",
                "1",
                "--dir",
                "fixture",
                "--run-id",
                &manifest.run_uuid,
            ]),
            monotonic_offset_ms: None,
            utc: "2026-08-27T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn controls_binds_an_existing_run_invalid_before_using_it_as_terminal_evidence() {
        let bound = TempDir::new("controls_bound_run_invalid");
        let bound_manifest = fixture_manifest(bound.path());
        manifest::init_run(bound.path(), &bound_manifest).unwrap();
        run_invalid_record(&bound_manifest)
            .write(bound.path())
            .unwrap();
        let before = snapshot(bound.path());
        assert_eq!(
            controls_mode(
                bound.path(),
                &bound_manifest.run_uuid,
                argv(&[
                    "--controls",
                    "--dir",
                    "fixture",
                    "--run-id",
                    &bound_manifest.run_uuid,
                ]),
            ),
            controls::RunStatus::InstrumentInvalid.exit_code()
        );
        assert_eq!(snapshot(bound.path()), before);

        let foreign = TempDir::new("controls_foreign_run_invalid");
        let foreign_manifest = fixture_manifest(foreign.path());
        manifest::init_run(foreign.path(), &foreign_manifest).unwrap();
        let mut record = run_invalid_record(&foreign_manifest);
        record.run_uuid = "1".repeat(32);
        record.write(foreign.path()).unwrap();
        let before = snapshot(foreign.path());
        assert_eq!(
            controls_mode(
                foreign.path(),
                &foreign_manifest.run_uuid,
                argv(&[
                    "--controls",
                    "--dir",
                    "fixture",
                    "--run-id",
                    &foreign_manifest.run_uuid,
                ]),
            ),
            controls::RunStatus::JournalInvalid.exit_code()
        );
        assert_eq!(snapshot(foreign.path()), before);
    }

    #[test]
    fn session_checks_run_invalid_before_other_preconditions_or_live_work() {
        let bound = TempDir::new("session_bound_run_invalid");
        let bound_manifest = fixture_manifest(bound.path());
        manifest::init_run(bound.path(), &bound_manifest).unwrap();
        run_invalid_record(&bound_manifest)
            .write(bound.path())
            .unwrap();
        let before = snapshot(bound.path());
        assert_eq!(
            session_mode(
                bound.path(),
                1,
                &bound_manifest.run_uuid,
                argv(&[
                    "--session",
                    "1",
                    "--dir",
                    "fixture",
                    "--run-id",
                    &bound_manifest.run_uuid,
                ]),
            ),
            controls::RunStatus::InstrumentInvalid.exit_code()
        );
        assert_eq!(snapshot(bound.path()), before);

        let foreign = TempDir::new("session_foreign_run_invalid");
        let foreign_manifest = fixture_manifest(foreign.path());
        manifest::init_run(foreign.path(), &foreign_manifest).unwrap();
        let mut record = run_invalid_record(&foreign_manifest);
        record.run_uuid = "1".repeat(32);
        record.write(foreign.path()).unwrap();
        let before = snapshot(foreign.path());
        assert_eq!(
            session_mode(
                foreign.path(),
                1,
                &foreign_manifest.run_uuid,
                argv(&[
                    "--session",
                    "1",
                    "--dir",
                    "fixture",
                    "--run-id",
                    &foreign_manifest.run_uuid,
                ]),
            ),
            controls::RunStatus::JournalInvalid.exit_code()
        );
        assert_eq!(snapshot(foreign.path()), before);
    }

    /// A run with genuinely complete controls and no session journals, so the
    /// predecessor precondition is the branch actually under test.
    fn run_ready_for_sessions(name: &str) -> (TempDir, manifest::RunManifest) {
        let d = TempDir::new(name);
        let m = write_synthetic_complete_run(d.path());
        for s in 1..=session::SESSIONS {
            let _ = std::fs::remove_file(d.path().join(decision::session_journal_name(s)));
        }
        (d, m)
    }

    fn session_argv(session: u8, run_id: &str) -> Vec<String> {
        argv(&[
            "--session",
            &session.to_string(),
            "--dir",
            "fixture",
            "--run-id",
            run_id,
        ])
    }

    /// A predecessor that has not run is an **operational** refusal, exit 2 —
    /// never `HOST-NOT-QUALIFIED`. Exit 1 is a §7.2 result that only
    /// `--finalize` may publish, and only with a closure record (§C14.1); a
    /// `--session` invocation that measured nothing has neither.
    #[test]
    fn a_predecessor_that_has_not_run_is_refused_and_never_a_verdict() {
        for session in [2u8, 3, 6] {
            let (d, m) = run_ready_for_sessions(&format!("pred_absent_{session}"));
            let before = snapshot(d.path());
            let code = session_mode(
                d.path(),
                session,
                &m.run_uuid,
                session_argv(session, &m.run_uuid),
            );
            assert_eq!(
                code,
                controls::RunStatus::RefusedBeforeMeasurement.exit_code(),
                "--session {session} with no predecessor must be an operational refusal"
            );
            assert_ne!(
                code,
                controls::RunStatus::HostNotQualified.exit_code(),
                "a mode that measured nothing must not publish a §7.2 verdict"
            );
            assert_eq!(snapshot(d.path()), before, "the refusal writes nothing");
            assert!(!d.path().join(decision::CLOSURE_FILE).exists());
        }
    }

    /// §C13.7: a damaged predecessor journal, or one whose metadata does not
    /// bind to this run, is an integrity fault — exit 4, not the exit 2 a
    /// missing step earns and not the exit 1 both used to collapse into.
    #[test]
    fn a_damaged_predecessor_journal_is_journal_invalid() {
        // A journal whose header is corrupt.
        let (d, m) = run_ready_for_sessions("pred_damaged");
        std::fs::write(
            d.path().join(decision::session_journal_name(1)),
            b"#rc021_meta not a header\n",
        )
        .unwrap();
        let before = snapshot(d.path());
        assert_eq!(
            session_mode(d.path(), 2, &m.run_uuid, session_argv(2, &m.run_uuid)),
            controls::RunStatus::JournalInvalid.exit_code()
        );
        assert_eq!(snapshot(d.path()), before);

        // A journal whose header and §C9 metadata are perfect and *do* bind,
        // but whose body is not the frozen plan. Only `classify_session`'s own
        // grammar can see this, so it is the case that separates "damaged" from
        // "not completed" — the earlier two are caught by the metadata checks.
        let (d, m) = run_ready_for_sessions("pred_bad_rows");
        let meta = journal_metadata(
            &m,
            argv(&["--session", "1"]),
            journal::MetaSession::Qualification(1),
        );
        let ctx = row_context(&m);
        let mut j =
            journal::Journal::create(&d.path().join(decision::session_journal_name(1)), &meta)
                .unwrap();
        j.append(&ctx.session_open(1, 10, 0.1)).unwrap();
        let first = session::plan_session(1).unwrap()[0];
        for n in 0..session::MEASUREMENTS_PER_SESSION {
            // Fifteen rows, all at the first coordinate: the right count and
            // the wrong plan.
            let mut row = ctx.lost(
                1,
                first.phase,
                first.block,
                Some(first.measurement_index),
                20 + u64::from(n),
                "",
            );
            row.status = journal::Status::Ok;
            row.sentinel_first_ms = Some(1.0);
            row.sentinel_last_ms = Some(1.01);
            row.paired_spread = protocol::paired_spread(1.0, 1.01);
            row.load_avg_start = Some(0.1);
            row.load_avg_end = Some(0.1);
            j.append(&row).unwrap();
        }
        j.append(&ctx.session_close(1, 99, true, 0.1)).unwrap();
        drop(j);
        let before = snapshot(d.path());
        assert_eq!(
            session_mode(d.path(), 2, &m.run_uuid, session_argv(2, &m.run_uuid)),
            controls::RunStatus::JournalInvalid.exit_code(),
            "a predecessor whose rows are not the frozen plan is an integrity fault"
        );
        assert_eq!(snapshot(d.path()), before);

        // A well-formed journal that belongs to a different run.
        let (d, m) = run_ready_for_sessions("pred_foreign");
        let mut foreign = fixture_manifest(d.path());
        foreign.run_uuid = "1".repeat(32);
        let meta = journal_metadata(
            &foreign,
            argv(&["--session", "1"]),
            journal::MetaSession::Qualification(1),
        );
        journal::Journal::create(&d.path().join(decision::session_journal_name(1)), &meta).unwrap();
        let before = snapshot(d.path());
        assert_eq!(
            session_mode(d.path(), 2, &m.run_uuid, session_argv(2, &m.run_uuid)),
            controls::RunStatus::JournalInvalid.exit_code(),
            "a predecessor journal from another run is an integrity fault"
        );
        assert_eq!(snapshot(d.path()), before);
    }

    /// The standing invariant behind the two tests above, checked across every
    /// `--session` refusal a fixture can reach: **no `--session` invocation may
    /// return a Class II code.**
    ///
    /// `0` and `1` are §7.2 results, derived only by `--finalize` from the
    /// ledger and only alongside a closure record (§C14.1). `--session` either
    /// measures — and then says nothing about the verdict — or refuses. This
    /// invariant is what the collapsed predecessor branch violated, and it
    /// holds whichever refusal happens to fire first on a given host.
    #[test]
    fn no_session_refusal_ever_returns_a_class_two_code() {
        let class_two = [
            controls::RunStatus::HostQualified.exit_code(),
            controls::RunStatus::HostNotQualified.exit_code(),
        ];
        /// A fixture builder: the scratch run, its manifest, and the session
        /// the invocation asks for.
        type Arrange = Box<dyn Fn() -> (TempDir, manifest::RunManifest, u8)>;
        let mut cases: Vec<(&str, Arrange)> = Vec::new();
        cases.push((
            "no predecessor",
            Box::new(|| {
                let (d, m) = run_ready_for_sessions("inv_no_pred");
                (d, m, 2)
            }),
        ));
        cases.push((
            "damaged predecessor",
            Box::new(|| {
                let (d, m) = run_ready_for_sessions("inv_damaged");
                std::fs::write(d.path().join(decision::session_journal_name(1)), b"junk\n")
                    .unwrap();
                (d, m, 2)
            }),
        ));
        cases.push((
            "aborted predecessor",
            Box::new(|| {
                let (d, m) = run_ready_for_sessions("inv_aborted");
                let meta = journal_metadata(
                    &m,
                    argv(&["--session", "1"]),
                    journal::MetaSession::Qualification(1),
                );
                let ctx = row_context(&m);
                let mut j = journal::Journal::create(
                    &d.path().join(decision::session_journal_name(1)),
                    &meta,
                )
                .unwrap();
                j.append(&ctx.session_open(1, 10, 0.1)).unwrap();
                j.append(&ctx.session_close(1, 20, false, 0.1)).unwrap();
                (d, m, 2)
            }),
        ));
        cases.push((
            "untrustworthy clock",
            Box::new(|| {
                // §C12.3's terminal clock fact. Whether this reaches the clock
                // branch depends on the provenance gate, which needs a
                // committed tree — but the invariant must hold either way.
                let d = TempDir::new("inv_clock");
                let m = write_synthetic_complete_run_with(d.path(), |m| {
                    m.run_start_uptime_ms = u64::MAX;
                });
                for s in 1..=session::SESSIONS {
                    let _ = std::fs::remove_file(d.path().join(decision::session_journal_name(s)));
                }
                (d, m, 1)
            }),
        ));
        cases.push((
            "this session already exists",
            Box::new(|| {
                let (d, m) = run_ready_for_sessions("inv_exists");
                let meta = journal_metadata(
                    &m,
                    argv(&["--session", "1"]),
                    journal::MetaSession::Qualification(1),
                );
                journal::Journal::create(&d.path().join(decision::session_journal_name(1)), &meta)
                    .unwrap();
                (d, m, 1)
            }),
        ));

        for (name, arrange) in cases {
            let (d, m, session) = arrange();
            let code = session_mode(
                d.path(),
                session,
                &m.run_uuid,
                session_argv(session, &m.run_uuid),
            );
            assert!(
                !class_two.contains(&code),
                "{name}: --session returned the Class II code {code}"
            );
            assert!(
                !d.path().join(decision::CLOSURE_FILE).exists(),
                "{name}: --session must never write a closure"
            );
        }
    }

    /// §C14.2 for the two measuring modes. The terminal lock is not only
    /// `--finalize`'s: a complete closure refuses `--controls` and `--session`
    /// at exit 2 ("finished"), an empty or partial path at exit 4 ("broken"),
    /// and neither may write a byte.
    #[test]
    fn the_terminal_lock_refuses_both_measuring_modes() {
        for (name, bytes, want) in [
            (
                "empty closure path",
                Vec::new(),
                controls::RunStatus::JournalInvalid.exit_code(),
            ),
            (
                "partial closure",
                b"{\"schema_version\":".to_vec(),
                controls::RunStatus::JournalInvalid.exit_code(),
            ),
        ] {
            let d = TempDir::new(&format!("lock_{name}"));
            let m = write_synthetic_complete_run(d.path());
            std::fs::write(d.path().join(decision::CLOSURE_FILE), &bytes).unwrap();
            let before = snapshot(d.path());
            assert_eq!(
                controls_mode(
                    d.path(),
                    &m.run_uuid,
                    argv(&["--controls", "--dir", "fixture", "--run-id", &m.run_uuid]),
                ),
                want,
                "{name}: --controls"
            );
            assert_eq!(
                session_mode(d.path(), 1, &m.run_uuid, session_argv(1, &m.run_uuid)),
                want,
                "{name}: --session"
            );
            assert_eq!(
                snapshot(d.path()),
                before,
                "{name}: a locked run writes nothing"
            );
        }

        // A *complete* closure is "finished", not "broken": exit 2.
        let d = TempDir::new("lock_complete");
        let m = write_synthetic_complete_run(d.path());
        assert_eq!(
            finalize_mode(d.path(), &m.run_uuid),
            controls::MODE_SUCCESS_EXIT_CODE,
            "the fixture finalizes cleanly first"
        );
        let before = snapshot(d.path());
        for (what, code) in [
            (
                "--controls",
                controls_mode(
                    d.path(),
                    &m.run_uuid,
                    argv(&["--controls", "--dir", "fixture", "--run-id", &m.run_uuid]),
                ),
            ),
            (
                "--session",
                session_mode(d.path(), 1, &m.run_uuid, session_argv(1, &m.run_uuid)),
            ),
        ] {
            assert_eq!(
                code,
                controls::RunStatus::RefusedBeforeMeasurement.exit_code(),
                "{what}: a finished run is refused, not diagnosed"
            );
        }
        assert_eq!(snapshot(d.path()), before);
    }

    // §C11.45's short-gap refusal has **no CLI-level test**, deliberately.
    // The branch sits past `live_facts`, whose provenance gate refuses on any
    // uncommitted tree, so in-process it returns INSTRUMENT-INVALID before the
    // gap is ever checked. The rule and its exit code are covered where they
    // live — `session::tests::the_gap_floor_is_inclusive_and_a_backwards_clock_
    // is_not_a_short_gap` and the frozen `SESSION_VARIANTS` classification
    // table. The missing wiring test is `OPEN_PROBLEMS.md` §0b item J, whose
    // remedy is a `LiveFacts` seam on `session_mode`.

    // `--init-run`'s reserved-path refusal likewise has **no CLI-level test**.
    // A test written for it passed for the wrong reason — `init_run_mode`
    // reaches `repository_root`/`live_facts` and the provenance gate refuses a
    // dirty tree at the same exit 2 — and a mutation that bypassed
    // `manifest::preflight_run_dir` entirely survived it. A test that cannot
    // distinguish the branch it names is worse than none, so it was removed.
    // The rule is covered where it lives: `manifest::tests::
    // any_journal_shaped_name_blocks_the_run`, `similar_but_unreserved_names_
    // do_not_block` and `reserved_path_errors_are_not_read_as_absence`.
    // Same seam, same remedy: `OPEN_PROBLEMS.md` §0b item J.

    /// §C12.3's record must describe the host **at the failure**, not the
    /// snapshot the preflight took minutes earlier.
    ///
    /// A stale snapshot has already *passed* that preflight by construction, so
    /// reusing it writes `observed == expected` — "nothing changed" — beside a
    /// reason saying the host changed, and recomputes `monotonic_offset_ms`
    /// from a stale uptime, producing a number where §C12.3 requires `null`.
    #[test]
    fn the_run_invalid_record_describes_the_host_at_the_failure() {
        let d = TempDir::new("record_freshness");
        let mut m = fixture_manifest(d.path());
        // A manifest whose boot id is not this host's: any re-observation must
        // therefore disagree with `expected`, and the clock must be untrusted.
        m.boot_id = "a-boot-this-host-never-had".to_string();
        manifest::init_run(d.path(), &m).unwrap();
        let stale = LiveFacts {
            host_fields: m.host_fields.clone(),
            // The lie a stale snapshot tells: "observed == expected".
            boot_id: m.boot_id.clone(),
            uptime_ms: m.run_start_uptime_ms + 1,
            provenance: provenance::ProvenanceSnapshot {
                repo_commit: m.repo_commit.clone(),
                prereg_commit: m.prereg_commit.clone(),
                amendment_commits: m.amendment_commits.clone(),
                instrument_birth_commit: m.instrument_birth_commit.clone(),
            },
            thread_count: m.thread_count,
        };

        let code = write_run_invalid(
            d.path(),
            &m,
            &stale,
            argv(&["--session", "1"]),
            "measurement clock: boot_id changed".to_string(),
        );
        assert_eq!(code, controls::RunStatus::InstrumentInvalid.exit_code());

        let raw = std::fs::read_to_string(d.path().join(session::RUN_INVALID_FILE)).unwrap();
        let rec = session::RunInvalid::parse(&raw).expect("the record must be canonical");
        assert_ne!(
            rec.boot_id_observed, rec.boot_id_expected,
            "a record of a mismatch that says nothing changed is worse than none"
        );
        assert!(
            rec.monotonic_offset_ms.is_none(),
            "§C12.3: null exactly when the clock is untrustworthy — not a number \
             recomputed from the stale snapshot"
        );
    }

    /// A failure that is neither a host nor a provenance mismatch must not
    /// write `run_invalid.json`. Once that file exists the run is unrecoverable
    /// — every later mode is INSTRUMENT-INVALID and `--finalize` derives Class
    /// I — so a one-off unreadable `/sys` or an EIO on the frozen G11 file would
    /// destroy a run a retry would have completed.
    #[test]
    fn only_host_facts_write_the_record_and_the_modes_agree_on_the_rest() {
        // `SessionOutcome` is the single owner of that rule; assert it names
        // exactly the two host outcomes.
        for outcome in crate::controls::sample_session_outcomes() {
            let writes = outcome.writes_run_invalid();
            let is_host_fact = matches!(
                outcome,
                session::SessionOutcome::Terminal { .. }
                    | session::SessionOutcome::HostChangedMidSession { .. }
            );
            assert_eq!(
                writes,
                is_host_fact,
                "{} must {} write the record",
                outcome.variant_name(),
                if is_host_fact { "" } else { "not" }
            );
        }

        // The CLI half of this deliberately does **not** call `session_mode`.
        //
        // `session_mode` uses `RealSessionHost`, so its behaviour depends on
        // whether the working tree is clean: while it is dirty
        // `provenance::check` refuses first and the test passes without
        // reaching any decision, and once the change is committed the same
        // call runs on to `check_configuration`, where a synthetic fixture
        // manifest legitimately mismatches the real host and writes
        // `run_invalid.json`. A test whose outcome flips on `git commit` is
        // testing the tree, not the code. The CLI behaviour is covered through
        // the seam instead, by
        // `an_unreadable_sentinel_refuses_without_poisoning_the_run`.
    }

    /// A `SessionHost` whose every reading is under the test's control, so the
    /// branches behind the provenance gate can finally be reached.
    struct FakeSessionHost {
        facts: LiveFacts,
        sentinel_fails: bool,
        repo_fails: bool,
    }

    impl FakeSessionHost {
        fn new(m: &manifest::RunManifest) -> Self {
            FakeSessionHost {
                facts: LiveFacts {
                    host_fields: m.host_fields.clone(),
                    boot_id: m.boot_id.clone(),
                    uptime_ms: m.run_start_uptime_ms + 10 * session::MIN_GAP_MS,
                    provenance: provenance::ProvenanceSnapshot {
                        repo_commit: m.repo_commit.clone(),
                        prereg_commit: m.prereg_commit.clone(),
                        amendment_commits: m.amendment_commits.clone(),
                        instrument_birth_commit: m.instrument_birth_commit.clone(),
                    },
                    thread_count: m.thread_count,
                },
                sentinel_fails: false,
                repo_fails: false,
            }
        }
    }

    impl SessionHost for FakeSessionHost {
        fn repo_root(&mut self) -> Result<PathBuf, String> {
            if self.repo_fails {
                return Err("git is not on PATH".to_string());
            }
            Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        }
        fn facts(&mut self, _repo: &Path) -> Result<LiveFacts, String> {
            Ok(self.facts.clone())
        }
        fn observations(&mut self, _cpu_set: &str) -> Result<(f64, host::DiagProbe), String> {
            Ok((
                0.00002,
                host::DiagProbe {
                    cpu_time: false,
                    ctx_switches: false,
                    freq: false,
                },
            ))
        }
        fn sentinel(&mut self, _repo: &Path) -> Result<protocol::VerifiedSentinelInstance, String> {
            if self.sentinel_fails {
                return Err("G11 is unreadable".to_string());
            }
            Err("the test never measures".to_string())
        }
        fn exe(&mut self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/nonexistent/exe"))
        }
    }

    /// §C11.45: a gap below 600 000 ms is an **operational** refusal — exit 2,
    /// before the journal is created, retryable after waiting. It is explicitly
    /// "never a terminal run-level verdict".
    ///
    /// Reachable only through the seam: in-process the provenance gate returns
    /// first, which is exactly why the branch had no test until now.
    #[test]
    fn a_short_gap_refuses_at_two_and_creates_no_journal() {
        let (d, m) = run_ready_for_sessions("short_gap");
        let mut env = FakeSessionHost::new(&m);
        // The controls-complete anchor is 2 ms; put "now" one millisecond short.
        env.facts.uptime_ms = m.run_start_uptime_ms + 2 + session::MIN_GAP_MS - 1;
        let before = snapshot(d.path());
        let code = session_mode_with(
            d.path(),
            1,
            &m.run_uuid,
            session_argv(1, &m.run_uuid),
            &mut env,
        );
        assert_eq!(
            code,
            controls::RunStatus::RefusedBeforeMeasurement.exit_code(),
            "a short gap is operational, never a verdict"
        );
        assert!(
            !d.path().join(decision::session_journal_name(1)).exists(),
            "the refusal precedes journal creation"
        );
        assert!(!d.path().join(session::RUN_INVALID_FILE).exists());
        assert_eq!(snapshot(d.path()), before);
    }

    /// §C12.3 through the CLI: an uptime below the run's own start is a terminal
    /// host fact — `run_invalid.json`, exit 3 — not a zero offset quietly handed
    /// to the gap check.
    #[test]
    fn an_untrustworthy_clock_writes_the_record_through_the_cli() {
        // The fixture's own start must be non-zero for "uptime went backwards"
        // to be expressible at all — with a start of 0 no reading is below it,
        // `monotonic_offset_ms` succeeds, and the branch under test is never
        // entered. A mutation proved exactly that.
        let d = TempDir::new("cli_clock");
        let m = write_synthetic_complete_run_with(d.path(), |m| {
            m.run_start_uptime_ms = 5 * session::MIN_GAP_MS;
        });
        for s in 1..=session::SESSIONS {
            let _ = std::fs::remove_file(d.path().join(decision::session_journal_name(s)));
        }
        // Put the ten-minute anchor at 0. Now the two behaviours separate:
        // reporting the clock fact writes the §C12.3 record and exits 3, while
        // folding the error into a zero "now" makes the gap 0 ms and reports a
        // **correctable short gap**, exit 2 — inviting the operator to wait and
        // retry a host that had already moved underneath the run. Without this,
        // both paths reach exit 3 by different routes and no test can tell them
        // apart; a mutation proved exactly that.
        let marker = d.path().join(controls::CONTROLS_COMPLETE);
        let mut complete =
            controls::ControlsComplete::parse(&std::fs::read_to_string(&marker).unwrap()).unwrap();
        complete.monotonic_offset_ms = 0;
        std::fs::write(&marker, complete.render()).unwrap();

        let mut env = FakeSessionHost::new(&m);
        env.facts.uptime_ms = m.run_start_uptime_ms - 1; // genuinely backwards
        let code = session_mode_with(
            d.path(),
            1,
            &m.run_uuid,
            session_argv(1, &m.run_uuid),
            &mut env,
        );
        assert_eq!(
            code,
            controls::RunStatus::InstrumentInvalid.exit_code(),
            "a host that already moved is terminal, not a short gap to wait out"
        );
        assert_ne!(
            code,
            controls::RunStatus::RefusedBeforeMeasurement.exit_code(),
            "exit 2 here would invite a retry that can never succeed"
        );
        let raw = std::fs::read_to_string(d.path().join(session::RUN_INVALID_FILE))
            .expect("§C12.3 requires the record");
        let rec = session::RunInvalid::parse(&raw).expect("canonical");
        assert_eq!(rec.run_uuid, m.run_uuid, "the record binds to this run");
        assert!(
            rec.reason.contains("uptime") || rec.reason.contains("clock"),
            "the reason must name what was seen: {}",
            rec.reason
        );
        // `monotonic_offset_ms` is deliberately **not** asserted null here. The
        // fake's uptime is a *preflight* reading, and `write_run_invalid`
        // re-observes rather than trusting it — so on a healthy host the
        // current clock is legitimately an integer. The null case is covered by
        // `the_run_invalid_record_describes_the_host_at_the_failure`, where
        // re-observation genuinely disagrees with the manifest.
        assert!(!d.path().join(decision::session_journal_name(1)).exists());
    }

    /// An unreadable sentinel file is a correctable refusal. It must **not**
    /// write `run_invalid.json`: once that exists every later mode is
    /// INSTRUMENT-INVALID and `--finalize` derives Class I, so a one-off EIO
    /// would destroy a run a retry would have completed.
    #[test]
    fn an_unreadable_sentinel_refuses_without_poisoning_the_run() {
        let (d, m) = run_ready_for_sessions("cli_sentinel");
        let mut env = FakeSessionHost::new(&m);
        env.sentinel_fails = true;
        let before = snapshot(d.path());
        let code = session_mode_with(
            d.path(),
            1,
            &m.run_uuid,
            session_argv(1, &m.run_uuid),
            &mut env,
        );
        assert_eq!(
            code,
            controls::RunStatus::RefusedBeforeMeasurement.exit_code(),
            "a missing file is not a host mismatch and not broken evidence"
        );
        assert!(
            !d.path().join(session::RUN_INVALID_FILE).exists(),
            "a non-host failure must never poison the run"
        );
        assert_eq!(snapshot(d.path()), before);
    }

    /// A missing `git`, an unresolvable repository root or a failed provenance
    /// check says nothing about the journals. `--controls` and `--init-run`
    /// class it a correctable refusal; `--session` used to answer exit 4 —
    /// "the evidence could not be written" — for a perfectly healthy run whose
    /// shell simply had no `git` on its PATH.
    #[test]
    fn a_missing_repository_is_a_refusal_not_broken_evidence() {
        let (d, m) = run_ready_for_sessions("no_repo");
        let mut env = FakeSessionHost::new(&m);
        env.repo_fails = true;
        let before = snapshot(d.path());
        let code = session_mode_with(
            d.path(),
            1,
            &m.run_uuid,
            session_argv(1, &m.run_uuid),
            &mut env,
        );
        assert_eq!(
            code,
            controls::RunStatus::RefusedBeforeMeasurement.exit_code(),
            "the same failure `controls_mode` and `init_run_mode` call correctable"
        );
        assert_ne!(code, controls::RunStatus::JournalInvalid.exit_code());
        assert!(!d.path().join(session::RUN_INVALID_FILE).exists());
        assert_eq!(snapshot(d.path()), before);
    }

    /// The optimisation guard, at the level it can actually be checked.
    ///
    /// Its *wiring* into the two measuring modes cannot be exercised here: the
    /// guard only fires on an unoptimised build, and every build of this
    /// workspace is optimised, so no test can make either mode refuse for that
    /// reason — and a test that merely observed exit 2 could not tell the guard
    /// apart from any other refusal on the same path. What is checkable, and
    /// what actually went wrong, is the predicate: the original guard was
    /// `cfg!(debug_assertions)`, which is **constant `false`** here, so it could
    /// never fire in either mode.
    #[test]
    fn the_optimisation_guard_is_decided_from_evidence() {
        assert!(controls::is_optimised_build());
        assert!(require_release_build().is_ok());
        // No assertion about `cfg!(debug_assertions)` itself: it is constant
        // `false` here, which is precisely why it could not serve as the guard.
        // The predicate's own behaviour is covered exhaustively in
        // `session::tests::the_optimisation_predicate_separates_the_cases_the_
        // old_proxy_could_not`.
    }

    /// `--finalize` must report the verdict it reached, not just the shape of
    /// the outcome. `variant_name()` is `"Wrote"` for every terminal status.
    #[test]
    fn finalize_reports_the_verdict_and_the_failing_durability_step() {
        assert_eq!(
            decision::FinalizeOutcome::Wrote {
                status: controls::RunStatus::HostQualified
            }
            .describe(),
            "Wrote: HOST-QUALIFIED"
        );
        assert_eq!(
            decision::FinalizeOutcome::Wrote {
                status: controls::RunStatus::JournalInvalid
            }
            .describe(),
            "Wrote: JOURNAL-INVALID"
        );
        assert_eq!(
            decision::FinalizeOutcome::DurabilityFailed { at: "sync_all" }.describe(),
            "DurabilityFailed at sync_all"
        );
        // The two must be distinguishable, which `variant_name` alone is not.
        assert_ne!(
            decision::FinalizeOutcome::Wrote {
                status: controls::RunStatus::HostQualified
            }
            .describe(),
            decision::FinalizeOutcome::Wrote {
                status: controls::RunStatus::HostNotQualified
            }
            .describe()
        );
    }

    /// One invocation, one `utc_start` across every header it writes.
    ///
    /// `--controls` read the wall clock twice: once inside the metadata builder
    /// for the control journal, once for the N3 and P2 headers. A run
    /// straddling a second boundary stamped them differently, and nothing in
    /// `metadata_binds` or `ControlsContext::bind` compares `utc_start`, so it
    /// failed silently.
    #[test]
    fn one_invocation_writes_one_utc_start() {
        let raw = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/bin/exp_rc021_host_qualify/main.rs"),
        )
        .unwrap();
        for (mode, start, end) in [
            ("controls", "fn controls_mode(", "fn metadata_binds("),
            ("session", "fn session_mode_with(", "fn finalize_mode("),
        ] {
            let body = &raw[raw.find(start).unwrap()..raw.find(end).unwrap()];
            assert_eq!(
                body.matches("invocation_utc()").count(),
                1,
                "{mode} must read the wall clock once"
            );
            assert!(
                !body.contains("chrono::Utc::now()"),
                "{mode} must not read the clock directly"
            );
        }
        // And the one helper is the only direct reader in the production half.
        // Scanning the whole file would count this test's own string literals.
        let production = &raw[..raw.find("mod cli_tests").unwrap()];
        assert_eq!(
            production.matches("chrono::Utc::now()").count(),
            1,
            "exactly one direct wall-clock read, inside `invocation_utc`"
        );
    }

    /// One cause, one classification. `LiveEnvironment::new` fails only on an
    /// unparsable `cpu_set`, and nothing durable exists at that point in either
    /// mode, so both must call it the same correctable refusal.
    #[test]
    fn the_two_measuring_modes_agree_on_a_bad_cpu_set() {
        let raw = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/bin/exp_rc021_host_qualify/main.rs"),
        )
        .unwrap();
        for (mode, start, end) in [
            ("controls", "fn controls_mode(", "fn metadata_binds("),
            ("session", "fn session_mode_with(", "fn finalize_mode("),
        ] {
            let body = &raw[raw.find(start).unwrap()..raw.find(end).unwrap()];
            let call = body.find("LiveEnvironment::new(").expect(mode);
            let arm = &body[call..];
            let arm = &arm[..arm.find("};").unwrap_or(arm.len())];
            assert!(
                arm.contains("RefusedBeforeMeasurement"),
                "{mode} must call a bad cpu_set correctable, not an invalid instrument"
            );
            assert!(!arm.contains("InstrumentInvalid"), "{mode}");
        }
    }
}
