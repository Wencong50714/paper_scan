use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIResponse {
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub message: Message,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub struct LLMClient {
    config: LLMConfig,
    client: reqwest::Client,
}

#[derive(Debug)]
pub struct LLMConfig {
    base_url: String,
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: Option<u32>,
}

impl LLMConfig {
    /// 从环境变量加载配置，一次性完成
    pub fn load() -> Result<Self> {
        dotenv::dotenv().ok();

        let base_url = env::var("BASE_URL").unwrap_or_else(|_| {
            "https://generativelanguage.googleapis.com/v1beta/openai".to_string()
        });

        let api_key = env::var("API_KEY").context("必须在 .env 文件或环境中设置 API_KEY")?;

        let model = env::var("MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string());

        let temperature = env::var("TEMPERATURE")
            .ok()
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.7);

        let max_tokens = env::var("MAX_TOKENS")
            .ok()
            .and_then(|s| s.parse::<u32>().ok());

        Ok(Self {
            base_url,
            api_key,
            model,
            temperature,
            max_tokens,
        })
    }
}

impl LLMClient {
    pub fn new() -> Result<Self> {
        let config = LLMConfig::load()?;
        Ok(Self {
            config,
            client: reqwest::Client::new(),
        })
    }

    pub async fn generate_note(&self, prompt: &str, paper_content: &str) -> Result<String> {
        let request_body = OpenAIRequest {
            model: self.config.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: prompt.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: paper_content.to_string(),
                },
            ],
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
        };

        println!("{:#?}", self.config);

        let response = self
            .client
            .post(format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API request failed: {}", error_text));
        }

        let response_data: OpenAIResponse = response.json().await?;

        if let Some(choice) = response_data.choices.first() {
            Ok(choice.message.content.trim().to_string())
        } else {
            Err(anyhow::anyhow!("No response from API"))
        }
    }

    pub async fn generate_note_with_images(
        &self,
        prompt: &str,
        paper_content: &str,
        image_references: &[String],
    ) -> Result<String> {
        let mut full_content = paper_content.to_string();

        if !image_references.is_empty() {
            full_content.push_str("\n\n图像文件列表:\n");
            for (i, img) in image_references.iter().enumerate() {
                full_content.push_str(&format!("- 图像 {}: {}\n", i + 1, img));
            }
        }

        self.generate_note(prompt, &full_content).await
    }
}

impl Default for LLMClient {
    fn default() -> Self {
        Self::new().expect("Failed to create LLM client")
    }
}
