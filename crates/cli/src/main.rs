mod config;
mod ctl;

use anyhow::Result;
use clap::Parser;
use config::{Cli, Commands};

fn main() {
    let code = match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e:#}");
            1
        }
    };
    if code != 0 {
        std::process::exit(code);
    }
}

fn run() -> Result<i32> {
    match Cli::parse().command {
        Commands::Start(args) => {
            ctl::start(&args)?;
            Ok(0)
        }
        Commands::Restart(args) => {
            ctl::restart(&args)?;
            Ok(0)
        }
        Commands::Stop => {
            ctl::stop()?;
            Ok(0)
        }
        Commands::Uninstall => ctl::uninstall(),
        Commands::Status => ctl::status(),
        Commands::Daemon(args) => {
            ctl::daemon(args)?;
            Ok(0)
        }
    }
}
