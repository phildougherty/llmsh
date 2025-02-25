// src/shell/executor.rs
use anyhow::{Result, Context};
use std::process::{Command, Stdio};
use std::fs::{File, OpenOptions};
use std::io::Write;
use crate::shell::command_parser::{Pipeline, SimpleCommand, Redirection};
use crate::utils::path_utils;

pub struct Executor;

impl Executor {
    pub fn execute(pipeline: &Pipeline) -> Result<i32> {
        if pipeline.commands.is_empty() {
            return Ok(0);
        }
        
        // Single command without pipes
        if pipeline.commands.len() == 1 && !pipeline.commands[0].redirections.contains(&Redirection::Pipe) {
            return Self::execute_simple_command(&pipeline.commands[0], pipeline.background);
        }
        
        // Pipeline with multiple commands
        let mut children = Vec::new();
        let mut prev_stdout = None;
        
        for (i, cmd) in pipeline.commands.iter().enumerate() {
            let is_last = i == pipeline.commands.len() - 1;
            
            // Set up stdin from previous command's stdout
            let stdin = if let Some(prev_out) = prev_stdout.take() {
                Stdio::from(prev_out)
            } else {
                Stdio::inherit()
            };
            
            // Set up stdout for piping to next command
            let stdout = if is_last {
                Stdio::inherit()
            } else {
                Stdio::piped()
            };
            
            // Create the command
            let mut command = Self::create_command(cmd)?;
            command.stdin(stdin);
            command.stdout(stdout);
            
            // Apply redirections
            Self::apply_redirections(&mut command, cmd)?;
            
            // Spawn the command
            let mut child = command.spawn()
                .with_context(|| format!("Failed to spawn command: {}", cmd.program))?;
            
            // Save stdout for the next command if not the last command
            if !is_last {
                prev_stdout = child.stdout.take();
            }
            
            // Add to list of children
            children.push(child);
        }
        
        // Wait for all children to complete
        let mut exit_code = 0;
        for mut child in children {
            let status = child.wait()
                .with_context(|| "Failed to wait for child process")?;
            if !status.success() {
                exit_code = status.code().unwrap_or(1);
            }
        }
        
        Ok(exit_code)
    }
    
    fn execute_simple_command(cmd: &SimpleCommand, background: bool) -> Result<i32> {
        // Create the command
        let mut command = Self::create_command(cmd)?;
        
        // Apply redirections
        Self::apply_redirections(&mut command, cmd)?;
        
        if background {
            // Run in background
            let child = command.spawn()
                .with_context(|| format!("Failed to spawn command: {}", cmd.program))?;
            println!("[{}] {}", child.id(), cmd.program);
            Ok(0)
        } else {
            // Run in foreground
            let status = command.status()
                .with_context(|| format!("Failed to execute command: {}", cmd.program))?;
            Ok(status.code().unwrap_or(0))
        }
    }
    
    fn create_command(cmd: &SimpleCommand) -> Result<Command> {
        // Find the executable
        let executable = path_utils::find_executable(&cmd.program)
            .with_context(|| format!("Command not found: {}", cmd.program))?;
        
        // Create the command
        let mut command = Command::new(executable);
        
        // Add arguments
        command.args(&cmd.args);
        
        Ok(command)
    }
    
    fn apply_redirections(command: &mut Command, cmd: &SimpleCommand) -> Result<()> {
        for redirection in &cmd.redirections {
            match redirection {
                Redirection::Input(filename) => {
                    let file = File::open(filename)
                        .with_context(|| format!("Failed to open file for input: {}", filename))?;
                    command.stdin(Stdio::from(file));
                },
                Redirection::Output(filename) => {
                    let file = File::create(filename)
                        .with_context(|| format!("Failed to create file for output: {}", filename))?;
                    command.stdout(Stdio::from(file));
                },
                Redirection::Append(filename) => {
                    let file = OpenOptions::new()
                        .write(true)
                        .append(true)
                        .create(true)
                        .open(filename)
                        .with_context(|| format!("Failed to open file for append: {}", filename))?;
                    command.stdout(Stdio::from(file));
                },
                Redirection::ErrorOutput(filename) => {
                    let file = File::create(filename)
                        .with_context(|| format!("Failed to create file for error output: {}", filename))?;
                    command.stderr(Stdio::from(file));
                },
                Redirection::ErrorAppend(filename) => {
                    let file = OpenOptions::new()
                        .write(true)
                        .append(true)
                        .create(true)
                        .open(filename)
                        .with_context(|| format!("Failed to open file for error append: {}", filename))?;
                    command.stderr(Stdio::from(file));
                },
                Redirection::Pipe => {
                    // Pipes are handled separately
                },
            }
        }
        
        Ok(())
    }
}