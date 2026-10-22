//! `septuvo-native-gate`: the native-code review gate (D-005, D-015, N4-01).
//!
//! ```text
//! septuvo-native-gate deps --manifest-path desktop/Cargo.toml --allowlist desktop/native-allowlist.txt [--target T]
//! septuvo-native-gate artifacts --allowlist desktop/native-allowlist.txt --target-dir desktop/target BINARY...
//! ```
//!
//! Exit status: 0 pass, 1 the rule is broken, 2 usage or I/O error.

use septuvo_native_gate::{Allowlist, Report, check_artifacts, check_workspace};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage:
  septuvo-native-gate deps --manifest-path PATH --allowlist PATH [--target TRIPLE]
  septuvo-native-gate artifacts --allowlist PATH --target-dir PATH BINARY...";

const DEFAULT_TARGET: &str = "x86_64-unknown-linux-gnu";

struct Args {
    mode: String,
    manifest: Option<PathBuf>,
    allowlist: Option<PathBuf>,
    target: String,
    target_dir: Option<PathBuf>,
    binaries: Vec<PathBuf>,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let mut it = args.iter();
    let mode = it.next().ok_or("missing mode")?.clone();
    if mode != "deps" && mode != "artifacts" {
        return Err(format!("unknown mode `{mode}`"));
    }
    let mut a = Args {
        mode,
        manifest: None,
        allowlist: None,
        target: DEFAULT_TARGET.into(),
        target_dir: None,
        binaries: Vec::new(),
    };
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().cloned().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--manifest-path" => a.manifest = Some(value(arg)?.into()),
            "--allowlist" => a.allowlist = Some(value(arg)?.into()),
            "--target" => a.target = value(arg)?,
            "--target-dir" => a.target_dir = Some(value(arg)?.into()),
            s if s.starts_with('-') => return Err(format!("unknown option `{s}`")),
            s => a.binaries.push(s.into()),
        }
    }
    if a.allowlist.is_none() {
        return Err("--allowlist is required".into());
    }
    match a.mode.as_str() {
        "deps" if a.manifest.is_none() || !a.binaries.is_empty() => {
            Err("deps needs --manifest-path and no binaries".into())
        }
        "artifacts" if a.target_dir.is_none() || a.binaries.is_empty() => {
            Err("artifacts needs --target-dir and at least one binary".into())
        }
        _ => Ok(a),
    }
}

fn print(title: &str, unit: &str, r: &Report) {
    println!("Native-code gate ({title}): {} {unit} inspected", r.crates);
    for a in &r.allowed {
        println!("  allowed: {a}");
    }
    for n in &r.inert_native {
        println!("  inert:   {n}");
    }
    if !r.os_interfaces.is_empty() {
        let list: Vec<_> = r.os_interfaces.iter().map(String::as_str).collect();
        println!("  Reviewed native libraries: {}", list.join(", "));
    }
    for v in &r.violations {
        println!("  VIOLATION: {v}");
    }
    if r.passed() {
        println!(
            "PASS: every non-Rust native component is reviewed and listed (D-015); nothing is compiled from non-Rust sources."
        );
    } else {
        println!(
            "FAIL: {} unreviewed non-Rust native component(s) (D-015).",
            r.violations.len()
        );
    }
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse(&raw) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("septuvo-native-gate: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let allowlist_path = args.allowlist.clone().unwrap_or_default();
    let list = match std::fs::read_to_string(&allowlist_path)
        .map_err(|e| e.to_string())
        .and_then(|t| Allowlist::parse(&t))
    {
        Ok(l) => l,
        Err(e) => {
            eprintln!("septuvo-native-gate: {}: {e}", allowlist_path.display());
            return ExitCode::from(2);
        }
    };
    let result = match args.mode.as_str() {
        "deps" => check_workspace(&args.manifest.unwrap_or_default(), &args.target, &list)
            .map(|r| ("dependencies", "crate(s)", r)),
        _ => check_artifacts(&args.target_dir.unwrap_or_default(), &args.binaries, &list)
            .map(|r| ("built artifacts", "binary(ies)", r)),
    };
    match result {
        Ok((title, unit, report)) => {
            print(title, unit, &report);
            if report.passed() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("septuvo-native-gate: {e}");
            ExitCode::from(2)
        }
    }
}
