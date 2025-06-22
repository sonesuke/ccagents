use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to config YAML file
    #[arg(short, long, global = true)]
    pub rules: Option<PathBuf>,

    /// Enable debug logging for internal details
    #[arg(short, long, global = true)]
    pub debug: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Load and display rules
    Show(ShowArgs),
    /// Test rule matching against capture text
    Test(TestArgs),
}

#[derive(Args, Debug)]
pub struct ShowArgs {
    /// Path to config YAML file
    #[arg(short, long, default_value = "config.yaml")]
    pub rules: PathBuf,
}

#[derive(Args, Debug)]
pub struct TestArgs {
    /// Path to config YAML file
    #[arg(short, long, default_value = "config.yaml")]
    pub rules: PathBuf,
    /// Capture text to test against rules
    #[arg(short, long)]
    pub capture: String,
}
