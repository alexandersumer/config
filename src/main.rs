mod cli;
mod commands;
mod config_root;
mod error;
mod install;
mod links;
mod registry;
mod regression;
mod repair;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
