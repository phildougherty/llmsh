use std::path::PathBuf;
use anyhow::Result;

pub struct LoginShell {
    home_dir: PathBuf,
}

impl LoginShell {
    pub fn new() -> Result<Self> {
        Ok(LoginShell {
            home_dir: dirs::home_dir().unwrap_or_default(),
        })
    }

    pub fn initialize(&self) -> Result<()> {
        self.process_profile_files()?;
        self.setup_environment()?;
        Ok(())
    }

    fn process_profile_files(&self) -> Result<()> {
        // Process global profile
        if let Ok(contents) = std::fs::read_to_string("/etc/profile") {
            self.process_profile_content(&contents)?;
        }

        // Process user profile
        let profile_path = self.home_dir.join(".profile");
        if let Ok(contents) = std::fs::read_to_string(profile_path) {
            self.process_profile_content(&contents)?;
        }

        Ok(())
    }

    fn process_profile_content(&self, content: &str) -> Result<()> {
        for line in content.lines() {
            if line.starts_with("export ") {
                let parts: Vec<&str> = line["export ".len()..].splitn(2, '=').collect();
                if parts.len() == 2 {
                    std::env::set_var(parts[0].trim(), parts[1].trim().trim_matches('"'));
                }
            }
        }
        Ok(())
    }

    fn setup_environment(&self) -> Result<()> {
        if std::env::var("PATH").is_err() {
            std::env::set_var("PATH", "/usr/local/bin:/usr/bin:/bin");
        }
        Ok(())
    }
}
