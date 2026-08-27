use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn exits_nonzero_without_input_files() {
    let mut command = Command::cargo_bin("wot").unwrap();

    command
        .assert()
        .failure()
        .stderr(predicate::str::contains("no input files provided"));
}

#[test]
fn reports_package_version() {
    let mut command = Command::cargo_bin("wot").unwrap();

    command
        .arg("--version")
        .assert()
        .success()
        .stdout(format!("wot {}\n", env!("CARGO_PKG_VERSION")));
}

#[test]
fn outlines_multiple_files_in_input_order() {
    let directory = tempdir().unwrap();
    let markdown = directory.path().join("doc.md");
    let python = directory.path().join("sample.py");
    fs::write(&markdown, "# Intro\nbody\n").unwrap();
    fs::write(&python, "def run():\n    return 1\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .args(["--min-lines", "0"])
        .args([markdown.as_os_str(), python.as_os_str()]);

    command.assert().success().stdout(
        predicate::str::contains("- Intro [L1-L2]")
            .and(predicate::str::contains("- def run [L1-L2]")),
    );
}

#[test]
fn outlines_org_files_from_the_cli() {
    let directory = tempdir().unwrap();
    let org = directory.path().join("notes.org");
    fs::write(&org, "* Root\nbody\n** TODO Child :tag:\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command.args(["--min-lines", "0"]).arg(&org);

    command.assert().success().stdout(
        predicate::str::contains("- Root [L1-L3]")
            .and(predicate::str::contains("  - TODO Child :tag: [L3]")),
    );
}

#[test]
fn outlines_structured_files_from_the_cli() {
    let directory = tempdir().unwrap();
    let yaml = directory.path().join("config.yaml");
    let toml = directory.path().join("settings.toml");
    let dockerfile = directory.path().join("Dockerfile");
    fs::write(&yaml, "service:\n  image: nginx\n").unwrap();
    fs::write(&toml, "[package]\nname = \"wot\"\n").unwrap();
    fs::write(&dockerfile, "FROM scratch\nCOPY app /app\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command.args(["--min-lines", "0"]).args([
        yaml.as_os_str(),
        toml.as_os_str(),
        dockerfile.as_os_str(),
    ]);

    command.assert().success().stdout(
        predicate::str::contains("- service: object [L1-L2]")
            .and(predicate::str::contains("  - name: \"wot\" [L2]"))
            .and(predicate::str::contains("- FROM scratch [L1-L2]")),
    );
}

#[test]
fn outlines_tree_sitter_code_files_from_the_cli() {
    let directory = tempdir().unwrap();
    let rust = directory.path().join("lib.rs");
    let shell = directory.path().join("build.sh");
    fs::write(&rust, "pub struct App;\n\npub fn run() {}\n").unwrap();
    fs::write(&shell, "build() {\n  echo build\n}\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .args(["--min-lines", "0"])
        .args([rust.as_os_str(), shell.as_os_str()]);

    command.assert().success().stdout(
        predicate::str::contains("- struct App [L1]")
            .and(predicate::str::contains("- fn run [L3]"))
            .and(predicate::str::contains("- function build [L1-L3]")),
    );
}

#[test]
fn applies_max_depth_to_cli_output() {
    let directory = tempdir().unwrap();
    let markdown = directory.path().join("doc.md");
    fs::write(&markdown, "# Root\n## Child\n### Hidden\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .args(["--max-depth", "2", "--min-lines", "0"])
        .arg(&markdown);

    command
        .assert()
        .success()
        .stdout(predicate::str::contains("- Root [L1-L3]"))
        .stdout(predicate::str::contains("  - Child [L2-L3]"))
        .stdout(predicate::str::contains("Hidden").not());
}

#[test]
fn recurses_directories_and_rejects_unsupported_explicit_files() {
    let directory = tempdir().unwrap();
    let supported = directory.path().join("kept.rs");
    let unsupported = directory.path().join("notes.txt");
    fs::write(&supported, "pub fn kept() {}\n").unwrap();
    fs::write(&unsupported, "hello").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .args(["--min-lines", "0"])
        .args([directory.path().as_os_str(), unsupported.as_os_str()]);

    command
        .assert()
        .failure()
        .stdout(predicate::str::contains("- fn kept [L1]"))
        .stderr(predicate::str::contains("unsupported file type"));
}

#[test]
fn processes_remaining_files_after_a_failure() {
    let directory = tempdir().unwrap();
    let missing = directory.path().join("missing.md");
    let markdown = directory.path().join("doc.md");
    fs::write(&markdown, "# Kept\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .args(["--min-lines", "0"])
        .args([missing.as_os_str(), markdown.as_os_str()]);

    command
        .assert()
        .failure()
        .stdout(predicate::str::contains("- Kept [L1]"))
        .stderr(predicate::str::contains(format!("{}", missing.display())));
}
