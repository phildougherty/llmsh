use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashSet;

pub struct CompletionEngine {
    commands: HashSet<String>,
}

impl CompletionEngine {
    pub fn new() -> Self {
        CompletionEngine {
            commands: HashSet::new(),
        }
    }
    
    pub fn initialize(&mut self) -> Result<()> {
        // Load commands from PATH
        self.load_commands_from_path()?;
        
        // Add built-in commands
        self.add_builtin_commands();
        
        Ok(())
    }
    
    fn load_commands_from_path(&mut self) -> Result<()> {
        if let Ok(path) = std::env::var("PATH") {
            for path_entry in path.split(':') {
                let path_dir = Path::new(path_entry);
                if path_dir.exists() && path_dir.is_dir() {
                    if let Ok(entries) = fs::read_dir(path_dir) {
                        for entry in entries.flatten() {
                            if let Ok(file_type) = entry.file_type() {
                                if file_type.is_file() {
                                    if let Some(name) = entry.file_name().to_str() {
                                        // Check if the file is executable
                                        if let Ok(metadata) = entry.metadata() {
                                            let permissions = metadata.permissions();
                                            #[cfg(unix)]
                                            {
                                                use std::os::unix::fs::PermissionsExt;
                                                if permissions.mode() & 0o111 != 0 {
                                                    self.commands.insert(name.to_string());
                                                }
                                            }
                                            #[cfg(not(unix))]
                                            {
                                                self.commands.insert(name.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn add_builtin_commands(&mut self) {
        // Add shell built-ins
        let builtins = [
            "cd", "alias", "unalias", "exit", "help", "jobs", "fg", "bg",
            "echo", "export", "source", ".", "history", "pwd", "type",
        ];
        
        for cmd in builtins {
            self.commands.insert(cmd.to_string());
        }
    }
    
    pub fn get_commands(&self) -> Vec<String> {
        self.commands.iter().cloned().collect()
    }
    
    pub fn complete_command(&self, partial: &str) -> Vec<String> {
        self.commands
            .iter()
            .filter(|cmd| cmd.starts_with(partial))
            .cloned()
            .collect()
    }
    
    pub fn complete_path(&self, partial: &str) -> Vec<String> {
        let mut results = Vec::new();
        
        // Handle home directory expansion
        let expanded_partial = if partial.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                if partial.len() == 1 {
                    // Just "~"
                    home.to_string_lossy().to_string()
                } else {
                    // "~/something"
                    home.join(&partial[2..]).to_string_lossy().to_string()
                }
            } else {
                partial.to_string()
            }
        } else {
            partial.to_string()
        };
        
        // Split into directory and file parts
        let (dir_part, file_part) = if let Some(last_slash) = expanded_partial.rfind('/') {
            let dir = &expanded_partial[..=last_slash];
            let file = &expanded_partial[last_slash + 1..];
            (PathBuf::from(dir), file.to_string())
        } else {
            (PathBuf::from("."), expanded_partial)
        };
        
        // Read directory entries
        if dir_part.exists() && dir_part.is_dir() {
            if let Ok(entries) = fs::read_dir(&dir_part) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with(&file_part) {
                            let mut full_path = dir_part.join(name);
                            
                            // Add trailing slash for directories
                            if let Ok(metadata) = entry.metadata() {
                                if metadata.is_dir() {
                                    full_path = full_path.join("");
                                }
                            }
                            
                            results.push(full_path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
        
        results
    }
}