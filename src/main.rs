mod shell;
mod llm;
mod terminal;
mod system;
mod utils;
mod config;

use crate::shell::Shell;
use anyhow::Result;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    dotenv::dotenv().ok();
    
    // Handle installation if --install flag is present
    if env::args().any(|arg| arg == "--install") {
        let current_exe = env::current_exe()?;
        let installer = crate::system::installer::Installer::new(current_exe);
        installer.install()?;
        println!("LLM Shell installed successfully!");
        return Ok(());
    }
    
    let mut shell = Shell::new();
    shell.run().await?;
    
    Ok(())
}