use clap::{Args, Parser, Subcommand};
use ggok_core::config::ConfigOverrides;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "ggok",
    version,
    about = "webui for grok build cli",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Fork into the background and print the login token")]
    Start(StartArgs),
    #[command(about = "Stop if running, then start")]
    Restart {
        #[command(flatten)]
        args: StartArgs,
        #[arg(long)]
        all: bool,
    },
    #[command(about = "Show pid, listen address, leader, and session occupancy")]
    Status,
    #[command(about = "Stop the web daemon; --all also stops an idle leader this process started")]
    Stop {
        #[arg(long)]
        all: bool,
    },
    #[command(about = "Stop ggok and delete its binary, config, logs, and cache")]
    Uninstall,
    #[command(about = "Download the latest release and replace this binary")]
    Update,
    #[command(name = "__daemon", hide = true)]
    Daemon(StartArgs),
}

#[derive(Debug, Clone, Args)]
pub struct StartArgs {
    #[arg(long, env = "GGOK_BIND")]
    pub bind: Option<String>,
    #[arg(long)]
    pub token_file: Option<PathBuf>,
    #[arg(long, env = "GROK_HOME")]
    pub grok_home: Option<PathBuf>,
    #[arg(long, env = "GGOK_GROK_BIN")]
    pub grok_bin: Option<String>,
    #[arg(long)]
    pub poll_secs: Option<u64>,
    #[arg(long, env = "GGOK_PERMISSION_MODE")]
    pub permission_mode: Option<String>,
    #[arg(long)]
    pub upload_max_bytes: Option<u64>,
    #[arg(long, env = "GGOK_CONFIG")]
    pub config: Option<PathBuf>,
}

impl StartArgs {
    #[must_use]
    pub fn into_overrides(self) -> ConfigOverrides {
        ConfigOverrides {
            bind: self.bind,
            token_file: self.token_file,
            grok_home: self.grok_home,
            grok_bin: self.grok_bin,
            poll_secs: self.poll_secs,
            permission_mode: self.permission_mode,
            upload_max_bytes: self.upload_max_bytes,
            config: self.config,
        }
    }
}
