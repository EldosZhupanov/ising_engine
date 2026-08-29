//! Capture the build facts that only a build script can see.
//!
//! `option_env!`/`cfg!` in the crate cannot observe how this binary was
//! compiled: `.cargo/config.toml` passes rustflags as rustc *arguments*, not
//! environment, and in this workspace `cfg!(debug_assertions)` is `false` in
//! every profile — so the release guard that RC-021 relied on was dead code and
//! `build_flags` recorded `<none>` for a binary built with `-Ctarget-cpu=native`.
//!
//! Cargo does set `PROFILE`, `OPT_LEVEL`, `DEBUG` and `CARGO_ENCODED_RUSTFLAGS`
//! for build scripts. Recording them is the only way an instrument whose whole
//! purpose is certifying timings can state truthfully how it was built.

fn main() {
    for key in ["PROFILE", "OPT_LEVEL", "DEBUG", "CARGO_ENCODED_RUSTFLAGS"] {
        let value = std::env::var(key).unwrap_or_else(|_| "UNAVAILABLE".to_string());
        println!("cargo:rustc-env=RC021_{key}={value}");
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
    println!("cargo:rerun-if-env-changed=RUSTFLAGS");
}
