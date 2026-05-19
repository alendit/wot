const SKILL_NAME: &str = "create-file-outline";

fn main() {
    println!("cargo:rerun-if-changed=skills/{SKILL_NAME}/SKILL.md");
}
