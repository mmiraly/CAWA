mod cli;
mod config;
mod runner;
mod tui;

mod notifications;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use colored::*;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::cli::{Cli, Commands};
use crate::config::{AliasConfig, AliasEntry, load_config, save_config};
use crate::runner::execute_command;

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

fn main() -> Result<()> {
    let args = Cli::parse();
    let program_name = get_program_name();
    let mut success = true;
    let mut should_notify = args.notify;

    let mut executed_alias = None;

    match args.command {
        // ... (Add, Remove, List unchanged)
        Some(Commands::Add {
            parallel,
            desc,
            alias,
            commands,
        }) => {
            let mut config = load_config()?;

            let entry = if parallel {
                AliasEntry::Parallel(commands.clone())
            } else {
                if commands.len() > 1 {
                    AliasEntry::Single(commands.join(" "))
                } else {
                    AliasEntry::Single(commands[0].clone())
                }
            };

            let display_val = match &entry {
                AliasEntry::Single(s) => s.clone(),
                AliasEntry::Parallel(v) => format!("[{}]", v.join(", ")),
            };

            config.aliases.insert(alias.clone(), AliasConfig { entry, description: desc });
            save_config(&config)?;

            println!(
                "{} {} now stores {}",
                "🐙".truecolor(80, 80, 80),
                program_name.bold(),
                display_val.cyan()
            );
        }
        Some(Commands::Remove { alias }) => {
            let mut config = load_config()?;
            if config.aliases.remove(&alias).is_some() { // remove returns the old value if it existed
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
                println!("No aliases found.");
            } else {
                println!("{} Aliases", "🐙".truecolor(80, 80, 80));
                // sort so the output is stable across runs
                let mut sorted: Vec<_> = config.aliases.into_iter().collect();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));
                for (alias, ac) in sorted {
                    match &ac.entry {
                        AliasEntry::Single(s) => {
                            println!(
                                "{} {} → {}",
                                program_name.dimmed(),
                                alias.bold(),
                                s.cyan()
                            );
                        }
                        AliasEntry::Parallel(cmds) => {
                            println!(
                                "{} {} → {}",
                                program_name.dimmed(),
                                alias.bold(),
                                "[parallel]".yellow()
                            );
                            for cmd in cmds {
                                println!("    {} {}", "└".dimmed(), cmd.cyan());
                            }
                        }
                    }
                    if let Some(desc) = &ac.description {
                        println!("    {} {}", "ℹ".dimmed(), desc.dimmed());
                    }
                }
            }
        }
        Some(Commands::Tui) => {
            let config = load_config()?;
            if let Some(selected_alias) = tui::run_tui(&config)? {
                executed_alias = Some(selected_alias.clone());
                success = run_configured_alias(&config, &selected_alias, &[])?;
            }
        }
        Some(Commands::External(args)) => {
            if args.is_empty() {
                Cli::command().print_help()?;
                return Ok(());
            }
            let alias = &args[0];
            let raw_extra_args = &args[1..];

            // Filter out --notify from arguments passed to the alias
            let mut extra_args = Vec::new();
            for arg in raw_extra_args {
                if arg == "--notify" {
                    should_notify = true;
                } else {
                    extra_args.push(arg.clone());
                }
            }

            executed_alias = Some(alias.clone());
            let config = load_config()?;

            success = run_configured_alias(&config, alias, &extra_args)?;
        }
        None => {
            Cli::command().print_help()?;
        }
    }

    if should_notify {
        if let Err(e) = notifications::send(success, executed_alias.as_deref()) {
            eprintln!(
                "{} Failed to send notification: {}",
                "🐙".truecolor(80, 80, 80),
                e
            );
        }
    }

    if !success {
        std::process::exit(1);
    }

    Ok(())
}

fn run_configured_alias(
    config: &crate::config::Config,
    alias: &str,
    extra_args: &[String],
) -> Result<bool> {
    if let Some(ac) = config.aliases.get(alias) {
        let start = Instant::now();

        let success = match &ac.entry {
            AliasEntry::Single(cmd) => {
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

                let failure_occurred = Arc::new(AtomicBool::new(false));
                let mut handles = vec![];

                for cmd in cmds {
                    // append extra args to each sub-command, same as single aliases do
                    let cmd_str = if !extra_args.is_empty() {
                        format!("{} {}", cmd, extra_args.join(" "))
                    } else {
                        cmd.clone()
                    };
                    let fail_flag = failure_occurred.clone();
                    handles.push(thread::spawn(move || {
                        if !execute_command(&cmd_str) {
                            fail_flag.store(true, Ordering::Relaxed);
                        }
                    }));
                }

                for h in handles {
                    let _ = h.join();
                }

                !failure_occurred.load(Ordering::Relaxed)
            }
        };

        if config.enable_timing.unwrap_or(false) {
            // round to ms so humantime doesn't print nanoseconds
            let duration = Duration::from_millis(start.elapsed().as_millis() as u64);

            if success {
                println!(
                    "{}⏱️  {}",
                    "🐙".truecolor(80, 80, 80),
                    humantime::format_duration(duration)
                );
            } else {
                eprintln!(
                    "{}⏱️  {} (Failed)",
                    "🐙".truecolor(80, 80, 80),
                    humantime::format_duration(duration)
                );
                return Ok(false);
            }
        } else if !success {
            return Ok(false);
        }
        Ok(true)
    } else {
        eprintln!("Unknown command or alias: {}", alias);
        Ok(false)
    }
}
