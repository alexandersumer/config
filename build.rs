use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
fn main() {
    // Exported source snapshots intentionally report unknown provenance.
    // These are build metadata only, never proof that the installed binary
    // contains every change in a later checkout.
    let revision = git(&["rev-parse", "HEAD"])
        .filter(|s| matches!(s.len(), 40 | 64) && s.bytes().all(|b| b.is_ascii_hexdigit()));
    println!(
        "cargo:rustc-env=WORKCTL_BUILD_REVISION={}",
        revision.as_deref().unwrap_or("unknown")
    );
    for name in [
        Some("HEAD".to_owned()),
        git(&["symbolic-ref", "--quiet", "HEAD"]),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(path) = git(&["rev-parse", "--git-path", &name]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}
