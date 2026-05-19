use anyhow::Result;
use colored::*;
use std::io::{self, Write};
use std::path::Path;

use crate::config::{AliasConfig, AliasEntry, Config, save_config, CONFIG_FILE};

// read a line from stdin, stripping the trailing newline
fn prompt(label: &str) -> Result<String> {
    print!("{}", label);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

// ask a yes/no question, defaulting to the given value if the user just hits enter
fn confirm(label: &str, default_yes: bool) -> Result<bool> {
    let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
    let answer = prompt(&format!("{} {} ", label, hint))?;
    Ok(match answer.to_lowercase().as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default_yes,
    })
}

pub fn run_init() -> Result<()> {
    let octopus = "🐙".truecolor(80, 80, 80);

    println!("{} Setting up cawa for this project.", octopus);
    println!();

    // warn if a config already exists so the user doesn't accidentally blow it away
    if Path::new(CONFIG_FILE).exists() {
        println!("{} {} already exists.", octopus, CONFIG_FILE.yellow());
        if !confirm("Overwrite it?", false)? {
            println!("{} Aborted.", octopus);
            return Ok(());
        }
        println!();
    }

    let identifier = {
        let val = prompt("Project identifier (optional, press enter to skip): ")?;
        if val.is_empty() { None } else { Some(val) }
    };

    let enable_timing = confirm("Enable execution timing?", false)?;

    println!();

    // optionally seed the config with a first alias so there's something to look at
    let mut aliases = std::collections::HashMap::new();
    if confirm("Add a starter alias?", true)? {
        println!();
        let name = loop {
            let n = prompt("  Alias name: ")?;
            if !n.is_empty() { break n; }
            println!("  Name can't be empty.");
        };

        let cmd = loop {
            let c = prompt("  Command: ")?;
            if !c.is_empty() { break c; }
            println!("  Command can't be empty.");
        };

        let desc = {
            let d = prompt("  Description (optional): ")?;
            if d.is_empty() { None } else { Some(d) }
        };

        aliases.insert(name.clone(), AliasConfig {
            entry: AliasEntry::Single(cmd),
            description: desc,
            timeout_secs: None,
        });

        println!();
        println!("{} Added alias {}.", octopus, name.cyan());
    }

    let config = Config {
        identifier,
        enable_timing: if enable_timing { Some(true) } else { None },
        aliases,
    };

    save_config(&config)?;

    println!();
    println!("{} {} created. You're good to go.", octopus, CONFIG_FILE.cyan());

    Ok(())
}
