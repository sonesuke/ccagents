use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to config YAML file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Enable debug logging for internal details
    #[arg(short, long)]
    pub debug: bool,
}
