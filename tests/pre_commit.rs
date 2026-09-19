use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(command: &str, root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_config-tools"))
        .args([command, "--config-root"])
        .arg(root)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run config-tools")
}

#[test]
fn pre_commit_checks_repository_without_requiring_home_installation() {
    let fixture = tempfile::tempdir().expect("fixture checkout");
    let root = fixture.path();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join(".agents");
    for entry in walkdir::WalkDir::new(&source) {
        let entry = entry.expect("walk skills");
        let destination = root
            .join(".agents")
            .join(entry.path().strip_prefix(&source).unwrap());
        if entry.file_type().is_dir() {
            fs::create_dir_all(destination).expect("create skill directory");
        } else if entry.file_type().is_file() {
            fs::copy(entry.path(), destination).expect("copy skill file");
        }
    }
    // A separate real crate avoids recursively running this integration test.
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"hook-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("fixture manifest");
    fs::create_dir(root.join("src")).expect("fixture src");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn value() -> u8 {\n    1\n}\n",
    )
    .expect("fixture source");

    let result = run("check-install", root);
    assert!(!result.status.success(), "fixture must not be installed");

    let result = run("pre-commit", root);
    assert!(
        result.status.success(),
        "uninstalled checkout should pass repository checks: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let skill = root.join(".agents/skills/surgical-edit/SKILL.md");
    let valid_skill = fs::read(&skill).expect("valid skill");
    fs::write(&skill, "invalid skill\n").expect("break skill");
    let result = run("pre-commit", root);
    assert!(
        !result.status.success(),
        "invalid skills must block commits"
    );
    assert!(String::from_utf8_lossy(&result.stderr).contains("Config validation failed"));
    fs::write(&skill, valid_skill).expect("restore skill");

    fs::write(
        root.join("src/lib.rs"),
        "#[test]\nfn regression() {\n    assert_eq!(1, 2);\n}\n",
    )
    .expect("failing fixture test");
    let result = run("pre-commit", root);
    assert!(!result.status.success(), "failing tests must block commits");
    assert!(String::from_utf8_lossy(&result.stderr).contains("cargo test failed"));
}
