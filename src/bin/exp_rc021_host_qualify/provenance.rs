//! RC-021 provenance — Amendment 2 §C15, base §6 P6.
//!
//! Chronology is established by **strict ancestry only**. Commit dates are not
//! evidence of order: rebase rewrites them, cherry-pick changes them, `--date`
//! sets them and clock skew corrupts them. Nothing here compares a timestamp.
//!
//! The decision is a pure function over gathered facts, so it is testable
//! without a repository; the git calls are a thin wrapper around it.

#![allow(dead_code)] // Consumers arrive in later commits of the §12 plan.

use std::path::Path;
use std::process::Command;

/// §C15: the pre-registration and its two amendments, in amendment order.
pub const BINDING_DOCS: [&str; 3] = [
    "research/PREREG_RC021_HOST_INSTRUMENT.md",
    "research/PREREG_RC021_AMENDMENT_1.md",
    "research/PREREG_RC021_AMENDMENT_2.md",
];

/// The file whose first-adding commit defines `instrument_birth_commit`.
pub const BIRTH_PATH: &str = "src/bin/exp_rc021_host_qualify/main.rs";

/// Every file the instrument is made of; all must be tracked and clean.
pub const INSTRUMENT_FILES: [&str; 10] = [
    "src/bin/exp_rc021_host_qualify/main.rs",
    "src/bin/exp_rc021_host_qualify/controls.rs",
    "src/bin/exp_rc021_host_qualify/decision.rs",
    "src/bin/exp_rc021_host_qualify/journal.rs",
    "src/bin/exp_rc021_host_qualify/host.rs",
    "src/bin/exp_rc021_host_qualify/manifest.rs",
    "src/bin/exp_rc021_host_qualify/protocol.rs",
    "src/bin/exp_rc021_host_qualify/provenance.rs",
    "src/bin/exp_rc021_host_qualify/session.rs",
    // Instrument code, not build tooling: the sole source of
    // `RC021_OPT_LEVEL` and `RC021_CARGO_ENCODED_RUSTFLAGS`, which decide
    // `is_optimised_build()` — the gate admitting both measuring modes — and
    // which supply the build facts P6 records durably. Outside this list an
    // edit making it emit `RC021_OPT_LEVEL=3` for an unoptimised binary would
    // leave the tree "clean", pass P6, and have the closure certify a build
    // that never happened.
    "build.rs",
];

// ======================================================================= ERROR

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ProvError {
    /// A `git` invocation failed, or produced output that is not valid UTF-8.
    Command {
        what: String,
        why: String,
    },
    /// §C15: exactly one adding commit is required.
    BirthNotUnique {
        found: usize,
    },
    NotFortyHex {
        what: String,
        value: String,
    },
    /// A binding document is not a strict ancestor of the instrument's birth.
    NotStrictAncestor {
        doc: &'static str,
        doc_commit: String,
    },
    BirthNotAncestorOfHead {
        birth: String,
        head: String,
    },
    Untracked(String),
    Dirty(String),
    /// §C4: exactly `[Amendment 1, Amendment 2]`.
    AmendmentShape {
        got: usize,
    },
}

impl std::fmt::Display for ProvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProvError::Command { what, why } => write!(f, "git {what} failed: {why}"),
            ProvError::BirthNotUnique { found } => write!(
                f,
                "instrument_birth_commit must have exactly one result, found {found}"
            ),
            ProvError::NotFortyHex { what, value } => {
                write!(f, "{what} must be 40 lower-case hex, got {value:?}")
            }
            ProvError::NotStrictAncestor { doc, doc_commit } => write!(
                f,
                "{doc} ({doc_commit}) is not a strict ancestor of the instrument birth commit"
            ),
            ProvError::BirthNotAncestorOfHead { birth, head } => {
                write!(f, "birth {birth} is not an ancestor of HEAD {head}")
            }
            ProvError::Untracked(p) => write!(f, "{p} is not tracked by git"),
            ProvError::Dirty(p) => write!(f, "{p} has uncommitted modifications"),
            ProvError::AmendmentShape { got } => {
                write!(f, "amendment_commits must hold exactly 2 SHAs, got {got}")
            }
        }
    }
}

// ======================================================================= FACTS

/// Everything the decision needs, gathered separately so the decision itself is
/// pure. `doc_commits` is in [`BINDING_DOCS`] order.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AncestryFacts {
    pub head: String,
    pub birth: String,
    pub doc_commits: [String; 3],
    /// For each binding document: is its commit a **strict** ancestor of birth?
    pub doc_strict_ancestor_of_birth: [bool; 3],
    pub birth_ancestor_of_head: bool,
    /// Paths that are not tracked, and paths that are tracked but modified.
    pub untracked: Vec<String>,
    pub dirty: Vec<String>,
}

