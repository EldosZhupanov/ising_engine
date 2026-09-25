//! Fail-closed official-checker gate for experimental Market Split binaries.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

pub enum CertificateStatus {
    Verified(PathBuf),
    Rejected(PathBuf),
}

pub fn certify(
    instance: &Path,
    name: &str,
    bits: &[i8],
    checker: &Path,
    solutions_dir: &Path,
    candidates_dir: &Path,
    rejected_dir: &Path,
) -> io::Result<CertificateStatus> {
    if bits.is_empty() || bits.iter().any(|&bit| bit != 0 && bit != 1) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Market Split candidate must be a nonempty binary vector",
        ));
    }
    std::fs::create_dir_all(candidates_dir)?;
    let bit_string: String = bits
        .iter()
        .map(|&bit| char::from(b'0' + bit as u8))
        .collect();
    let candidate_name = format!("{name}--{bit_string}.sol");
    let candidate = candidates_dir.join(&candidate_name);
    let contents = bits.iter().map(i8::to_string).collect::<Vec<_>>().join(" ") + "\n";
    std::fs::write(&candidate, contents)?;

    if !checker.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "official checker missing: {}; candidate retained at {}",
                checker.display(),
                candidate.display()
            ),
        ));
    }

    let output = Command::new(checker)
        .arg(instance)
        .arg(&candidate)
        .output()?;
    if !output.status.success() {
        std::fs::create_dir_all(rejected_dir)?;
        let rejected = rejected_dir.join(candidate_name);
        if rejected.exists() && std::fs::read(&rejected)? != std::fs::read(&candidate)? {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "would overwrite a different rejected candidate: {}",
                    rejected.display()
                ),
            ));
        }
        std::fs::rename(&candidate, &rejected)?;
        eprintln!(
            "official Market Split checker rejected {} (exit {:?}): {}{}",
            rejected.display(),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return Ok(CertificateStatus::Rejected(rejected));
    }

    std::fs::create_dir_all(solutions_dir)?;
    let verified = solutions_dir.join(format!("{name}.sol"));
    if verified.exists() {
        if std::fs::read(&verified)? != std::fs::read(&candidate)? {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "would overwrite a different verified solution: {}",
                    verified.display()
                ),
            ));
        }
    } else {
        std::fs::rename(&candidate, &verified)?;
    }
    Ok(CertificateStatus::Verified(verified))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    fn fixture() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "ising-marketsplit-cert-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn rejected_candidate_never_enters_solutions() {
        let dir = fixture();
        let instance = dir.join("instance.dat");
        std::fs::write(&instance, "dummy").unwrap();
        let accepted = certify(
            &instance,
            "case",
            &[0, 1],
            Path::new("/bin/false"),
            &dir.join("solutions"),
            &dir.join("candidates"),
            &dir.join("rejected"),
        )
        .unwrap();
        assert!(matches!(accepted, CertificateStatus::Rejected(_)));
        assert!(!dir.join("solutions/case.sol").exists());
        assert!(dir.join("rejected/case--01.sol").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn successful_checker_promotes_candidate() {
        let dir = fixture();
        let instance = dir.join("instance.dat");
        std::fs::write(&instance, "dummy").unwrap();
        let accepted = certify(
            &instance,
            "case",
            &[1, 0],
            Path::new("/bin/true"),
            &dir.join("solutions"),
            &dir.join("candidates"),
            &dir.join("rejected"),
        )
        .unwrap();
        assert!(matches!(accepted, CertificateStatus::Verified(_)));
        assert_eq!(
            std::fs::read_to_string(dir.join("solutions/case.sol")).unwrap(),
            "1 0\n"
        );
        assert!(!dir.join("candidates/case--10.sol").exists());
        assert!(!dir.join("rejected/case--10.sol").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_checker_retains_uncertified_candidate() {
        let dir = fixture();
        let instance = dir.join("instance.dat");
        std::fs::write(&instance, "dummy").unwrap();
        let result = certify(
            &instance,
            "case",
            &[0, 1],
            &dir.join("missing-checker"),
            &dir.join("solutions"),
            &dir.join("candidates"),
            &dir.join("rejected"),
        );
        assert_eq!(result.err().unwrap().kind(), io::ErrorKind::NotFound);
        assert!(dir.join("candidates/case--01.sol").exists());
        assert!(!dir.join("solutions/case.sol").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
