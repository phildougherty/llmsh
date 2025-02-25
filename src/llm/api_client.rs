use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::config::CONFIG;
use regex::Regex;
use lazy_static::lazy_static;

#[derive(Clone)]
pub struct APIClient {
    client: Client,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

lazy_static! {
    static ref CODE_BLOCK_RE: Regex = Regex::new(r"```(?:shell|bash)?\s*([^`]+)```").unwrap();
}

impl APIClient {
    pub fn new() -> Self {
        APIClient {
            client: Client::new(),
        }
    }

    pub async fn chat(&self, question: &str) -> Result<String> {
        let request = OllamaRequest {
            model: CONFIG.llm_model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "You are a helpful command-line assistant. Provide clear, concise answers.".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: question.to_string(),
                },
            ],
            stream: false,
        };

        let response = self.client
            .post(format!("{}/v1/chat/completions", CONFIG.llm_host))
            .json(&request)
            .send()
            .await?
            .json::<OllamaResponse>()
            .await?;

        Ok(response.choices[0].message.content.trim().to_string())
    }

    pub async fn translate_command(&self, natural_command: &str) -> Result<String> {
        let request = OllamaRequest {
            model: CONFIG.llm_model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "You are a shell command translator. Convert natural language to shell commands. Respond ONLY with the exact command to execute, nothing else. No markdown, no explanations.".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: natural_command.to_string(),
                },
            ],
            stream: false,
        };

        let response = self.client
            .post(format!("{}/v1/chat/completions", CONFIG.llm_host))
            .json(&request)
            .send()
            .await?
            .json::<OllamaResponse>()
            .await?;

        let command = response.choices[0].message.content.clone();
        Ok(self.clean_command_output(&command))
    }

    pub async fn get_command_explanation(&self, command: &str) -> Result<String> {
        let request = OllamaRequest {
            model: CONFIG.llm_model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "Explain what this shell command does in one brief sentence:".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: command.to_string(),
                },
            ],
            stream: false,
        };

        let response = self.client
            .post(format!("{}/v1/chat/completions", CONFIG.llm_host))
            .json(&request)
            .send()
            .await?
            .json::<OllamaResponse>()
            .await?;

        Ok(response.choices[0].message.content.trim().to_string())
    }

    pub async fn suggest_commands(&self, context: &str, command_prefix: Option<&str>) -> Result<Vec<String>> {
        let system_prompt = if let Some(prefix) = command_prefix {
            format!(
                "Suggest 3 useful variations or related commands for '{}'. Provide only the commands, one per line, no explanations.",
                prefix
            )
        } else {
            "Suggest 3 useful shell commands based on the current context. Provide only the commands, one per line, no explanations.".to_string()
        };
    
        let request = OllamaRequest {
            model: CONFIG.llm_model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                Message {
                    role: "user".to_string(),
                    content: context.to_string(),
                },
            ],
            stream: false,
        };
    
        let response = self.client
            .post(format!("{}/v1/chat/completions", CONFIG.llm_host))
            .json(&request)
            .send()
            .await?
            .json::<OllamaResponse>()
            .await?;
    
        Ok(response.choices[0].message.content
            .lines()
            .map(|s| self.clean_command_output(s))
            .filter(|s| !s.is_empty())
            .collect())
    }

    fn clean_command_output(&self, output: &str) -> String {
        // First try to extract command from code blocks
        if let Some(captures) = CODE_BLOCK_RE.captures(output) {
            if let Some(command) = captures.get(1) {
                return command.as_str().trim().to_string();
            }
        }

        // If no code blocks, clean up the raw output
        output
            .lines()
            .next()
            .unwrap_or(output)
            .trim()
            .trim_matches('`')
            .to_string()
    }
}