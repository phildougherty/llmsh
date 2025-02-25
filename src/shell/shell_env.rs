use anyhow::{Result, Context};
use std::env;
use std::fs;
use log::debug;

pub struct Environment {
    env_vars: std::collections::HashMap<String, String>,
    is_login_shell: bool,
}

impl Environment {
    pub fn new(is_login_shell: bool) -> Self {
        Environment {
            env_vars: std::collections::HashMap::new(),
            is_login_shell,
        }
    }
    
    pub fn initialize(&mut self) -> Result<()> {
        // Set basic environment variables
        self.set_default_env_vars();
        
        // Process profile files for login shells
        if self.is_login_shell {
            self.process_login_files()?;
        }
        
        // Process rc files for all shells
        self.process_rc_files()?;
        
        // Apply all environment variables
        self.apply_env_vars();
        
        Ok(())
    }
    
    fn set_default_env_vars(&mut self) {
        // Set PATH if not already set or append to it
        let default_path = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";
        
        if let Ok(current_path) = env::var("PATH") {
            if !current_path.contains(default_path) {
                self.env_vars.insert("PATH".to_string(), format!("{}:{}", current_path, default_path));
            }
        } else {
            self.env_vars.insert("PATH".to_string(), default_path.to_string());
        }
        
        // Set HOME if not already set
        if env::var("HOME").is_err() {
            if let Some(home) = dirs::home_dir() {
                self.env_vars.insert("HOME".to_string(), home.to_string_lossy().to_string());
            }
        }
        
        // Set SHELL to point to our shell
        if let Ok(exe) = env::current_exe() {
            self.env_vars.insert("SHELL".to_string(), exe.to_string_lossy().to_string());
        }
        
        // Set basic terminal variables
        self.env_vars.insert("TERM".to_string(), "xterm-256color".to_string());
    }
    
    fn process_login_files(&mut self) -> Result<()> {
        debug!("Processing login files");
        
        // Process /etc/profile
        if let Ok(content) = fs::read_to_string("/etc/profile") {
            self.parse_env_file(&content);
        }
        
        // Process ~/.profile
        let home = dirs::home_dir().context("Could not determine home directory")?;
        let profile_path = home.join(".profile");
        if let Ok(content) = fs::read_to_string(profile_path) {
            self.parse_env_file(&content);
        }
        
        // Process ~/.bash_profile or ~/.bash_login if they exist
        let bash_profile = home.join(".bash_profile");
        let bash_login = home.join(".bash_login");
        
        if bash_profile.exists() {
            if let Ok(content) = fs::read_to_string(bash_profile) {
                self.parse_env_file(&content);
            }
        } else if bash_login.exists() {
            if let Ok(content) = fs::read_to_string(bash_login) {
                self.parse_env_file(&content);
            }
        }
        
        Ok(())
    }
    
    fn process_rc_files(&mut self) -> Result<()> {
        debug!("Processing rc files");
        
        // Process /etc/bashrc
        if let Ok(content) = fs::read_to_string("/etc/bashrc") {
            self.parse_env_file(&content);
        }
        
        // Process ~/.bashrc
        let home = dirs::home_dir().context("Could not determine home directory")?;
        let bashrc_path = home.join(".bashrc");
        if let Ok(content) = fs::read_to_string(bashrc_path) {
            self.parse_env_file(&content);
        }
        
        // Process ~/.llm_shellrc if it exists
        let llm_shellrc = home.join(".llm_shellrc");
        if llm_shellrc.exists() {
            if let Ok(content) = fs::read_to_string(llm_shellrc) {
                self.parse_env_file(&content);
            }
        }
        
        Ok(())
    }
    
    fn parse_env_file(&mut self, content: &str) {
        for line in content.lines() {
            let line = line.trim();
            
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Handle export statements
            if line.starts_with("export ") {
                let parts: Vec<&str> = line["export ".len()..].splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim();
                    let value = parts[1].trim().trim_matches('"').trim_matches('\'');
                    self.env_vars.insert(key.to_string(), value.to_string());
                }
            }
        }
    }
    
    fn apply_env_vars(&self) {
        for (key, value) in &self.env_vars {
            env::set_var(key, value);
        }
    }
}