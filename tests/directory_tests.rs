use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn renders_an_integrated_tree_and_marks_the_walk_boundary() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("project");
    fs::create_dir_all(root.join("src/deep")).unwrap();
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn visible() {}\n").unwrap();
    fs::write(root.join("src/deep/hidden.rs"), "pub fn hidden() {}\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    let output = command
        .args(["--walk-depth", "2", "--min-lines", "0"])
        .arg(&root)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();

    let expected = format!(
        "- `{}/`\n  - `Cargo.toml`\n    - package [L1-L2]\n      - name: \"demo\" [L2]\n  - `src/`\n    - `deep/` *(not expanded: walk depth limit 2)*\n    - `lib.rs`\n      - fn visible [L1]\n",
        root.display()
    );
    assert_eq!(output, expected);
    assert!(!output.contains("fn hidden"));
}

#[test]
fn default_walk_depth_is_three_and_zero_leaves_the_root_unexpanded() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("project");
    fs::create_dir_all(root.join("one/two/three")).unwrap();
    fs::write(root.join("one/two/visible.rs"), "pub fn visible() {}\n").unwrap();
    fs::write(root.join("one/two/three/hidden.rs"), "pub fn hidden() {}\n").unwrap();

    let mut default_command = Command::cargo_bin("wot").unwrap();
    default_command
        .args(["--min-lines", "0"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("- fn visible [L1]"))
        .stdout(predicate::str::contains(
            "`three/` *(not expanded: walk depth limit 3)*",
        ))
        .stdout(predicate::str::contains("fn hidden").not());

    let mut zero_command = Command::cargo_bin("wot").unwrap();
    zero_command
        .args(["--walk-depth", "0", "--min-lines", "0"])
        .arg(&root)
        .assert()
        .success()
        .stdout(format!(
            "- `{}/` *(not expanded: walk depth limit 0)*\n",
            root.display()
        ));
}

#[test]
fn includes_hidden_supported_files_respects_ignores_and_skips_unsupported_files() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("project");
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join(".github/workflows")).unwrap();
    fs::create_dir_all(root.join("ignored")).unwrap();
    fs::write(root.join(".gitignore"), "ignored/\n").unwrap();
    fs::write(root.join(".git/config.toml"), "[secret]\nvalue = 1\n").unwrap();
    fs::write(root.join(".github/workflows/ci.yml"), "name: CI\n").unwrap();
    fs::write(root.join("ignored/ignored.rs"), "pub fn ignored() {}\n").unwrap();
    fs::write(root.join("notes.txt"), "unsupported\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .args(["--min-lines", "0"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("`.github/`"))
        .stdout(predicate::str::contains("`ci.yml`"))
        .stdout(predicate::str::contains("name: \"CI\""))
        .stdout(predicate::str::contains("config.toml").not())
        .stdout(predicate::str::contains("fn ignored").not())
        .stdout(predicate::str::contains("notes.txt").not());
}

#[test]
fn nests_short_verbatim_files_but_redacts_discovered_dotenv_secrets() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join(".env"), "APP_NAME=wot\nAPI_TOKEN=secret-token\n").unwrap();
    fs::write(root.join("notes.md"), "# Notes\nbody\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("- `.env`\n"))
        .stdout(predicate::str::contains("API_TOKEN: <redacted>"))
        .stdout(predicate::str::contains("secret-token").not())
        .stdout(predicate::str::contains(
            "- `notes.md`\n    ```markdown\n    # Notes\n    body\n    ```\n",
        ));
}

#[test]
fn json_preserves_files_and_adds_directory_tree_metadata() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("project");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("root.rs"), "pub fn root() {}\n").unwrap();
    fs::write(root.join("src/hidden.rs"), "pub fn hidden() {}\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    let output = command
        .args(["--format", "json", "--walk-depth", "1", "--min-lines", "0"])
        .arg(&root)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(json["files"].as_array().unwrap().len(), 1);
    assert_eq!(
        json["files"][0]["path"],
        root.join("root.rs").display().to_string()
    );
    assert!(json["errors"].as_array().unwrap().is_empty());
    assert_eq!(json["directories"].as_array().unwrap().len(), 1);
    assert_eq!(json["directories"][0]["path"], root.display().to_string());
    assert_eq!(json["directories"][0]["max_depth"], 1);
    assert_eq!(json["directories"][0]["truncated"], true);
    assert_eq!(json["directories"][0]["entries"][0]["kind"], "file");
    assert_eq!(json["directories"][0]["entries"][1]["kind"], "directory");
    assert_eq!(json["directories"][0]["entries"][1]["truncated"], true);
    assert!(json["directories"][0]["entries"][1]["entries"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn rejects_forced_language_for_directory_inputs() {
    let temporary = tempdir().unwrap();
    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .args(["--language", "rust"])
        .arg(temporary.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--language cannot be combined with directory inputs",
        ));
}

#[test]
fn preserves_mixed_root_order_and_does_not_deduplicate_overlapping_inputs() {
    let temporary = tempdir().unwrap();
    let root = temporary.path().join("project");
    fs::create_dir(&root).unwrap();
    let rust = root.join("lib.rs");
    fs::write(&rust, "pub fn repeated() {}\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    let output = command
        .args(["--min-lines", "0"])
        .arg(&rust)
        .arg(&root)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();

    assert!(output.starts_with("- fn repeated [L1]\n\n- `"));
    assert_eq!(output.matches("fn repeated").count(), 2);
}

#[test]
fn reports_an_empty_supported_tree_explicitly() {
    let temporary = tempdir().unwrap();
    fs::write(temporary.path().join("notes.txt"), "unsupported\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .arg(temporary.path())
        .assert()
        .success()
        .stdout(format!(
            "- `{}/`\n  - *(no supported files)*\n",
            temporary.path().display()
        ));
}

#[test]
fn escapes_file_labels_and_verbatim_fences_in_the_integrated_tree() {
    let temporary = tempdir().unwrap();
    let markdown = temporary.path().join("odd`name.md");
    fs::write(&markdown, "```text\nhello\n```\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .arg(temporary.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("- `` odd`name.md ``\n"))
        .stdout(predicate::str::contains(
            "    ````markdown\n    ```text\n    hello\n    ```\n    ````\n",
        ));
}

#[test]
fn continues_after_a_discovered_file_fails_and_exits_nonzero() {
    let temporary = tempdir().unwrap();
    let bad = temporary.path().join("bad.yaml");
    fs::write(&bad, "broken: [\n").unwrap();
    fs::write(temporary.path().join("good.rs"), "pub fn good() {}\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .args(["--min-lines", "0"])
        .arg(temporary.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains("- fn good [L1]"))
        .stderr(predicate::str::contains(bad.display().to_string()));
}

#[cfg(unix)]
#[test]
fn skips_symlinked_files_and_directories() {
    use std::os::unix::fs::symlink;

    let temporary = tempdir().unwrap();
    let root = temporary.path().join("project");
    let outside = temporary.path().join("outside");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("outside.rs"), "pub fn outside() {}\n").unwrap();
    symlink(outside.join("outside.rs"), root.join("linked.rs")).unwrap();
    symlink(&outside, root.join("linked-directory")).unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .args(["--min-lines", "0"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("linked").not())
        .stdout(predicate::str::contains("fn outside").not());
}
