use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub fn execute_command(cmd_str: &str, timeout_secs: Option<u64>) -> bool {
    let child = Command::new("sh")
        .arg("-c")
        .arg(cmd_str)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(_) => return false,
    };

    let Some(timeout) = timeout_secs else {
        // no timeout configured, just wait normally
        return child.wait().map(|s| s.success()).unwrap_or(false);
    };

    let pid = child.id();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = child.wait().map(|s| s.success()).unwrap_or(false);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(Duration::from_secs(timeout)) {
        Ok(result) => result,
        Err(_) => {
            // kill the shell process and report failure
            let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
            false
        }
    }
}
