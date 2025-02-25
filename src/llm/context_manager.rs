#[derive(Clone)]
pub struct ContextManager {
    current_dir: String,
    last_commands: Vec<String>,
}

impl ContextManager {
    pub fn new() -> Self {
        ContextManager {
            current_dir: std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            last_commands: Vec::new(),
        }
    }

    pub fn get_context(&self) -> String {
        format!(
            "Current directory: {}. Last commands: {}",
            self.current_dir,
            self.last_commands.join(", ")
        )
    }

    pub fn update_directory(&mut self, new_dir: &str) {
        self.current_dir = new_dir.to_string();
    }

    pub fn add_command(&mut self, command: &str) {
        self.last_commands.push(command.to_string());
        if self.last_commands.len() > 5 {
            self.last_commands.remove(0);
        }
    }
}