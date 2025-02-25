mod api_client;
pub mod context_manager;

use anyhow::Result;

#[derive(Clone)]
pub struct LLMClient {
    pub(crate) api_client: api_client::APIClient,
    pub(crate) context_manager: context_manager::ContextManager,
}

impl LLMClient {
    pub fn new() -> Self {
        LLMClient {
            api_client: api_client::APIClient::new(),
            context_manager: context_manager::ContextManager::new(),
        }
    }

    pub async fn translate_command(&self, natural_command: &str) -> Result<String> {
        self.api_client.translate_command(natural_command).await
    }

    pub async fn get_command_explanation(&self, command: &str) -> Result<String> {
        self.api_client.get_command_explanation(command).await
    }

    pub async fn suggest_commands(&self, context: &str, command_prefix: Option<&str>) -> Result<Vec<String>> {
        self.api_client.suggest_commands(context, command_prefix).await
    }

    pub async fn chat(&self, question: &str) -> Result<String> {
        self.api_client.chat(question).await
    }
}