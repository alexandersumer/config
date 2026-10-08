mod cli;
mod commands;
mod config_root;
mod error;
mod install;
mod links;
mod registry;
mod regression;
mod reset;
fn main() -> std::process::ExitCode {
    let mut args = std::env::args_os();
    let executable = args.next().unwrap_or_default();
    let args: Vec<_> = args.collect();
    if std::path::Path::new(&executable)
        .file_name()
        .is_some_and(|n| n == "reset_to_origin")
    {
        return std::process::ExitCode::from(reset::run(args));
    }
    if args.first().is_some_and(|a| a == "reset-to-origin") {
        return std::process::ExitCode::from(reset::run(args.into_iter().skip(1).collect()));
    }
    match cli::run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err}");
            std::process::ExitCode::FAILURE
        }
    }
}
