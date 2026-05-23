use std::path::Path;

use wot::model::Language;

#[test]
fn detects_structured_file_extensions_and_special_names() {
    assert_eq!(
        Language::from_path(Path::new("lib.rs")),
        Some(Language::Rust)
    );
    assert_eq!(
        Language::from_path(Path::new("component.tsx")),
        Some(Language::TypeScript)
    );
    assert_eq!(
        Language::from_path(Path::new("app.js")),
        Some(Language::JavaScript)
    );
    assert_eq!(
        Language::from_path(Path::new("main.go")),
        Some(Language::Go)
    );
    assert_eq!(Language::from_path(Path::new("main.c")), Some(Language::C));
    assert_eq!(
        Language::from_path(Path::new("header.h")),
        Some(Language::C)
    );
    assert_eq!(
        Language::from_path(Path::new("app.cpp")),
        Some(Language::Cpp)
    );
    assert_eq!(
        Language::from_path(Path::new("app.cc")),
        Some(Language::Cpp)
    );
    assert_eq!(
        Language::from_path(Path::new("app.cxx")),
        Some(Language::Cpp)
    );
    assert_eq!(
        Language::from_path(Path::new("app.hpp")),
        Some(Language::Cpp)
    );
    assert_eq!(
        Language::from_path(Path::new("App.java")),
        Some(Language::Java)
    );
    assert_eq!(
        Language::from_path(Path::new("App.kt")),
        Some(Language::Kotlin)
    );
    assert_eq!(
        Language::from_path(Path::new("App.cs")),
        Some(Language::CSharp)
    );
    assert_eq!(
        Language::from_path(Path::new("script.sh")),
        Some(Language::Shell)
    );
    assert_eq!(
        Language::from_path(Path::new("core.clj")),
        Some(Language::Clojure)
    );
    assert_eq!(
        Language::from_path(Path::new("demo.el")),
        Some(Language::Elisp)
    );
    assert_eq!(
        Language::from_path(Path::new("notes.org")),
        Some(Language::Org)
    );
    assert_eq!(
        Language::from_path(Path::new("config.yaml")),
        Some(Language::Yaml)
    );
    assert_eq!(
        Language::from_path(Path::new("config.yml")),
        Some(Language::Yaml)
    );
    assert_eq!(
        Language::from_path(Path::new("Cargo.toml")),
        Some(Language::Toml)
    );
    assert_eq!(
        Language::from_path(Path::new("settings.ini")),
        Some(Language::Ini)
    );
    assert_eq!(
        Language::from_path(Path::new(".env")),
        Some(Language::Dotenv)
    );
    assert_eq!(
        Language::from_path(Path::new(".env.local")),
        Some(Language::Dotenv)
    );
    assert_eq!(
        Language::from_path(Path::new("layout.xml")),
        Some(Language::Xml)
    );
    assert_eq!(
        Language::from_path(Path::new("icon.svg")),
        Some(Language::Xml)
    );
    assert_eq!(
        Language::from_path(Path::new("main.tf")),
        Some(Language::Hcl)
    );
    assert_eq!(
        Language::from_path(Path::new("vars.tfvars")),
        Some(Language::Hcl)
    );
    assert_eq!(
        Language::from_path(Path::new("Dockerfile")),
        Some(Language::Dockerfile)
    );
    assert_eq!(
        Language::from_path(Path::new("build.dockerfile")),
        Some(Language::Dockerfile)
    );
    assert_eq!(
        Language::from_path(Path::new("analysis.ipynb")),
        Some(Language::Notebook)
    );
}

#[test]
fn keeps_deferred_tabular_and_streaming_data_unsupported() {
    assert_eq!(Language::from_path(Path::new("data.csv")), None);
    assert_eq!(Language::from_path(Path::new("data.tsv")), None);
    assert_eq!(Language::from_path(Path::new("events.jsonl")), None);
    assert_eq!(Language::from_path(Path::new("events.ndjson")), None);
}

#[test]
fn detects_org_forced_language_aliases() {
    assert_eq!(Language::from_name("org"), Some(Language::Org));
    assert_eq!(Language::from_name("org-mode"), Some(Language::Org));
}
