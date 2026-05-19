mod cli;
mod config;
mod notifications;
mod runner;
mod tui;
mod wizard;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use colored::*;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::cli::{Cli, Commands};
use crate::config::{AliasConfig, AliasEntry, load_config, load_global_config, load_merged_config, load_state, save_config, save_global_config, save_state, unix_now};
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
    let dry_run = args.dry_run;

    let mut executed_alias = None;

    match args.command {
        // ... (Add, Remove, List unchanged)
        Some(Commands::Add {
            parallel,
            desc,
            timeout,
            global,
            alias,
            commands,
        }) => {
            let mut config = if global { load_global_config()? } else { load_config()? };

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

            config.aliases.insert(alias.clone(), AliasConfig { entry, description: desc, timeout_secs: timeout });
            if global { save_global_config(&config)?; } else { save_config(&config)?; }

            println!(
                "{} {} now stores {}",
                "🐙".truecolor(80, 80, 80),
                program_name.bold(),
                display_val.cyan()
            );
        }
        Some(Commands::Remove { global, alias }) => {
            let mut config = if global { load_global_config()? } else { load_config()? };
            if config.aliases.remove(&alias).is_some() { // remove returns the old value if it existed
                if global { save_global_config(&config)?; } else { save_config(&config)?; }
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
            let local = load_config()?;
            let global_cfg = load_global_config().unwrap_or_default();
            let state = load_state();
            let now = unix_now();

            // build a sorted list: local aliases + global-only ones tagged with [global]
            let mut entries: Vec<(String, &AliasConfig, bool)> = Vec::new();
            for (k, v) in &local.aliases {
                entries.push((k.clone(), v, false));
            }
            for (k, v) in &global_cfg.aliases {
                if !local.aliases.contains_key(k) {
                    entries.push((k.clone(), v, true));
                }
            }

            if entries.is_empty() {
                println!("No aliases found.");
            } else {
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                println!("{} Aliases", "🐙".truecolor(80, 80, 80));
                // sort so the output is stable across runs
                for (alias, ac, is_global) in entries {
                    let tag = if is_global { " [global]".dimmed().to_string() } else { String::new() };
                    match &ac.entry {
                        AliasEntry::Single(s) => {
                            println!("{} {}{} → {}", program_name.dimmed(), alias.bold(), tag, s.cyan());
                        }
                        AliasEntry::Parallel(cmds) => {
                            println!("{} {}{} → {}", program_name.dimmed(), alias.bold(), tag, "[parallel]".yellow());
                            for cmd in cmds {
                                println!("    {} {}", "└".dimmed(), cmd.cyan());
                            }
                        }
                    }
                    if let Some(desc) = &ac.description {
                        println!("    {} {}", "ℹ".dimmed(), desc.dimmed());
                    }
                    if let Some(&last) = state.get(&alias) {
                        let age = std::time::Duration::from_secs(now.saturating_sub(last));
                        println!("    {} ran {} ago", "⏱".dimmed(), humantime::format_duration(age));
                    }
                }
            }
        }
        Some(Commands::Init) => {
            wizard::run_init()?;
        }
        Some(Commands::Edit { global, alias }) => {
            let mut config = if global { load_global_config()? } else { load_config()? };

            if let Some(ac) = config.aliases.get(&alias).cloned() {
                let tmp = std::env::temp_dir().join(format!("cawa_edit_{}.txt", unix_now()));
                let contents = match &ac.entry {
                    AliasEntry::Single(cmd) => cmd.clone(),
                    // parallel entries get one command per line so the user can add/remove/reorder
                    AliasEntry::Parallel(cmds) => cmds.join("\n"),
                };
                std::fs::write(&tmp, &contents)?;

                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
                let status = std::process::Command::new(&editor).arg(&tmp).status()?;
                let edited = std::fs::read_to_string(&tmp)?;
                let _ = std::fs::remove_file(&tmp);

                if !status.success() {
                    eprintln!("Editor exited with an error, alias unchanged.");
                } else {
                    let lines: Vec<String> = edited
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .map(|l| l.to_string())
                        .collect();

                    let new_entry = match lines.len() {
                        0 => { eprintln!("Editor result was empty, alias unchanged."); return Ok(()); }
                        1 => AliasEntry::Single(lines[0].clone()),
                        _ => AliasEntry::Parallel(lines),
                    };

                    config.aliases.insert(alias.clone(), AliasConfig {
                        entry: new_entry,
                        description: ac.description,
                        timeout_secs: ac.timeout_secs,
                    });
                    if global { save_global_config(&config)?; } else { save_config(&config)?; }
                    println!("{} {} updated.", "🐙".truecolor(80, 80, 80), alias.cyan());
                }
            } else {
                eprintln!("Alias '{}' not found.", alias);
            }
        }
        Some(Commands::Rename { old_alias, new_alias }) => {
            let mut config = load_config()?;
            if let Some(entry) = config.aliases.remove(&old_alias) {
                config.aliases.insert(new_alias.clone(), entry);
                save_config(&config)?;
                println!(
                    "{} {} → {}",
                    "🐙".truecolor(80, 80, 80),
                    old_alias.red(),
                    new_alias.cyan()
                );
            } else {
                eprintln!("Alias '{}' not found.", old_alias);
            }
        }
        Some(Commands::Run { parallel, timeout, commands }) => {
            let config = load_config()?;
            let entry = if parallel {
                AliasEntry::Parallel(commands)
            } else {
                AliasEntry::Single(commands.join(" "))
            };
            // one-off run: no alias name to look up, just execute directly
            success = run_entry(&entry, &[], config.enable_timing.unwrap_or(false), dry_run, timeout)?;
        }
        Some(Commands::Tui) => {
            // use merged so global aliases appear in the TUI
            let config = load_merged_config()?;
            if let Some(selected_alias) = tui::run_tui(&config)? {
                executed_alias = Some(selected_alias.clone());
                success = run_configured_alias(&config, &selected_alias, &[], dry_run)?;
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
            // use merged so global aliases are reachable by name
            let config = load_merged_config()?;

            success = run_configured_alias(&config, alias, &extra_args, dry_run)?;
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
    dry_run: bool,
) -> Result<bool> {
    if let Some(ac) = config.aliases.get(alias) {
        let result = run_entry(&ac.entry, extra_args, config.enable_timing.unwrap_or(false), dry_run, ac.timeout_secs)?;
        // record the run timestamp so cs list can show when this was last used
        if result && !dry_run {
            let mut state = load_state();
            state.insert(alias.to_string(), unix_now());
            let _ = save_state(&state);
        }
        Ok(result)
    } else {
        eprintln!("Unknown command or alias: {}", alias);
        Ok(false)
    }
}

fn run_entry(
    entry: &AliasEntry,
    extra_args: &[String],
    enable_timing: bool,
    dry_run: bool,
    timeout_secs: Option<u64>,
) -> Result<bool> {
    let start = Instant::now();

    let success = match entry {
        AliasEntry::Single(cmd) => {
            let final_cmd = if !extra_args.is_empty() {
                format!("{} {}", cmd, extra_args.join(" "))
            } else {
                cmd.clone()
            };
            if dry_run {
                println!("{} Would run: {}", "🐙".truecolor(80, 80, 80), final_cmd.cyan());
                true
            } else {
                println!("{} Executing: {}", "🐙".truecolor(80, 80, 80), final_cmd.cyan());
                execute_command(&final_cmd, timeout_secs)
            }
        }
        AliasEntry::Parallel(cmds) => {
            if dry_run {
                println!("{} Would run (parallel):", "🐙".truecolor(80, 80, 80));
                for cmd in cmds {
                    let full = if !extra_args.is_empty() {
                        format!("{} {}", cmd, extra_args.join(" "))
                    } else {
                        cmd.clone()
                    };
                    println!("    {} {}", "└".dimmed(), full.cyan());
                }
                return Ok(true);
            }

            println!("{} Executing (parallel): {:?}", "🐙".truecolor(80, 80, 80), cmds);

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
                    if !execute_command(&cmd_str, timeout_secs) {
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

    if enable_timing {
        // round to ms so humantime doesn't print nanoseconds
        let duration = Duration::from_millis(start.elapsed().as_millis() as u64);
        if success {
            println!("{}⏱️  {}", "🐙".truecolor(80, 80, 80), humantime::format_duration(duration));
        } else {
            eprintln!("{}⏱️  {} (Failed)", "🐙".truecolor(80, 80, 80), humantime::format_duration(duration));
            return Ok(false);
        }
    } else if !success {
        return Ok(false);
    }

    Ok(true)
}
