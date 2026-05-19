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
        .stderr(predicate::str::contains("Usage:"));
}

#[test]
fn outlines_multiple_files_in_input_order() {
    let directory = tempdir().unwrap();
    let markdown = directory.path().join("doc.md");
    let python = directory.path().join("sample.py");
    fs::write(&markdown, "# Intro\nbody\n").unwrap();
    fs::write(&python, "def run():\n    return 1\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command.args([markdown.as_os_str(), python.as_os_str()]);

    command.assert().success().stdout(
        predicate::str::contains(format!("# {}", markdown.display()))
            .and(predicate::str::contains("- Intro [L1-L2]"))
            .and(predicate::str::contains(format!("# {}", python.display())))
            .and(predicate::str::contains("- def run [L1-L2]")),
    );
}

#[test]
fn applies_max_depth_to_cli_output() {
    let directory = tempdir().unwrap();
    let markdown = directory.path().join("doc.md");
    fs::write(&markdown, "# Root\n## Child\n### Hidden\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command.args(["--max-depth", "2"]).arg(&markdown);

    command
        .assert()
        .success()
        .stdout(predicate::str::contains("- Root [L1-L3]"))
        .stdout(predicate::str::contains("  - Child [L2-L3]"))
        .stdout(predicate::str::contains("Hidden").not());
}

#[test]
fn rejects_directories_and_unsupported_files() {
    let directory = tempdir().unwrap();
    let unsupported = directory.path().join("notes.txt");
    fs::write(&unsupported, "hello").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command.args([directory.path().as_os_str(), unsupported.as_os_str()]);

    command.assert().failure().stderr(
        predicate::str::contains("is a directory")
            .and(predicate::str::contains("unsupported file type")),
    );
}

#[test]
fn processes_remaining_files_after_a_failure() {
    let directory = tempdir().unwrap();
    let missing = directory.path().join("missing.md");
    let markdown = directory.path().join("doc.md");
    fs::write(&markdown, "# Kept\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command.args([missing.as_os_str(), markdown.as_os_str()]);

    command
        .assert()
        .failure()
        .stdout(predicate::str::contains("- Kept [L1]"))
        .stderr(predicate::str::contains(format!("{}", missing.display())));
}