/// What a passing provenance check yields.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProvenanceSnapshot {
    pub repo_commit: String,
    /// The pre-registration's own first-adding commit.
    pub prereg_commit: String,
    /// §C4: exactly `[Amendment 1, Amendment 2]`.
    pub amendment_commits: Vec<String>,
    pub instrument_birth_commit: String,
}

fn is_forty_hex(s: &str) -> bool {
    s.len() == 40
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// §C15, decided purely. Every check is an ancestry or a working-tree fact;
/// none is a date.
pub fn decide(f: &AncestryFacts) -> Result<ProvenanceSnapshot, ProvError> {
    for (what, v) in [("HEAD", &f.head), ("instrument_birth_commit", &f.birth)] {
        if !is_forty_hex(v) {
            return Err(ProvError::NotFortyHex {
                what: what.to_string(),
                value: v.clone(),
            });
        }
    }
    for (i, c) in f.doc_commits.iter().enumerate() {
        if !is_forty_hex(c) {
            return Err(ProvError::NotFortyHex {
                what: BINDING_DOCS[i].to_string(),
                value: c.clone(),
            });
        }
    }
    if let Some(p) = f.untracked.first() {
        return Err(ProvError::Untracked(p.clone()));
    }
    if let Some(p) = f.dirty.first() {
        return Err(ProvError::Dirty(p.clone()));
    }
    for (i, ok) in f.doc_strict_ancestor_of_birth.iter().enumerate() {
        if !ok {
            return Err(ProvError::NotStrictAncestor {
                doc: BINDING_DOCS[i],
                doc_commit: f.doc_commits[i].clone(),
            });
        }
    }
    if !f.birth_ancestor_of_head {
        return Err(ProvError::BirthNotAncestorOfHead {
            birth: f.birth.clone(),
            head: f.head.clone(),
        });
    }
    let amendment_commits = vec![f.doc_commits[1].clone(), f.doc_commits[2].clone()];
    if amendment_commits.len() != 2 {
        return Err(ProvError::AmendmentShape {
            got: amendment_commits.len(),
        });
    }
    Ok(ProvenanceSnapshot {
        repo_commit: f.head.clone(),
        prereg_commit: f.doc_commits[0].clone(),
        amendment_commits,
        instrument_birth_commit: f.birth.clone(),
    })
}

// ================================================================ GIT WRAPPER

fn git(repo: &Path, args: &[&str]) -> Result<String, ProvError> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| ProvError::Command {
            what: args.join(" "),
            why: e.to_string(),
        })?;
    if !out.status.success() {
        return Err(ProvError::Command {
            what: args.join(" "),
            why: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }
    String::from_utf8(out.stdout)
        .map(|s| s.trim().to_string())
        .map_err(|e| ProvError::Command {
            what: args.join(" "),
            why: format!("output is not UTF-8: {e}"),
        })
}

/// A git **predicate**, tri-state by exit code.
///
/// `git` uses exit 1 for "the answer is no" and 128 (or a signal) for "the
/// question was invalid". Collapsing both to `false` would silently turn a bad
/// revision, a broken repository or a killed process into a passing negative —
/// so only exit 1 is a legitimate `false`.
fn git_predicate(repo: &Path, args: &[&str]) -> Result<bool, ProvError> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| ProvError::Command {
            what: args.join(" "),
            why: e.to_string(),
        })?;
    match out.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        other => Err(ProvError::Command {
            what: args.join(" "),
            why: format!(
                "unexpected status {}: {}",
                other
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "terminated by signal".to_string()),
                String::from_utf8_lossy(&out.stderr).trim()
            ),
        }),
    }
}

/// §C15 verbatim: `git log --follow --diff-filter=A --format=%H -- <path>`,
/// which must yield **exactly one** result.
pub fn birth_commit(repo: &Path, path: &str) -> Result<String, ProvError> {
    let out = git(
        repo,
        &[
            "log",
            "--follow",
            "--diff-filter=A",
            "--format=%H",
            "--",
            path,
        ],
    )?;
    let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() != 1 {
        return Err(ProvError::BirthNotUnique { found: lines.len() });
    }
    Ok(lines[0].to_string())
}

