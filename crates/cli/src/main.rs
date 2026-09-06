mod config;
mod ctl;
mod update;

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
        Commands::Restart { args, all } => ctl::restart(&args, all),
        Commands::Stop { all } => ctl::stop(all),
        Commands::Uninstall => Ok(ctl::uninstall()),
        Commands::Update => update::run(),
        Commands::Status => ctl::status(),
        Commands::Daemon(args) => {
            ctl::daemon(args)?;
            Ok(0)
        }
        Commands::McpAsk => {
            ggok_agent::run_mcp_ask()?;
            Ok(0)
        }
    }
}
