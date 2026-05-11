mod cli;
mod commands;
mod error;
mod install;
mod links;
mod registry;
mod regression;
mod repair;
mod repo;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
