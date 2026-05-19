use std::path::Path;

use wot::model::Language;

#[test]
fn detects_structured_file_extensions_and_special_names() {
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
