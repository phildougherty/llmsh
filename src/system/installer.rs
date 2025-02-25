use anyhow::Result;
use std::path::PathBuf;
use std::fs;

pub struct Installer {
    binary_path: PathBuf,
}

impl Installer {
    pub fn new(binary_path: PathBuf) -> Self {
        Installer { binary_path }
    }

    pub fn install(&self) -> Result<()> {
        self.copy_binary()?;
        self.update_shells_file()?;
        Ok(())
    }

    fn copy_binary(&self) -> Result<()> {
        fs::copy(&self.binary_path, "/usr/bin/llm-shell")?;
        Ok(())
    }

    fn update_shells_file(&self) -> Result<()> {
        let shells_path = "/etc/shells";
        let shell_path = "/usr/bin/llm-shell";
        
        let content = fs::read_to_string(shells_path)?;
        if !content.contains(shell_path) {
            fs::write(shells_path, format!("{}\n{}", content, shell_path))?;
        }
        
        Ok(())
    }
}
