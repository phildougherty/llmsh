use crate::llm::LLMClient;
use anyhow::Result;
use std::collections::HashMap;

pub struct Documentation {
    cache: HashMap<String, String>,
    llm_client: LLMClient,
}

impl Documentation {
    pub fn new(llm_client: LLMClient) -> Self {
        Documentation {
            cache: HashMap::new(),
            llm_client,
        }
    }

    pub async fn get_command_help(&mut self, command: &str) -> Result<String> {
        if let Some(cached) = self.cache.get(command) {
            return Ok(cached.clone());
        }

        let explanation = self.llm_client.get_command_explanation(command).await?;
        self.cache.insert(command.to_string(), explanation.clone());
        Ok(explanation)
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}
