mod cli;
mod commands;
mod config_root;
mod error;
mod install;
mod links;
mod registry;
mod regression;
fn main() -> std::process::ExitCode {
    match cli::run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err}");
            std::process::ExitCode::FAILURE
        }
    }
}
