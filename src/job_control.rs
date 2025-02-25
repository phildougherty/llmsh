use anyhow::Result;
use std::process::{Command, Stdio};

pub struct JobControl {
    background_jobs: Vec<u32>, // PIDs of background processes
}

impl JobControl {
    pub fn new() -> Self {
        JobControl {
            background_jobs: Vec::new(),
        }
    }

    pub fn execute(&mut self, command: &str) -> Result<()> {
        let parts: Vec<&str> = shellwords::split(command)?
            .iter()
            .map(|s| s.as_str())
            .collect();
        if parts.is_empty() {
            return Ok(());
        }

        let background = command.ends_with('&');
        let command_str = if background {
            command[..command.len()-1].trim()
        } else {
            command
        };

        let mut cmd = Command::new(parts[0]);
        cmd.args(&parts[1..])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let child = cmd.spawn()?;

        if background {
            self.background_jobs.push(child.id());
            println!("[{}] {}", self.background_jobs.len(), child.id());
        } else {
            let status = child.wait()?;
            if !status.success() {
                eprintln!("Command failed with exit code: {}", status.code().unwrap_or(-1));
            }
        }

        Ok(())
    }
}
