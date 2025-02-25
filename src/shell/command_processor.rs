// src/shell/command_processor.rs
use anyhow::Result;

#[derive(Debug)]
pub struct Command {
    pub command: String,
    pub is_natural_language: bool,
}

pub struct CommandProcessor;

impl CommandProcessor {
    pub fn new() -> Self {
        CommandProcessor
    }

    pub fn parse(&self, input: &str) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        
        // Split by semicolons to handle multiple commands
        for cmd_str in input.split(';') {
            let trimmed = cmd_str.trim();
            if trimmed.is_empty() {
                continue;
            }
            
            // Check if this looks like natural language
            let is_natural_language = self.detect_natural_language(trimmed);
            
            commands.push(Command {
                command: trimmed.to_string(),
                is_natural_language,
            });
        }
        
        Ok(commands)
    }
    
    fn detect_natural_language(&self, input: &str) -> bool {
        // Simple heuristic: if it has multiple words and doesn't start with a common command
        let common_commands = [
            "ls", "cd", "grep", "find", "cat", "echo", "mkdir", "rm", "cp", "mv",
            "git", "docker", "ssh", "sudo", "apt", "yum", "dnf", "pacman", "brew",
            "python", "node", "npm", "cargo", "rustc", "gcc", "make", "ps", "top",
            "kill", "systemctl", "journalctl", "curl", "wget", "tar", "zip", "unzip",
        ];
        
        let words: Vec<&str> = input.split_whitespace().collect();
        if words.is_empty() {
            return false;
        }
        
        // If it starts with a common command, probably not natural language
        if common_commands.contains(&words[0]) {
            return false;
        }
        
        // If it has 4+ words, likely natural language
        if words.len() >= 4 {
            return true;
        }
        
        // Check for natural language patterns
        let natural_patterns = [
            "show", "find", "list", "get", "display", "create", "make", "tell",
            "give", "use", "how", "what", "where", "can", "could", "would", "should",
            "explain", "help", "search", "look", "count", "calculate", "summarize",
        ];
        
        natural_patterns.iter().any(|&pattern| words[0].eq_ignore_ascii_case(pattern))
    }
}