/// The commit that first added a path, **without `--follow`**.
///
/// §C15 prescribes `--follow` for `main.rs` alone. Applying it to the binding
/// documents is actively wrong: `--follow` treats a similar file as a rename,
/// so two documents withnearly identical content can be attributed to each other's
/// commit. Exactly one adding commit is still required.
pub fn first_adding_commit(repo: &Path, path: &str) -> Result<String, ProvError> {
    let out = git(repo, &["log", "--diff-filter=A", "--format=%H", "--", path])?;
    let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() != 1 {
        return Err(ProvError::BirthNotUnique { found: lines.len() });
    }
    Ok(lines[0].to_string())
}

/// `a` is an ancestor of `b`, by `git merge-base --is-ancestor` and nothing
/// else.
pub fn is_ancestor(repo: &Path, a: &str, b: &str) -> Result<bool, ProvError> {
    git_predicate(repo, &["merge-base", "--is-ancestor", a, b])
}

/// **Strict**: an ancestor, and not the same commit.
pub fn is_strict_ancestor(repo: &Path, a: &str, b: &str) -> Result<bool, ProvError> {
    Ok(a != b && is_ancestor(repo, a, b)?)
}

fn is_tracked(repo: &Path, path: &str) -> Result<bool, ProvError> {
    git_predicate(repo, &["ls-files", "--error-unmatch", path])
}

fn is_clean(repo: &Path, path: &str) -> Result<bool, ProvError> {
    git_predicate(repo, &["diff", "--quiet", "HEAD", "--", path])
}

/// Gather every fact §C15 needs, then decide. The gathering is the only part
/// that touches git.
pub fn gather(repo: &Path) -> Result<AncestryFacts, ProvError> {
    let head = git(repo, &["rev-parse", "HEAD"])?;
    let birth = birth_commit(repo, BIRTH_PATH)?;

    let mut doc_commits: [String; 3] = Default::default();
    let mut strict: [bool; 3] = [false; 3];
    for (i, d) in BINDING_DOCS.iter().enumerate() {
        doc_commits[i] = first_adding_commit(repo, d)?;
        strict[i] = is_strict_ancestor(repo, &doc_commits[i], &birth)?;
    }

    let mut untracked = Vec::new();
    let mut dirty = Vec::new();
    for p in BINDING_DOCS.iter().chain(INSTRUMENT_FILES.iter()) {
        if !is_tracked(repo, p)? {
            untracked.push((*p).to_string());
        } else if !is_clean(repo, p)? {
            dirty.push((*p).to_string());
        }
    }

    Ok(AncestryFacts {
        head: head.clone(),
        birth: birth.clone(),
        doc_commits,
        doc_strict_ancestor_of_birth: strict,
        birth_ancestor_of_head: is_ancestor(repo, &birth, &head)?,
        untracked,
        dirty,
    })
}

/// The P6 gate: gather, then decide.
pub fn check(repo: &Path) -> Result<ProvenanceSnapshot, ProvError> {
    decide(&gather(repo)?)
}

