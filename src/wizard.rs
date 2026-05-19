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
    let oct = "🐙".truecolor(80, 80, 80);

    println!("{} Setting up cawa for this project.", oct);
    println!();

    // warn if a config already exists so the user doesn't accidentally blow it away
    if Path::new(CONFIG_FILE).exists() {
        println!("{} {} already exists.", oct, CONFIG_FILE.yellow());
        if !confirm("Overwrite it?", false)? {
            println!("{} Aborted.", oct);
            return Ok(());
        }
        println!();
    }

    // alias first — gives the user something working before they touch config options
    println!("  Let's create your first alias.");
    println!("  {}", "Aliases are shortcuts to commands you run often in this project.".dimmed());
    println!();

    let mut aliases = std::collections::HashMap::new();

    let name = loop {
        let n = prompt("  Alias name (e.g. build, test, ship): ")?;
        if !n.is_empty() { break n; }
        println!("  Name can't be empty.");
    };

    let cmd = loop {
        let c = prompt("  Command to run: ")?;
        if !c.is_empty() { break c; }
        println!("  Command can't be empty.");
    };

    let desc = {
        let d = prompt("  Short description (optional, shown in cs list and tui): ")?;
        if d.is_empty() { None } else { Some(d) }
    };

    aliases.insert(name.clone(), AliasConfig {
        entry: AliasEntry::Single(cmd),
        description: desc,
        timeout_secs: None,
    });

    println!();
    println!("  {} Alias {} created.", oct, name.cyan());
    println!();

    // project identifier — explain what it's actually for before asking
    println!("  {}", "Optional: give this project a name. It shows up as a label".dimmed());
    println!("  {}", "in the config file and helps when sharing aliases with a team.".dimmed());
    let identifier = {
        let val = prompt("  Project name (press enter to skip): ")?;
        if val.is_empty() { None } else { Some(val) }
    };

    println!();

    // timing — explain what it does so the y/n isn't a guess
    println!("  {}", "Optional: print how long each command took after it finishes.".dimmed());
    println!("  {}", "Useful for slow builds. Can be toggled in .cawa_cfg.json later.".dimmed());
    let enable_timing = confirm("  Enable execution timing?", false)?;

    let config = Config {
        identifier,
        enable_timing: if enable_timing { Some(true) } else { None },
        aliases,
    };

    save_config(&config)?;

    println!();
    println!("{} {} created. Run {} to try your first alias.", oct, CONFIG_FILE.cyan(), format!("cs {}", name).bold());

    Ok(())
}
