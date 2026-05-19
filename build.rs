use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const SKILL_NAME: &str = "create-file-outlne";

fn main() {
    println!("cargo:rerun-if-changed=skills/{SKILL_NAME}/SKILL.md");

    if env::var("PROFILE").as_deref() != Ok("release") {
        return;
    }

    if env::var("WOT_SKIP_SKILL_INSTALL").as_deref() == Ok("1") {
        return;
    }

    if let Err(error) = install_skill() {
        panic!("failed to install {SKILL_NAME} skill: {error}");
    }
}

fn install_skill() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let source = manifest_dir
        .join("skills")
        .join(SKILL_NAME)
        .join("SKILL.md");
    let destination = skill_destination()?;

    fs::create_dir_all(
        destination
            .parent()
            .expect("skill destination has a parent"),
    )?;
    fs::copy(&source, &destination)?;

    println!(
        "cargo:warning=installed {SKILL_NAME} skill to {}",
        destination.display()
    );

    Ok(())
}

fn skill_destination() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = env::var("HOME")?;
    Ok(Path::new(&home)
        .join(".agents")
        .join("skills")
        .join(SKILL_NAME)
        .join("SKILL.md"))
}
