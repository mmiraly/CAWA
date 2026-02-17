use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cs", disable_help_subcommand = true)]
#[command(about = "Context-Aware Workspace Automation")]
pub struct Cli {
    #[arg(long, global = true)]
    pub notify: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Add {
        #[arg(short, long)]
        parallel: bool,
        alias: String,
        #[arg(required = true, num_args = 1..)]
        commands: Vec<String>,
    },
    Remove {
        alias: String,
    },
    // Interactive mode
    Tui,
    List,
    #[command(external_subcommand)]
    External(Vec<String>),
}
