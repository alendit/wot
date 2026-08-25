use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn markdown_headers_are_opt_in_and_min_lines_enables_verbatim_output() {
    let directory = tempdir().unwrap();
    let markdown = directory.path().join("doc.md");
    fs::write(&markdown, "# Intro\nbody\n").unwrap();

    let mut default_command = Command::cargo_bin("wot").unwrap();
    default_command.arg(&markdown);
    default_command
        .assert()
        .success()
        .stdout("# Intro\nbody\n")
        .stdout(predicate::str::contains(format!("# {}", markdown.display())).not());

    let mut outline_command = Command::cargo_bin("wot").unwrap();
    outline_command.args(["--min-lines", "0"]).arg(&markdown);
    outline_command
        .assert()
        .success()
        .stdout("- Intro [L1-L2]\n");

    let mut verbatim_command = Command::cargo_bin("wot").unwrap();
    verbatim_command
        .args(["--header", "--min-lines", "40"])
        .arg(&markdown);
    verbatim_command
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "# {}",
            markdown.display()
        )))
        .stdout(predicate::str::contains("# Intro\nbody\n"));
}

#[test]
fn renders_json_outline_with_nested_nodes_and_ranges() {
    let directory = tempdir().unwrap();
    let python = directory.path().join("sample.py");
    fs::write(&python, "class App:\n    def run(self):\n        pass\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    let output = command
        .args(["--format", "json", "--min-lines", "0"])
        .arg(&python)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(json["errors"].as_array().unwrap().len(), 0);
    assert_eq!(json["files"][0]["path"], python.display().to_string());
    assert_eq!(json["files"][0]["language"], "python");
    assert_eq!(json["files"][0]["mode"], "outline");
    assert_eq!(json["files"][0]["truncated"], false);
    assert_eq!(json["files"][0]["omitted_nodes"], 0);
    assert_eq!(json["files"][0]["nodes"][0]["label"], "class App");
    assert_eq!(json["files"][0]["nodes"][0]["kind"], "class");
    assert_eq!(json["files"][0]["nodes"][0]["range"]["display"], "L1-L3");
    assert_eq!(
        json["files"][0]["nodes"][0]["children"][0]["label"],
        "def run"
    );
}

#[test]
fn renders_valid_json_when_one_file_fails() {
    let directory = tempdir().unwrap();
    let missing = directory.path().join("missing.md");
    let rust = directory.path().join("lib.rs");
    fs::write(&rust, "pub fn run() {}\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    let output = command
        .args(["--format", "json", "--min-lines", "0"])
        .args([missing.as_os_str(), rust.as_os_str()])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(json["files"].as_array().unwrap().len(), 1);
    assert_eq!(json["files"][0]["language"], "rust");
    assert_eq!(json["errors"].as_array().unwrap().len(), 1);
    assert!(json["errors"][0]["message"]
        .as_str()
        .unwrap()
        .contains("missing.md"));
}

#[test]
fn lists_supported_languages_as_markdown_and_json() {
    let mut markdown_command = Command::cargo_bin("wot").unwrap();
    markdown_command
        .arg("--list-supported")
        .assert()
        .success()
        .stdout(predicate::str::contains("rust"))
        .stdout(predicate::str::contains(".rs"))
        .stdout(predicate::str::contains("org"))
        .stdout(predicate::str::contains(".org"))
        .stdout(predicate::str::contains("tree-sitter"));

    let mut json_command = Command::cargo_bin("wot").unwrap();
    let output = json_command
        .args(["--list-supported", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();

    assert!(json["languages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|language| language["id"] == "rust"
            && language["extensions"]
                .as_array()
                .unwrap()
                .contains(&Value::from(".rs"))));
    assert!(json["languages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|language| language["id"] == "org"
            && language["names"]
                .as_array()
                .unwrap()
                .contains(&Value::from("org-mode"))
            && language["extensions"]
                .as_array()
                .unwrap()
                .contains(&Value::from(".org"))));
}

#[test]
fn max_items_truncates_preorder_outline_and_reports_omitted_nodes() {
    let directory = tempdir().unwrap();
    let markdown = directory.path().join("doc.md");
    fs::write(&markdown, "# A\n## B\n# C\n").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    let output = command
        .args(["--format", "json", "--min-lines", "0", "--max-items", "1"])
        .arg(&markdown)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(json["files"][0]["truncated"], true);
    assert_eq!(json["files"][0]["omitted_nodes"], 2);
    assert_eq!(json["files"][0]["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(json["files"][0]["nodes"][0]["label"], "A");
    assert!(json["files"][0]["nodes"][0]["children"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn language_forces_extensionless_files_and_stdin_requires_language() {
    let directory = tempdir().unwrap();
    let extensionless = directory.path().join("BUILD");
    fs::write(&extensionless, "def run():\n    return 1\n").unwrap();

    let mut forced_file_command = Command::cargo_bin("wot").unwrap();
    forced_file_command
        .args(["--language", "python", "--min-lines", "0"])
        .arg(&extensionless)
        .assert()
        .success()
        .stdout(predicate::str::contains("- def run [L1-L2]"));

    let mut stdin_without_language = Command::cargo_bin("wot").unwrap();
    stdin_without_language
        .arg("--stdin")
        .write_stdin("def run():\n    return 1\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--stdin requires --language"));

    let mut stdin_with_language = Command::cargo_bin("wot").unwrap();
    stdin_with_language
        .args(["--stdin", "--language", "python", "--min-lines", "0"])
        .write_stdin("def run():\n    return 1\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("- def run [L1-L2]"));

    let mut org_stdin_with_language = Command::cargo_bin("wot").unwrap();
    org_stdin_with_language
        .args(["--stdin", "--language", "org", "--min-lines", "0"])
        .write_stdin("* Root\n** Child\n")
        .assert()
        .success()
        .stdout("- Root [L1-L2]\n  - Child [L2]\n");

    let mut stdin_with_file = Command::cargo_bin("wot").unwrap();
    stdin_with_file
        .args(["--stdin", "--language", "python"])
        .arg(&extensionless)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--stdin cannot be combined with file paths",
        ));
}

#[test]
fn lenient_mode_returns_partial_structured_outlines() {
    let directory = tempdir().unwrap();
    let yaml = directory.path().join("bad.yaml");
    let hcl = directory.path().join("bad.tf");
    let xml = directory.path().join("bad.xml");
    fs::write(&yaml, "service:\n  image: [\n").unwrap();
    fs::write(&hcl, "resource \"x\" \"y\" {\n  name = \"demo\"\n").unwrap();
    fs::write(&xml, "<root>\n  <child />\n").unwrap();

    let mut strict_command = Command::cargo_bin("wot").unwrap();
    strict_command
        .args(["--min-lines", "0"])
        .arg(&yaml)
        .assert()
        .failure();

    let mut lenient_command = Command::cargo_bin("wot").unwrap();
    lenient_command
        .args(["--lenient", "--min-lines", "0"])
        .args([yaml.as_os_str(), hcl.as_os_str(), xml.as_os_str()])
        .assert()
        .success()
        .stdout(predicate::str::contains("- service: object"))
        .stdout(predicate::str::contains("- resource \"x\" \"y\""))
        .stdout(predicate::str::contains("- root"));
}

#[test]
fn setup_installs_project_agent_skill_by_default() {
    let directory = tempdir().unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command.arg("setup").current_dir(directory.path());
    command
        .assert()
        .success()
        .stdout(predicate::str::contains(".agents"))
        .stdout(predicate::str::contains("create-file-outline"));

    let skill = directory
        .path()
        .join(".agents/skills/create-file-outline/SKILL.md");
    let content = fs::read_to_string(skill).unwrap();
    assert!(content.contains("name: create-file-outline"));
}

#[test]
fn setup_global_and_claude_install_to_home_roots() {
    let directory = tempdir().unwrap();
    let home = tempdir().unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .args(["setup", "-g", "--claude"])
        .current_dir(directory.path())
        .env("HOME", home.path());
    command
        .assert()
        .success()
        .stdout(predicate::str::contains(".agents"))
        .stdout(predicate::str::contains(".claude"));

    let agents_skill = home
        .path()
        .join(".agents/skills/create-file-outline/SKILL.md");
    let claude_skill = home
        .path()
        .join(".claude/skills/create-file-outline/SKILL.md");
    assert!(fs::read_to_string(agents_skill)
        .unwrap()
        .contains("name: create-file-outline"));
    assert!(fs::read_to_string(claude_skill)
        .unwrap()
        .contains("name: create-file-outline"));
}

#[test]
fn setup_global_hooks_install_to_home_hook_roots() {
    let directory = tempdir().unwrap();
    let home = tempdir().unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .args(["setup", "-g", "--claude", "--hooks"])
        .current_dir(directory.path())
        .env("HOME", home.path());
    command.assert().success();

    let codex_hooks = home.path().join(".codex/hooks.json");
    let claude_settings = home.path().join(".claude/settings.json");
    assert!(fs::read_to_string(codex_hooks)
        .unwrap()
        .contains("wot hook-check"));
    assert!(fs::read_to_string(claude_settings)
        .unwrap()
        .contains("wot hook-check"));
}

#[test]
fn setup_hooks_installs_codex_hook_and_agent_skill() {
    let directory = tempdir().unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .args(["setup", "--hooks"])
        .current_dir(directory.path());
    command
        .assert()
        .success()
        .stdout(predicate::str::contains(".agents"))
        .stdout(predicate::str::contains(".codex/hooks.json"));

    let skill = directory
        .path()
        .join(".agents/skills/create-file-outline/SKILL.md");
    assert!(fs::read_to_string(skill)
        .unwrap()
        .contains("name: create-file-outline"));

    let hooks_path = directory.path().join(".codex/hooks.json");
    let hooks: Value = serde_json::from_str(&fs::read_to_string(hooks_path).unwrap()).unwrap();
    let pre_tool = hooks["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(pre_tool.len(), 1);
    assert_eq!(pre_tool[0]["matcher"], "Bash");
    assert_eq!(pre_tool[0]["hooks"][0]["command"], "wot hook-check");
}

#[test]
fn setup_claude_hooks_installs_claude_settings_and_is_idempotent() {
    let directory = tempdir().unwrap();
    let settings_path = directory.path().join(".claude/settings.json");
    fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    fs::write(
        &settings_path,
        r#"{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"echo unrelated"}]}]},"permissions":{"allow":["Bash(cargo test)"]}}"#,
    )
    .unwrap();

    for _ in 0..2 {
        let mut command = Command::cargo_bin("wot").unwrap();
        command
            .args(["setup", "--claude", "--hooks"])
            .current_dir(directory.path());
        command.assert().success();
    }

    let settings: Value =
        serde_json::from_str(&fs::read_to_string(settings_path).unwrap()).unwrap();
    assert_eq!(settings["permissions"]["allow"][0], "Bash(cargo test)");

    let pre_tool = settings["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(pre_tool.len(), 2);
    assert_eq!(pre_tool[0]["hooks"][0]["command"], "echo unrelated");
    assert_eq!(pre_tool[1]["matcher"], "Bash|Read");
    assert_eq!(pre_tool[1]["hooks"][0]["command"], "wot hook-check");
}

#[test]
fn setup_hooks_preserves_unrelated_codex_hooks_and_replaces_prior_wot_hook() {
    let directory = tempdir().unwrap();
    let hooks_path = directory.path().join(".codex/hooks.json");
    fs::create_dir_all(hooks_path.parent().unwrap()).unwrap();
    fs::write(
        &hooks_path,
        r#"{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"echo unrelated"}]},{"matcher":"Bash","hooks":[{"type":"command","command":"/old/bin/wot hook-check"}]}]}}"#,
    )
    .unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .args(["setup", "--hooks"])
        .current_dir(directory.path());
    command.assert().success();

    let hooks: Value = serde_json::from_str(&fs::read_to_string(hooks_path).unwrap()).unwrap();
    let pre_tool = hooks["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(pre_tool.len(), 2);
    assert_eq!(pre_tool[0]["hooks"][0]["command"], "echo unrelated");
    assert_eq!(pre_tool[1]["hooks"][0]["command"], "wot hook-check");
}

#[test]
fn setup_hooks_rejects_invalid_existing_json() {
    let directory = tempdir().unwrap();
    let hooks_path = directory.path().join(".codex/hooks.json");
    fs::create_dir_all(hooks_path.parent().unwrap()).unwrap();
    fs::write(&hooks_path, "{not json").unwrap();

    let mut command = Command::cargo_bin("wot").unwrap();
    command
        .args(["setup", "--hooks"])
        .current_dir(directory.path());
    command
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid JSON"));
}

#[test]
fn hook_check_rewrites_broad_reads_before_execution() {
    let mut command = Command::cargo_bin("wot").unwrap();
    let output = command
        .arg("hook-check")
        .write_stdin(r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"cat src/cli.rs"}}"#)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(json["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    assert_eq!(json["hookSpecificOutput"]["permissionDecision"], "allow");
    assert_eq!(
        json["hookSpecificOutput"]["updatedInput"]["command"],
        "wot --header src/cli.rs"
    );
}

#[test]
fn hook_check_rewrites_compound_commands() {
    let mut command = Command::cargo_bin("wot").unwrap();
    let output = command
        .arg("hook-check")
        .write_stdin(
            r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git status --short && cat src/lib.rs"}}"#,
        )
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(
        json["hookSpecificOutput"]["updatedInput"]["command"],
        "git status --short && wot --header src/lib.rs"
    );
}

#[test]
fn hook_check_keeps_full_read_advisory_for_claude() {
    let mut command = Command::cargo_bin("wot").unwrap();
    let output = command
        .arg("hook-check")
        .write_stdin(
            r#"{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"src/cli.rs"}}"#,
        )
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(
        json["hookSpecificOutput"]["additionalContext"],
        "Use wot for a file overview."
    );
}

#[test]
fn hook_check_exits_silently_for_exact_or_unrelated_inputs() {
    for input in [
        r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rg \"hook-check\" src/cli.rs"}}"#,
        r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rg --files"}}"#,
        r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"wot README.md"}}"#,
        r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"sed -n '20,60p' src/cli.rs"}}"#,
        r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"sed -n '1,240p' src/cli.rs"}}"#,
        r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"cat src/cli.rs | sha256sum"}}"#,
        r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"cat notes.txt"}}"#,
        r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"cat AGENTS.md"}}"#,
        r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"cat skills/create-file-outline/SKILL.md"}}"#,
        r#"{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"src/cli.rs","offset":10,"limit":20}}"#,
        r#"{not json"#,
    ] {
        let mut command = Command::cargo_bin("wot").unwrap();
        command
            .arg("hook-check")
            .write_stdin(input)
            .assert()
            .success()
            .stdout("");
    }
}
