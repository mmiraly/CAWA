use std::process::{Command, Stdio};

pub fn execute_command(cmd_str: &str) -> bool {
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
