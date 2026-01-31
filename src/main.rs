use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Instant;

// stash aliases here - current folder
const CONFIG_FILE: &str = ".cawa_cfg.json";

#[derive(Parser)]
#[command(name = "cs", disable_help_subcommand = true)]
#[command(about = "Context-Aware Workspace Automation")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    // save new alias - runs parallel if -p used
    Add {
        #[arg(short, long)]
        parallel: bool,
        alias: String,
        #[arg(required = true, num_args = 1..)]
        commands: Vec<String>,
    },
    // nuke valid alias
    Remove {
        alias: String,
    },
    // show what we got
    List,

    // catch-all - runs aliases e.g. `cs foo`
    #[command(external_subcommand)]
    External(Vec<String>),
}

// support single cmd or parallel batch
#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum AliasEntry {
    Single(String),
    Parallel(Vec<String>),
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
    // optional project id - maybe handy later
    #[serde(skip_serializing_if = "Option::is_none")]
    identifier: Option<String>,
    // flip to true in json to see runtime speeds
    #[serde(default)]
    enable_timing: Option<bool>,
    // the meat - alias map
    #[serde(default)]
    aliases: HashMap<String, AliasEntry>,
}

// load config from disk, or default if missing
fn load_config() -> Result<Config> {
    if !Path::new(CONFIG_FILE).exists() {
        return Ok(Config::default());
    }
    let content = fs::read_to_string(CONFIG_FILE)?;
    // error if json is busted
    serde_json::from_str(&content).context("Failed to parse config file")
}

// dump config to disk - pretty print for humans
fn save_config(config: &Config) -> Result<()> {
    let content = serde_json::to_string_pretty(config)?;
    fs::write(CONFIG_FILE, content).context("Failed to write config file")
}

// figure out program name - adapts if you rename binary
// chameleon vibes 🦎
fn get_program_name() -> String {
    std::env::args()
        .next()
        .and_then(|s| {
            Path::new(&s)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "cs".to_string())
}

// kick off shell cmd - `sh -c` supports pipes, &&, etc
fn execute_command(cmd_str: &str) -> bool {
    let status = Command::new("sh")
        .arg("-c")
        .arg(cmd_str)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    match status {
        Ok(s) => s.success(),
        Err(_) => false,
    }
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let program_name = get_program_name();

    match args.command {
        Some(Commands::Add {
            parallel,
            alias,
            commands,
        }) => {
            let mut config = load_config()?;

            // store as list if -p
            // otherwise join args into one cmd string
            let entry = if parallel {
                AliasEntry::Parallel(commands.clone())
            } else {
                if commands.len() > 1 {
                    // multiple strings but no -p? assume one long cmd
                    // e.g. `cs add foo "echo a" "&&" "echo b"`
                    AliasEntry::Single(commands.join(" "))
                } else {
                    AliasEntry::Single(commands[0].clone())
                }
            };

            config.aliases.insert(alias.clone(), entry.clone());
            save_config(&config)?;

            // pretty feedback
            let display_val = match entry {
                AliasEntry::Single(s) => s,
                AliasEntry::Parallel(v) => format!("[{}]", v.join(", ")),
            };

            println!(
                "{} {} now stores {}",
                "🐙".truecolor(80, 80, 80),
                program_name.bold(),
                display_val.cyan()
            );
        }
        Some(Commands::Remove { alias }) => {
            let mut config = load_config()?;
            if config.aliases.remove(&alias).is_some() {
                save_config(&config)?;
                println!(
                    "{} {} {} removed.",
                    "🐙".truecolor(80, 80, 80),
                    program_name.bold(),
                    alias.red()
                );
            } else {
                eprintln!("Alias '{}' not found.", alias);
            }
        }
        Some(Commands::List) => {
            let config = load_config()?;
            if config.aliases.is_empty() {
                println!("No aliases found in {}", CONFIG_FILE);
            } else {
                println!("{} Aliases", "🐙".truecolor(80, 80, 80));
                for (alias, entry) in config.aliases {
                    let val = match entry {
                        AliasEntry::Single(s) => s,
                        AliasEntry::Parallel(v) => format!("[{}]", v.join(", ")),
                    };
                    println!(
                        "{} {} → {}",
                        program_name.dimmed(),
                        alias.bold(),
                        val.cyan()
                    );
                }
            }
        }
        // run the alias!
        Some(Commands::External(args)) => {
            if args.is_empty() {
                use clap::CommandFactory;
                Cli::command().print_help()?;
                return Ok(());
            }
            let alias = &args[0];
            let extra_args = &args[1..];

            let config = load_config()?;
            if let Some(entry) = config.aliases.get(alias) {
                let start = Instant::now();

                let success = match entry {
                    AliasEntry::Single(cmd) => {
                        // append runtime args - e.g. `cs run-tests -- --filter=foo`
                        let mut final_cmd = cmd.clone();
                        if !extra_args.is_empty() {
                            final_cmd.push_str(" ");
                            final_cmd.push_str(&extra_args.join(" "));
                        }
                        println!(
                            "{} Executing: {}",
                            "🐙".truecolor(80, 80, 80),
                            final_cmd.cyan()
                        );
                        execute_command(&final_cmd)
                    }
                    AliasEntry::Parallel(cmds) => {
                        println!(
                            "{} Executing (parallel): {:?}",
                            "🐙".truecolor(80, 80, 80),
                            cmds
                        );
                        if !extra_args.is_empty() {
                            println!(
                                "{} Warning: Arguments ignored for parallel alias.",
                                "🐙".truecolor(80, 80, 80)
                            );
                        }

                        // flag if any thread fails
                        let failure_occurred = Arc::new(AtomicBool::new(false));
                        let mut handles = vec![];

                        // thread per cmd
                        for cmd in cmds {
                            let cmd_str = cmd.clone();
                            let fail_flag = failure_occurred.clone();
                            handles.push(thread::spawn(move || {
                                if !execute_command(&cmd_str) {
                                    fail_flag.store(true, Ordering::Relaxed);
                                }
                            }));
                        }

                        // wait for all
                        for h in handles {
                            let _ = h.join();
                        }

                        !failure_occurred.load(Ordering::Relaxed)
                    }
                };

                // print duration if enabled in config
                if config.enable_timing.unwrap_or(false) {
                    let duration = start.elapsed();
                    let duration_s = duration.as_secs_f64();

                    if success {
                        println!("{}⏱️  {:.3} s", "🐙".truecolor(80, 80, 80), duration_s);
                    } else {
                        eprintln!(
                            "{}⏱️  {:.3} s (Failed)",
                            "🐙".truecolor(80, 80, 80),
                            duration_s
                        );
                        std::process::exit(1);
                    }
                } else if !success {
                    // failed but no timing? still exit error
                    std::process::exit(1);
                }
            } else {
                eprintln!("Unknown command or alias: {}", alias);
            }
        }
        None => {
            use clap::CommandFactory;
            Cli::command().print_help()?;
        }
    }

    Ok(())
}
