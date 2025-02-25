use anyhow::{Result, Context};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use dirs;

pub struct History {
    history_file: PathBuf,
    max_history_size: usize,
    entries: Vec<String>,
}

impl History {
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir().context("Could not determine home directory")?;
        let history_file = home_dir.join(".llm_shell_history");
        
        let mut history = History {
            history_file,
            max_history_size: 1000,
            entries: Vec::new(),
        };
        
        history.load()?;
        Ok(history)
    }
    
    pub fn load(&mut self) -> Result<()> {
        if !self.history_file.exists() {
            return Ok(());
        }
        
        let file = File::open(&self.history_file)?;
        let reader = BufReader::new(file);
        
        self.entries.clear();
        for line in reader.lines() {
            if let Ok(entry) = line {
                if !entry.trim().is_empty() {
                    self.entries.push(entry);
                }
            }
        }
        
        // Trim to max size
        if self.entries.len() > self.max_history_size {
            self.entries = self.entries[self.entries.len() - self.max_history_size..].to_vec();
        }
        
        Ok(())
    }
    
    pub fn save(&self) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.history_file)?;
            
        for entry in &self.entries {
            writeln!(file, "{}", entry)?;
        }
        
        Ok(())
    }
    
    pub fn add(&mut self, entry: &str) -> Result<()> {
        let entry = entry.trim();
        if entry.is_empty() {
            return Ok(());
        }
        
        // Don't add duplicate of the last command
        if let Some(last) = self.entries.last() {
            if last == entry {
                return Ok(());
            }
        }
        
        self.entries.push(entry.to_string());
        
        // Trim to max size
        if self.entries.len() > self.max_history_size {
            self.entries.remove(0);
        }
        
        self.save()?;
        Ok(())
    }
    
    pub fn get_entries(&self) -> &[String] {
        &self.entries
    }
}
