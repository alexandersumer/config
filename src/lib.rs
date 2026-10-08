mod cli;
mod commands;
mod config_root;
mod error;
mod git_domain;
mod install;
mod links;
mod presentation;
mod registry;
mod regression;
mod runtime;
mod workctl;
pub fn run() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    // This is the private reset worker protocol, never a public alias.
    if args.first().is_some_and(|a| a == "--internal-reset-worker") {
        return std::process::ExitCode::from(git_domain::worker(&args[1..]));
    }
    if std::env::args_os().next().is_some_and(|p| {
        std::path::Path::new(&p)
            .file_name()
            .is_some_and(|n| n == "config-tools")
    }) {
        match cli::run() {
            Ok(code) => code,
            Err(err) => {
                eprintln!("{err}");
                std::process::ExitCode::FAILURE
            }
        }
    } else {
        std::process::ExitCode::from(workctl::run(args))
    }
}
