use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cs", disable_help_subcommand = true)]
#[command(about = "Context-Aware Workspace Automation")]
pub struct Cli {
    #[arg(long, global = true)]
    pub notify: bool,
    #[arg(long, global = true)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Add {
        #[arg(short, long)]
        parallel: bool,
        #[arg(short = 'd', long)]
        desc: Option<String>,
        #[arg(long)]
        timeout: Option<u64>,
        #[arg(short = 'g', long)]
        global: bool,
        alias: String,
        #[arg(required = true, num_args = 1..)]
        commands: Vec<String>,
    },
    Remove {
        #[arg(short = 'g', long)]
        global: bool,
        alias: String,
    },
    Rename {
        old_alias: String,
        new_alias: String,
    },
    Edit {
        #[arg(short = 'g', long)]
        global: bool,
        alias: String,
    },
    Run {
        #[arg(short, long)]
        parallel: bool,
        #[arg(long)]
        timeout: Option<u64>,
        #[arg(required = true, num_args = 1..)]
        commands: Vec<String>,
    },
    // Interactive mode
    Tui,
    List,
    #[command(external_subcommand)]
    External(Vec<String>),
}
