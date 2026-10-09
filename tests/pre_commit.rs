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

fn git(root: &Path, args: &[&str]) {
    let result = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run fixture git");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn pre_commit_validates_index_and_preserves_local_changes() {
    let fixture = tempfile::tempdir().expect("fixture checkout");
    let root = fixture.path();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    if source.join(".git").exists() {
        git(
            source,
            &[
                "checkout-index",
                "--all",
                &format!("--prefix={}/", root.display()),
            ],
        );
        fs::remove_file(root.join("tests/pre_commit.rs"))
            .expect("exclude recursive integration tests");
    } else {
        for entry in walkdir::WalkDir::new(source)
            .into_iter()
            .filter_entry(|entry| {
                entry.depth() == 0
                    || (!matches!(entry.file_name().to_str(), Some("target" | ".git"))
                        && entry.path() != source.join("tests/pre_commit.rs"))
            })
        {
            let entry = entry.expect("walk staged source");
            let destination = root.join(entry.path().strip_prefix(source).unwrap());
            if entry.file_type().is_dir() {
                fs::create_dir_all(destination).expect("create fixture directory");
            } else if entry.file_type().is_file() {
                fs::copy(entry.path(), destination).expect("copy fixture file");
            }
        }
    }
    git(root, &["init", "--quiet"]);
    git(root, &["add", "."]);
    let source_file = root.join("src/main.rs");
    fs::write(&source_file, "invalid unstaged Rust\n").unwrap();
    let unrelated = root.join(".agents/skills/unrelated-local-skill");
    fs::create_dir_all(&unrelated).unwrap();
    fs::write(unrelated.join("SKILL.md"), "untracked work\n").unwrap();
    let result = run("pre-commit", root);
    assert!(
        result.status.success(),
        "staged checks should ignore local changes: {}{}",
        String::from_utf8_lossy(&result.stderr),
        String::from_utf8_lossy(&result.stdout)
    );
    assert_eq!(
        fs::read_to_string(&source_file).unwrap(),
        "invalid unstaged Rust\n"
    );
    assert_eq!(
        fs::read_to_string(unrelated.join("SKILL.md")).unwrap(),
        "untracked work\n"
    );

    let skill = root.join(".agents/skills/surgical-edit/SKILL.md");
    let valid = fs::read(&skill).unwrap();
    fs::write(&skill, "invalid staged skill\n").unwrap();
    git(root, &["add", ".agents/skills/surgical-edit/SKILL.md"]);
    fs::write(&skill, &valid).unwrap();
    let result = run("pre-commit", root);
    assert!(
        !result.status.success(),
        "invalid staged skill must block commit even with valid working copy"
    );
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("Config validation failed"),
        "{}{}",
        String::from_utf8_lossy(&result.stderr),
        String::from_utf8_lossy(&result.stdout)
    );
    assert_eq!(fs::read(&skill).unwrap(), valid);
}
