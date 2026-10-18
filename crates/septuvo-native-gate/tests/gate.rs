//! End-to-end checks of the native-code gate on throw-away Cargo workspaces
//! (N4-01 acceptance: the gate must fail when a crate that compiles C is
//! added). Only path dependencies are used, so nothing is downloaded.

use septuvo_native_gate::{Allowlist, check_artifacts, check_workspace};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TARGET: &str = "x86_64-unknown-linux-gnu";

fn cargo() -> PathBuf {
    std::env::var_os("CARGO").map_or_else(|| "cargo".into(), PathBuf::from)
}

struct Workspace(PathBuf);

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A workspace with an `app` crate; `with_c` adds a dependency that compiles
/// C in its build script, declares `links`, and has a build-tool dependency.
fn workspace(name: &str, with_c: bool) -> Workspace {
    let root =
        std::env::temp_dir().join(format!("septuvo-native-gate-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let dep = if with_c {
        "evil-sys = { path = \"../evil-sys\" }"
    } else {
        "pure = { path = \"../pure\" }"
    };
    write(
        &root.join("app/Cargo.toml"),
        &format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n{dep}\n\n[workspace]\n"
        ),
    );
    write(&root.join("app/src/main.rs"), "fn main() {}\n");
    write(
        &root.join("pure/Cargo.toml"),
        "[package]\nname = \"pure\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(&root.join("pure/src/lib.rs"), "pub fn f() -> u8 { 1 }\n");
    write(
        &root.join("evil-sys/Cargo.toml"),
        "[package]\nname = \"evil-sys\"\nversion = \"0.1.0\"\nedition = \"2024\"\nlinks = \"evil\"\nbuild = \"build.rs\"\n\n\
         [build-dependencies]\ncc = { path = \"../cc\" }\n",
    );
    write(
        &root.join("evil-sys/src/lib.rs"),
        "pub fn f() -> u8 { 1 }\n",
    );
    write(
        &root.join("evil-sys/evil.c"),
        "int evil(void) { return 1; }\n",
    );
    write(
        &root.join("evil-sys/build.rs"),
        "fn main() {\n    let out = std::env::var(\"OUT_DIR\").unwrap();\n    \
         let ok = std::process::Command::new(\"cc\").args([\"-c\", \"evil.c\", \"-o\"]).arg(format!(\"{out}/evil.o\")).status();\n    \
         assert!(ok.map(|s| s.success()).unwrap_or(false), \"C compiler failed\");\n    \
         let ok = std::process::Command::new(\"ar\").current_dir(&out).args([\"rcs\", \"libevil.a\", \"evil.o\"]).status();\n    \
         assert!(ok.map(|s| s.success()).unwrap_or(false), \"ar failed\");\n    \
         println!(\"cargo:rustc-link-search=native={out}\");\n    \
         println!(\"cargo:rustc-link-lib=static=evil\");\n}\n",
    );
    // A stand-in for the real `cc` crate: the gate judges it by name.
    write(
        &root.join("cc/Cargo.toml"),
        "[package]\nname = \"cc\"\nversion = \"1.0.0\"\nedition = \"2024\"\n",
    );
    write(&root.join("cc/src/lib.rs"), "pub fn build() {}\n");
    let status = Command::new(cargo())
        .args(["generate-lockfile", "--offline", "--manifest-path"])
        .arg(root.join("app/Cargo.toml"))
        .status()
        .unwrap();
    assert!(status.success());
    Workspace(root)
}

#[test]
fn a_pure_rust_workspace_passes() {
    let w = workspace("pure", false);
    let r = check_workspace(&w.0.join("app/Cargo.toml"), TARGET, &Allowlist::default()).unwrap();
    assert!(r.passed(), "{:?}", r.violations);
    assert_eq!(r.crates, 2);
}

#[test]
fn adding_a_crate_that_compiles_c_fails_the_gate() {
    let w = workspace("c", true);
    let r = check_workspace(&w.0.join("app/Cargo.toml"), TARGET, &Allowlist::default()).unwrap();
    assert!(!r.passed());
    let all = r.violations.join("\n");
    for expected in [
        "evil-sys 0.1.0: declares `links = \"evil\"`",
        "evil-sys 0.1.0: build dependency `cc`",
        "evil-sys 0.1.0: build script runs `cc`",
        "evil-sys 0.1.0: build script runs `ar`",
        "evil-sys 0.1.0: links library `evil`",
    ] {
        assert!(all.contains(expected), "missing {expected:?} in:\n{all}");
    }
    assert!(
        r.inert_native.iter().any(|n| n.contains("evil.c")),
        "{:?}",
        r.inert_native
    );
}

#[test]
fn compiled_native_objects_are_found_after_a_build() {
    let w = workspace("built", true);
    if Command::new("cc").arg("--version").output().is_err() {
        // Reported, not hidden: without a C compiler this case cannot run.
        eprintln!("SKIPPED: no C compiler on this machine to build the deliberately bad fixture");
        return;
    }
    let target_dir = w.0.join("target");
    let status = Command::new(cargo())
        .args(["build", "--offline", "--quiet", "--manifest-path"])
        .arg(w.0.join("app/Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir)
        .status()
        .unwrap();
    assert!(status.success());
    let list = Allowlist::parse("needed libc.so.6 -- C library\n").unwrap();
    let binary = target_dir.join("debug/app");
    let r = check_artifacts(&target_dir, &[binary], &list).unwrap();
    assert!(
        r.violations
            .iter()
            .any(|v| v.contains("produced the native object") && v.contains("evil.o")),
        "{:?}",
        r.violations
    );
}
