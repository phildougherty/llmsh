use lazy_static::lazy_static;
use std::sync::Arc;

#[derive(Clone)]
pub struct Config {
    pub llm_host: String,
    pub llm_model: String,
    pub max_context_items: usize,
    pub suggestion_count: usize,
    pub command_preview: bool,
}

lazy_static! {
    pub static ref CONFIG: Arc<Config> = Arc::new(Config {
        llm_host: "http://192.168.86.201:11434".to_string(),
        llm_model: "qwen2.5:14b".to_string(),
        max_context_items: 10,
        suggestion_count: 3,
        command_preview: true,
    });
}
