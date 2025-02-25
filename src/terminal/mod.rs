mod history;
mod completion;

use anyhow::Result;
use rustyline::{DefaultEditor, Config, EditMode};
use std::path::PathBuf;
use colored::*;
use std::env;
use std::process::Command;
use self::history::History;
use self::completion::CompletionEngine;

pub struct Terminal {
    editor: DefaultEditor,
    history: History,
    completion_engine: CompletionEngine,
}

impl Terminal {
    pub fn new() -> Self {
        // Configure rustyline
        let config = Config::builder()
            .edit_mode(EditMode::Emacs)
            .auto_add_history(false)
            .completion_type(rustyline::CompletionType::List)
            .build();
            
        let editor = DefaultEditor::with_config(config).unwrap_or_else(|_| DefaultEditor::new().unwrap());
        
        // Initialize history
        let history = History::new().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to initialize history: {}", e);
            History::new().unwrap()
        });
        
        // Initialize completion engine
        let mut completion_engine = CompletionEngine::new();
        completion_engine.initialize().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to initialize completion engine: {}", e);
        });
        
        Terminal {
            editor,
            history,
            completion_engine,
        }
    }

    pub fn read_line(&mut self) -> Result<(String, bool)> {
        let prompt = self.create_prompt()?;
        
        // Read input with tab completion
        let line = match self.editor.readline(&prompt) {
            Ok(line) => line,
            Err(err) => {
                // Handle different error types
                if err.to_string().contains("interrupted") {
                    // Ctrl+C was pressed
                    return Ok(("".to_string(), false));
                } else if err.to_string().contains("eof") {
                    // Ctrl+D was pressed - exit
                    return Ok(("exit".to_string(), false));
                } else {
                    return Err(anyhow::anyhow!("Error reading input: {}", err));
                }
            }
        };
        
        let trimmed = line.trim();
        
        // Consider showing suggestions if the line ends with '??'
        let show_suggestions = trimmed.ends_with("??");
        let line = trimmed.trim_end_matches('?').to_string();
        
        // Add to history if non-empty
        if !line.is_empty() {
            self.history.add(&line)?;
            self.editor.add_history_entry(&line)?;
        }
        
        Ok((line, show_suggestions))
    }

    fn create_prompt(&self) -> Result<String> {
        let cwd = env::current_dir()?;
        let home = dirs::home_dir().unwrap_or_default();
        let path = self.shorten_path(cwd, &home);
        
        let username = env::var("USER").unwrap_or_else(|_| "user".to_string());
        let hostname = self.get_hostname();
        let git_info = self.get_git_info()?;
        
        // Create a fancy multi-line prompt
        Ok(format!("\n{}{}{}{}{}",
            "┌─[".bright_blue(),
            username.bright_green(),
            "@".bright_blue(),
            hostname.bright_cyan(),
            "]".bright_blue(),
        ) + &format!("─[{}]", path.bright_yellow()) + &git_info + "\n" +
            &format!("└─{} ", "❯".bright_purple()))
    }

    fn get_hostname(&self) -> String {
        if let Ok(hostname) = Command::new("hostname")
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string()) {
            hostname
        } else {
            "unknown".to_string()
        }
    }

    fn shorten_path(&self, path: PathBuf, home: &PathBuf) -> String {
        let path_str = path.to_string_lossy();
        if let Ok(stripped) = path.strip_prefix(home) {
            if stripped.as_os_str().is_empty() {
                "~".to_string()
            } else {
                format!("~/{}", stripped.to_string_lossy())
            }
        } else {
            path_str.to_string()
        }
    }

    fn get_git_info(&self) -> Result<String> {
        // First check if we're in a git repository
        let is_git_repo = Command::new("git")
            .args(&["rev-parse", "--is-inside-work-tree"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        
        if !is_git_repo {
            return Ok(String::new());
        }
        
        // Try to get git branch
        let branch = Command::new("git")
            .args(&["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok()
                } else {
                    None
                }
            });
        
        // Try to get git status
        let status_clean = Command::new("git")
            .args(&["diff", "--quiet"])
            .status()
            .map(|status| status.success())
            .unwrap_or(true);
        
        match branch {
            Some(branch) => {
                let branch = branch.trim();
                let status_symbol = if status_clean {
                    "✓".green()
                } else {
                    "✗".red()
                };
                
                // Get ahead/behind status
                let ahead_behind = self.get_git_ahead_behind()?;
                
                Ok(format!("─[{}{}{}", 
                    branch.bright_purple(), 
                    status_symbol,
                    ahead_behind
                ) + "]")
            }
            None => Ok(String::new())
        }
    }
    
    fn get_git_ahead_behind(&self) -> Result<String> {
        // Get ahead/behind counts
        let output = Command::new("git")
            .args(&["rev-list", "--count", "--left-right", "@{upstream}...HEAD"])
            .output();
            
        match output {
            Ok(output) => {
                if output.status.success() {
                    let counts = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    let parts: Vec<&str> = counts.split_whitespace().collect();
                    
                    if parts.len() == 2 {
                        let behind = parts[0].parse::<usize>().unwrap_or(0);
                        let ahead = parts[1].parse::<usize>().unwrap_or(0);
                        
                        let mut status = String::new();
                        if ahead > 0 {
                            status.push_str(&format!(" ↑{}", ahead).yellow().to_string());
                        }
                        if behind > 0 {
                            status.push_str(&format!(" ↓{}", behind).red().to_string());
                        }
                        
                        Ok(status)
                    } else {
                        Ok(String::new())
                    }
                } else {
                    // Not tracking a remote branch
                    Ok(String::new())
                }
            }
            Err(_) => Ok(String::new())
        }
    }
    
    pub fn get_history(&self) -> &History {
        &self.history
    }
    
    pub fn add_to_history(&mut self, entry: &str) -> Result<()> {
        self.history.add(entry)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // Save history when terminal is dropped
        if let Err(e) = self.history.save() {
            eprintln!("Warning: Failed to save history: {}", e);
        }
    }
}