// ======================================================================= TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    /// A throw-away repository. The working repo is never touched: before this
    /// commit lands its own sources are legitimately dirty, so testing against
    /// it would be both wrong and flaky.
    struct Fixture(std::path::PathBuf);

    impl Fixture {
        fn new(name: &str) -> Fixture {
            let p = std::env::temp_dir().join(format!(
                "rc021_prov_{}_{}_{}",
                std::process::id(),
                SEQ.fetch_add(1, Ordering::SeqCst),
                name
            ));
            std::fs::create_dir_all(&p).unwrap();
            let f = Fixture(p);
            f.run(&["init", "-q"]);
            f.run(&["config", "user.email", "t@example.invalid"]);
            f.run(&["config", "user.name", "t"]);
            f.run(&["config", "commit.gpgsign", "false"]);
            f
        }
        fn path(&self) -> &Path {
            &self.0
        }
        fn run(&self, args: &[&str]) -> String {
            let out = Command::new("git")
                .arg("-C")
                .arg(&self.0)
                .args(args)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            String::from_utf8(out.stdout).unwrap().trim().to_string()
        }
        fn write(&self, rel: &str, body: &str) {
            let p = self.0.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        }
        fn commit(&self, rel: &str, body: &str, msg: &str) -> String {
            self.write(rel, body);
            self.run(&["add", rel]);
            self.run(&["commit", "-q", "-m", msg]);
            self.run(&["rev-parse", "HEAD"])
        }
        /// docs first, then the instrument: the ordering §C15 requires.
        fn happy(name: &str) -> Fixture {
            let f = Fixture::new(name);
            // Distinct content per file: identical bodies would let a rename
            // heuristic conflate them.
            for d in BINDING_DOCS {
                f.commit(d, &format!("doc for {d}\n"), d);
            }
            for i in INSTRUMENT_FILES {
                f.commit(i, &format!("// code for {i}\n"), i);
            }
            f
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn happy_ancestry_passes() {
        let f = Fixture::happy("happy");
        let facts = gather(f.path()).unwrap();
        assert!(facts.doc_strict_ancestor_of_birth.iter().all(|b| *b));
        assert!(facts.birth_ancestor_of_head);
        assert!(facts.untracked.is_empty() && facts.dirty.is_empty());

        let snap = decide(&facts).unwrap();
        assert_eq!(snap.repo_commit, facts.head);
        assert_eq!(snap.instrument_birth_commit, facts.birth);
        assert_eq!(snap.prereg_commit, facts.doc_commits[0]);
        // §C4: exactly Amendment 1 then Amendment 2.
        assert_eq!(snap.amendment_commits.len(), 2);
        assert_eq!(snap.amendment_commits[0], facts.doc_commits[1]);
        assert_eq!(snap.amendment_commits[1], facts.doc_commits[2]);
        assert_ne!(snap.amendment_commits[0], snap.amendment_commits[1]);
        assert_eq!(check(f.path()).unwrap(), snap);
    }

    #[test]
    fn document_after_the_instrument_is_not_a_strict_ancestor() {
        let f = Fixture::new("late_doc");
        // instrument first, amendment 2 afterwards
        f.commit(BINDING_DOCS[0], "prereg body\n", "prereg");
        f.commit(BINDING_DOCS[1], "a1 body\n", "a1");
        for i in INSTRUMENT_FILES {
            f.commit(i, &format!("// {i}\n"), i);
        }
        f.commit(BINDING_DOCS[2], "a2 body\n", "a2 late");

        let facts = gather(f.path()).unwrap();
        assert!(!facts.doc_strict_ancestor_of_birth[2]);
        assert!(matches!(
            decide(&facts),
            Err(ProvError::NotStrictAncestor {
                doc: "research/PREREG_RC021_AMENDMENT_2.md",
                ..
            })
        ));
    }

    #[test]
    fn a_document_equal_to_the_birth_commit_is_not_strict() {
        let f = Fixture::new("same_commit");
        // main.rs and amendment 2 land in the SAME commit: an ancestor test
        // would pass, a strict one must not.
        f.commit(BINDING_DOCS[0], "prereg body\n", "prereg");
        f.commit(BINDING_DOCS[1], "a1 body\n", "a1");
        f.write(BINDING_DOCS[2], "a2 body\n");
        f.write(BIRTH_PATH, "// main\n");
        f.run(&["add", "."]);
        f.run(&["commit", "-q", "-m", "doc and code together"]);
        for i in INSTRUMENT_FILES.iter().skip(1) {
            f.commit(i, &format!("// {i}\n"), i);
        }
        let facts = gather(f.path()).unwrap();
        assert_eq!(facts.doc_commits[2], facts.birth, "same commit");
        assert!(!facts.doc_strict_ancestor_of_birth[2]);
        assert!(matches!(
            decide(&facts),
            Err(ProvError::NotStrictAncestor { .. })
        ));
    }

    #[test]
    fn birth_not_ancestor_of_head_is_refused() {
        let f = Fixture::happy("branch");
        let facts = gather(f.path()).unwrap();
        // Construct the fact directly: a birth on an unrelated line of history.
        let mut bad = facts.clone();
        bad.birth_ancestor_of_head = false;
        assert!(matches!(
            decide(&bad),
            Err(ProvError::BirthNotAncestorOfHead { .. })
        ));
    }

    #[test]
    fn missing_or_ambiguous_birth_is_refused() {
        // missing: the path was never added
        let f = Fixture::new("missing_birth");
        f.commit(BINDING_DOCS[0], "prereg body\n", "prereg");
        assert!(matches!(
            birth_commit(f.path(), BIRTH_PATH),
            Err(ProvError::BirthNotUnique { found: 0 })
        ));
        assert!(gather(f.path()).is_err());

        // ambiguous: added, deleted, added again yields two adding commits
        let g = Fixture::new("ambiguous_birth");
        g.commit(BINDING_DOCS[0], "prereg body\n", "prereg");
        g.commit(BIRTH_PATH, "// v1\n", "add");
        g.run(&["rm", "-q", BIRTH_PATH]);
        g.run(&["commit", "-q", "-m", "remove"]);
        g.commit(BIRTH_PATH, "// v2\n", "add again");
        assert!(matches!(
            birth_commit(g.path(), BIRTH_PATH),
            Err(ProvError::BirthNotUnique { found: 2 })
        ));
    }

    #[test]
    fn untracked_and_dirty_files_are_refused() {
        // untracked instrument file
        let f = Fixture::new("untracked");
        for d in BINDING_DOCS {
            f.commit(d, &format!("doc {d}\n"), d);
        }
        f.commit(BIRTH_PATH, "// main\n", "birth");
        // Every instrument file but the last, which stays untracked. Indexed
        // from the end so adding a module cannot silently leave two untracked
        // and turn this into a different test.
        let last = INSTRUMENT_FILES.len() - 1;
        for i in INSTRUMENT_FILES.iter().take(last).skip(1) {
            f.commit(i, &format!("// {i}\n"), i);
        }
        f.write(INSTRUMENT_FILES[last], "// never added\n");
        let facts = gather(f.path()).unwrap();
        assert_eq!(facts.untracked, vec![INSTRUMENT_FILES[last].to_string()]);
        assert!(matches!(decide(&facts), Err(ProvError::Untracked(_))));

        // tracked but modified
        let g = Fixture::happy("dirty");
        g.write(BINDING_DOCS[1], "modified after commit\n");
        let facts = gather(g.path()).unwrap();
        assert!(facts.untracked.is_empty());
        assert_eq!(facts.dirty, vec![BINDING_DOCS[1].to_string()]);
        assert!(matches!(decide(&facts), Err(ProvError::Dirty(_))));
    }

    #[test]
    fn command_failure_is_an_explicit_refusal_not_a_fallback() {
        let d = std::env::temp_dir().join(format!(
            "rc021_prov_notrepo_{}_{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&d).unwrap();
        let r = gather(&d);
        assert!(matches!(r, Err(ProvError::Command { .. })), "got {r:?}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn malformed_commit_ids_are_refused() {
        let f = Fixture::happy("hex");
        let good = gather(f.path()).unwrap();
        for mutate in [
            (|x: &mut AncestryFacts| x.head = "short".into()) as fn(&mut AncestryFacts),
            |x: &mut AncestryFacts| x.birth = "Z".repeat(40),
            |x: &mut AncestryFacts| x.doc_commits[1] = "a".repeat(39),
            |x: &mut AncestryFacts| x.doc_commits[0] = "A".repeat(40), // upper case
        ] {
            let mut bad = good.clone();
            mutate(&mut bad);
            assert!(matches!(decide(&bad), Err(ProvError::NotFortyHex { .. })));
        }
    }

    #[test]
    fn ancestry_uses_merge_base_not_dates() {
        let f = Fixture::happy("no_dates");
        let facts = gather(f.path()).unwrap();
        // A commit is its own ancestor but never its own STRICT ancestor.
        assert!(is_ancestor(f.path(), &facts.birth, &facts.birth).unwrap());
        assert!(!is_strict_ancestor(f.path(), &facts.birth, &facts.birth).unwrap());
        // and the direction is not symmetric
        assert!(is_strict_ancestor(f.path(), &facts.doc_commits[0], &facts.birth).unwrap());
        assert!(!is_strict_ancestor(f.path(), &facts.birth, &facts.doc_commits[0]).unwrap());
    }

    /// A git predicate is tri-state: 0 true, 1 false, anything else a failure
    /// to answer. Folding "could not answer" into "false" would silently pass
    /// an ancestry check that was never actually performed.
    #[test]
    fn git_predicate_is_tri_state() {
        let f = Fixture::new("tristate");
        let a = f.commit("a.txt", "a\n", "a");
        let b = f.commit("b.txt", "b\n", "b");

        assert!(git_predicate(f.path(), &["merge-base", "--is-ancestor", &a, &b]).unwrap());
        assert!(!git_predicate(f.path(), &["merge-base", "--is-ancestor", &b, &a]).unwrap());

        // An unknown revision makes git exit 128: it did not answer "no", it
        // failed to answer at all.
        let err = git_predicate(
            f.path(),
            &["merge-base", "--is-ancestor", &"f".repeat(40), &b],
        )
        .unwrap_err();
        match err {
            ProvError::Command { what, why } => {
                assert!(what.contains("--is-ancestor"), "{what}");
                assert!(why.contains("unexpected status 128"), "{why}");
            }
            other => panic!("expected a command failure, got {other}"),
        }

        // ls-files on an absent path is a real "no", not a failure.
        assert!(!git_predicate(f.path(), &["ls-files", "--error-unmatch", "absent.txt"]).unwrap());
        assert!(git_predicate(f.path(), &["ls-files", "--error-unmatch", "a.txt"]).unwrap());
    }
}
