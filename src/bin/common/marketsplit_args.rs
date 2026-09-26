//! Shared, fail-fast CLI parsing for experimental lattice hunters.

use std::io;
use std::path::PathBuf;

pub struct HuntArgs {
    pub instance: Option<PathBuf>,
    pub timeout_secs: Option<u64>,
}

pub fn parse(args: &[String]) -> io::Result<HuntArgs> {
    let mut parsed = HuntArgs {
        instance: None,
        timeout_secs: None,
    };
    let mut timeout_seen = false;
    let mut continuous_seen = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--instance" => {
                let value = args.get(i + 1).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "--instance requires a path")
                })?;
                parsed.instance = Some(PathBuf::from(value));
                i += 2;
            }
            "--timeout" => {
                if continuous_seen {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--timeout conflicts with --continuous",
                    ));
                }
                let value = args.get(i + 1).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "--timeout requires seconds")
                })?;
                let seconds = value.parse::<u64>().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--timeout needs positive integer seconds",
                    )
                })?;
                if seconds == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--timeout must be greater than zero",
                    ));
                }
                parsed.timeout_secs = Some(seconds);
                timeout_seen = true;
                i += 2;
            }
            "--continuous" => {
                if timeout_seen {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--continuous conflicts with --timeout",
                    ));
                }
                continuous_seen = true;
                i += 1;
            }
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown argument: {other}"),
                ));
            }
        }
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn bounded_and_continuous_modes_are_explicit() {
        let bounded = parse(&args(&["bin", "--instance", "case.dat", "--timeout", "30"])).unwrap();
        assert_eq!(bounded.timeout_secs, Some(30));
        assert_eq!(bounded.instance, Some(PathBuf::from("case.dat")));
        assert!(parse(&args(&["bin", "--continuous"]))
            .unwrap()
            .timeout_secs
            .is_none());
    }

    #[test]
    fn malformed_timeout_never_becomes_continuous() {
        for values in [
            vec!["bin", "--timeout", "30s"],
            vec!["bin", "--timeout", "0"],
            vec!["bin", "--timeout"],
            vec!["bin", "--timeout", "30", "--continuous"],
            vec!["bin", "--unknown"],
        ] {
            assert_eq!(
                parse(&args(&values)).err().unwrap().kind(),
                io::ErrorKind::InvalidInput
            );
        }
    }
}
