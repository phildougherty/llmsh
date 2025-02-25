use anyhow::Result;
use std::collections::HashMap;
use std::fs;

pub struct AliasManager {
    aliases: HashMap<String, String>,
}

impl AliasManager {
    pub fn new() -> Self {
        AliasManager {
            aliases: HashMap::new(),
        }
    }
    
    pub fn initialize(&mut self) -> Result<()> {
        // Load system aliases
        if let Ok(content) = fs::read_to_string("/etc/bash.bashrc") {
            self.parse_aliases(&content);
        }
        
        // Load user aliases
        if let Some(home) = dirs::home_dir() {
            let bashrc = home.join(".bashrc");
            if let Ok(content) = fs::read_to_string(bashrc) {
                self.parse_aliases(&content);
            }
            
            // Load custom aliases file if it exists
            let aliases_file = home.join(".llm_shell_aliases");
            if aliases_file.exists() {
                if let Ok(content) = fs::read_to_string(aliases_file) {
                    self.parse_aliases(&content);
                }
            }
        }
        
        // Add some default aliases
        self.add_default_aliases();
        
        Ok(())
    }
    
    fn parse_aliases(&mut self, content: &str) {
        for line in content.lines() {
            let line = line.trim();
            
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Parse alias definitions
            if line.starts_with("alias ") {
                let alias_def = &line["alias ".len()..];
                if let Some(equals_pos) = alias_def.find('=') {
                    let name = alias_def[..equals_pos].trim();
                    let mut value = alias_def[equals_pos + 1..].trim();
                    
                    // Remove surrounding quotes if present
                    if (value.starts_with('\'') && value.ends_with('\'')) || 
                       (value.starts_with('"') && value.ends_with('"')) {
                        value = &value[1..value.len() - 1];
                    }
                    
                    self.aliases.insert(name.to_string(), value.to_string());
                }
            }
        }
    }
    
    fn add_default_aliases(&mut self) {
        // Add some useful default aliases
        self.aliases.insert("ll".to_string(), "ls -la".to_string());
        self.aliases.insert("la".to_string(), "ls -A".to_string());
        self.aliases.insert("l".to_string(), "ls -CF".to_string());
        self.aliases.insert("..".to_string(), "cd ..".to_string());
        self.aliases.insert("...".to_string(), "cd ../..".to_string());
    }
    
    pub fn expand(&self, command: &str) -> String {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return command.to_string();
        }
        
        if let Some(alias) = self.aliases.get(parts[0]) {
            if parts.len() > 1 {
                format!("{} {}", alias, parts[1..].join(" "))
            } else {
                alias.clone()
            }
        } else {
            command.to_string()
        }
    }
    
    pub fn add_alias(&mut self, name: &str, value: &str) -> Result<()> {
        self.aliases.insert(name.to_string(), value.to_string());
        self.save_aliases()?;
        Ok(())
    }
    
    pub fn remove_alias(&mut self, name: &str) -> Result<()> {
        self.aliases.remove(name);
        self.save_aliases()?;
        Ok(())
    }
    
    pub fn list_aliases(&self) -> Vec<(String, String)> {
        self.aliases
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
    
    fn save_aliases(&self) -> Result<()> {
        if let Some(home) = dirs::home_dir() {
            let aliases_file = home.join(".llm_shell_aliases");
            let mut content = String::new();
            
            for (name, value) in &self.aliases {
                content.push_str(&format!("alias {}='{}'\n", name, value));
            }
            
            fs::write(aliases_file, content)?;
        }
        
        Ok(())
    }
}