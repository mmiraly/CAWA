use anyhow::Result;
use colored::*;
use notify_rust::Notification;
use std::path::Path;

// figure out what we're called so we can sign the note
fn get_prog_name() -> String {
    std::env::args()
        .next()
        .and_then(|s| {
            Path::new(&s)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "cs".to_string())
}

// send the actual popup thingy
pub fn send(success: bool, alias: Option<&str>) -> Result<()> {
    let program_name = get_prog_name();
    let summary = format!("{} {}", "🐙", program_name);

    // figure out what to say based on how things went
    let body = if success {
        if let Some(a) = alias {
            format!("Alias '{}' finished successfully.", a)
        } else {
            "Command finished successfully.".to_string()
        }
    } else {
        if let Some(a) = alias {
            format!("Alias '{}' failed.", a)
        } else {
            "Command failed.".to_string()
        }
    };

    // build the notification object
    let mut notification = Notification::new();
    notification.summary(&summary).body(&body);

    #[cfg(target_os = "macos")]
    {
        // macos is picky about banners, so we force a dialog box via apple script.
        // using "display dialog" so it stays until clicked.
        // no icon param cause users asked for it clean.
        let res = std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!(
                "display dialog \"{}\" with title \"{}\" buttons {{\"OK\"}} default button \"OK\"",
                body, summary
            ))
            .output();

        // if apple script complains, we should prob know why
        if let Ok(output) = res {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!(
                    "{} osascript nope'd out: {}",
                    "🐙".truecolor(80, 80, 80),
                    stderr
                );
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // linux/windows usually play nice with the standard crate
        if let Err(e) = notification.show() {
            eprintln!(
                "{} notification failed to show: {}",
                "🐙".truecolor(80, 80, 80),
                e
            );
        }
    }

    Ok(())
}
