use std::path::Path;

use wot::model::{Language, NodeKind, SourceRange};
use wot::parsers::{dotenv, ini, toml, yaml};

#[test]
fn outlines_yaml_mappings_arrays_and_scalar_previews() {
    let source = r#"name: demo
services:
  web:
    image: nginx
    ports:
      - 80
"#;
    let outline = yaml::parse(Path::new("compose.yaml"), source, 4).unwrap();

    assert_eq!(outline.language, Language::Yaml);
    assert_eq!(outline.nodes[0].label, "name: \"demo\"");
    assert_eq!(outline.nodes[0].kind, NodeKind::ConfigKey);
    assert_eq!(outline.nodes[0].range, SourceRange::lines(1, 1));
    assert_eq!(outline.nodes[1].label, "services: object");
    assert_eq!(outline.nodes[1].range, SourceRange::lines(2, 6));
    assert_eq!(outline.nodes[1].children[0].label, "web: object");
    assert_eq!(
        outline.nodes[1].children[0].children[1].label,
        "ports: array[1]"
    );
    assert_eq!(
        outline.nodes[1].children[0].children[1].children[0].label,
        "[0]: 80"
    );
}

#[test]
fn respects_max_depth_for_yaml_outline_depth() {
    let source = "outer:\n  inner:\n    leaf: true\n";
    let outline = yaml::parse(Path::new("config.yml"), source, 2).unwrap();

    assert_eq!(outline.nodes[0].label, "outer: object");
    assert_eq!(outline.nodes[0].children[0].label, "inner: object");
    assert!(outline.nodes[0].children[0].children.is_empty());
}

#[test]
fn reports_invalid_yaml() {
    let error = yaml::parse(Path::new("bad.yaml"), "a: [\n", 3).unwrap_err();

    assert!(error.to_string().contains("bad.yaml"));
}

#[test]
fn outlines_toml_tables_arrays_and_values() {
    let source = r#"name = "wot"

[package]
version = "1.0"

[[bin]]
name = "wot"
"#;
    let outline = toml::parse(Path::new("Cargo.toml"), source, 3).unwrap();

    assert_eq!(outline.language, Language::Toml);
    assert_eq!(outline.nodes[0].label, "name: \"wot\"");
    assert_eq!(outline.nodes[0].range, SourceRange::lines(1, 1));
    assert_eq!(outline.nodes[1].label, "package");
    assert_eq!(outline.nodes[1].kind, NodeKind::ConfigSection);
    assert_eq!(outline.nodes[1].range, SourceRange::lines(3, 4));
    assert_eq!(outline.nodes[1].children[0].label, "version: \"1.0\"");
    assert_eq!(outline.nodes[2].label, "bin[]");
    assert_eq!(outline.nodes[2].range, SourceRange::lines(6, 7));
}

#[test]
fn reports_invalid_toml() {
    let error = toml::parse(Path::new("bad.toml"), "name = \n", 3).unwrap_err();

    assert!(error.to_string().contains("bad.toml"));
}

#[test]
fn outlines_ini_sections_and_keys() {
    let source = "root = yes\n\n[server]\nhost = localhost\nport = 8080\n";
    let outline = ini::parse(Path::new("settings.ini"), source, 3).unwrap();

    assert_eq!(outline.language, Language::Ini);
    assert_eq!(outline.nodes[0].label, "root: \"yes\"");
    assert_eq!(outline.nodes[1].label, "server");
    assert_eq!(outline.nodes[1].kind, NodeKind::ConfigSection);
    assert_eq!(outline.nodes[1].range, SourceRange::lines(3, 5));
    assert_eq!(outline.nodes[1].children[1].label, "port: 8080");
}

#[test]
fn outlines_env_keys_and_redacts_secret_values() {
    let source = "APP_NAME=wot\nAPI_TOKEN=secret-token\nEMPTY=\n";
    let outline = dotenv::parse(Path::new(".env.local"), source, 3).unwrap();

    assert_eq!(outline.language, Language::Dotenv);
    assert_eq!(outline.nodes[0].label, "APP_NAME: \"wot\"");
    assert_eq!(outline.nodes[1].label, "API_TOKEN: <redacted>");
    assert_eq!(outline.nodes[1].kind, NodeKind::ConfigKey);
    assert_eq!(outline.nodes[2].label, "EMPTY: \"\"");
}
