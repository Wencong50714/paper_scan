use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::llm_client::LLMClient;
use crate::processor::ProcessedContent;

pub struct NoteGenerator {
    client: LLMClient,
    system_prompt: String,
}

#[derive(Debug, serde::Serialize)]
pub struct GeneratedNote {
    pub paper_id: String,
    pub title: String,
    pub latex_content: String,
    pub metadata: NoteMetadata,
}

#[derive(Debug, serde::Serialize)]
pub struct NoteMetadata {
    pub generated_at: String,
    pub model_used: String,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

impl NoteGenerator {
    pub fn new() -> Result<Self> {
        let client = LLMClient::new()?;
        let system_prompt = load_system_prompt()?;
        
        Ok(Self {
            client,
            system_prompt,
        })
    }

    pub async fn generate_note(&self, 
        processed_content: &ProcessedContent
    ) -> Result<GeneratedNote> {
        let paper_summary = self.format_paper_content(processed_content);
        
        let generated_content = self.client
            .generate_note_with_images(
                &self.system_prompt,
                &paper_summary,
                &processed_content.image_files
            )
            .await?;

        // Post-process the generated content
        let processed_latex = self.post_process_latex(&generated_content);

        let note = GeneratedNote {
            paper_id: processed_content.paper_id.clone(),
            title: processed_content.title.clone(),
            latex_content: processed_latex,
            metadata: NoteMetadata {
                generated_at: chrono::Utc::now().to_rfc3339(),
                model_used: "gpt-3.5-turbo".to_string(),
                prompt_tokens: None,
                completion_tokens: None,
                total_tokens: None,
            },
        };

        Ok(note)
    }

    fn format_paper_content(&self, 
        processed_content: &ProcessedContent
    ) -> String {
        let mut content = String::new();
        
        content.push_str(&format!("论文标题: {}\n\n", processed_content.title));
        content.push_str(&format!("作者: {}\n\n", processed_content.authors.join(", ")));
        content.push_str(&format!("摘要:\n{}\n\n", processed_content.abstract_text));
        
        content.push_str("章节内容:\n");
        for section in &processed_content.sections {
            content.push_str(&format!("{} {}\n{}", 
                "#".repeat(section.level as usize), 
                section.title, 
                section.content
            ));
            content.push_str("\n\n");
        }

        if !processed_content.equations.is_empty() {
            content.push_str("重要公式:\n");
            for (i, eq) in processed_content.equations.iter().enumerate() {
                content.push_str(&format!("公式 {}: {}\n", i + 1, eq));
            }
            content.push_str("\n");
        }

        content
    }

    pub async fn save_note(&self, note: &GeneratedNote, output_path: &Path) -> Result<()> {
        fs::write(output_path, &note.latex_content)?;
        Ok(())
    }

    fn post_process_latex(&self, content: &str) -> String {
        let mut processed = content.to_string();
        
        // Remove first line and last line if they contain ```latex and ``` markers
        let lines: Vec<&str> = processed.lines().collect();
        if !lines.is_empty() {
            let mut start_idx = 0;
            let mut end_idx = lines.len();
            
            // Check if first line starts with ```latex or ```
            if lines[0].trim().starts_with("```") {
                start_idx = 1;
            }
            
            // Check if last line is ```
            if lines.len() > start_idx && lines[lines.len() - 1].trim() == "```" {
                end_idx = lines.len() - 1;
            }
            
            if start_idx > 0 || end_idx < lines.len() {
                processed = lines[start_idx..end_idx].join("\n");
            }
        }
        
        // Replace all occurrences of {output/ with {../../output/
        processed = processed.replace("{output/", "{../../output/");
        
        // Ensure the content doesn't start or end with extra newlines
        processed.trim().to_string()
    }
}

fn load_system_prompt() -> Result<String> {
    let prompt_path = "prompts.txt";
    let content = fs::read_to_string(prompt_path)?;
    Ok(content)
}


impl Default for NoteGenerator {
    fn default() -> Self {
        Self::new().expect("Failed to create NoteGenerator")
    }
}