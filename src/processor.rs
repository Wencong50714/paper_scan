use anyhow::Result;
use regex::Regex;
use std::fs;

use crate::downloader::PaperData;
use crate::extractor::{ArchiveExtractor, ExtractedContent};

#[derive(Debug, serde::Serialize)]
pub struct ProcessedContent {
    pub paper_id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub sections: Vec<Section>,
    pub figure_references: Vec<String>,
    pub equations: Vec<String>,
    pub full_text: String,
    pub image_files: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct Section {
    pub title: String,
    pub content: String,
    pub level: u8,
}

pub struct PaperProcessor {
    extractor: ArchiveExtractor,
}

impl PaperProcessor {
    pub fn new() -> Self {
        Self {
            extractor: ArchiveExtractor::new(),
        }
    }

    pub async fn process(&self, paper_data: PaperData) -> Result<ProcessedContent> {
        let archive_path = paper_data.archive_path.clone();

        // Extract archive contents
        let extracted = self.extractor.extract(paper_data)?;

        // Process the extracted content
        let result = self.process_extracted_content(extracted);

        // Clean up the downloaded archive after successful processing
        if result.is_ok() && archive_path.exists() {
            if let Err(e) = std::fs::remove_file(&archive_path) {
                eprintln!(
                    "Warning: Failed to remove downloaded archive {}: {}",
                    archive_path.display(),
                    e
                );
            } else {
                println!("Cleaned up downloaded archive: {}", archive_path.display());
            }
        }

        result
    }

    fn process_extracted_content(&self, extracted: ExtractedContent) -> Result<ProcessedContent> {
        let mut full_text = String::new();
        let mut title = String::new();
        let mut authors = Vec::new();
        let mut abstract_text = String::new();
        let mut sections = Vec::new();
        let mut figure_references = Vec::new();
        let mut equations = Vec::new();

        // Collect content from all TeX files
        let mut all_content = String::new();
        let mut files_read = 0;

        // First, try to read the main TeX file
        if let Some(main_tex) = &extracted.main_tex_file {
            if main_tex.exists() {
                println!("Reading main TeX file: {}", main_tex.display());
                if let Ok(content) = fs::read_to_string(main_tex) {
                    all_content.push_str(&content);
                    all_content.push_str("\n\n");
                    files_read += 1;
                }
            }
        }

        // Then read all other TeX files to get complete content
        for tex_file in &extracted.tex_files {
            if tex_file.exists() {
                println!("Reading TeX file: {}", tex_file.display());
                if let Ok(content) = fs::read_to_string(tex_file) {
                    all_content.push_str(&content);
                    all_content.push_str("\n\n");
                    files_read += 1;
                }
            }
        }

        if files_read > 0 {
            full_text = self.clean_tex_content(&all_content);

            // Extract metadata from combined content
            title = self.extract_title(&all_content);
            authors = self.extract_authors(&all_content);
            abstract_text = self.extract_abstract(&all_content);

            // Extract sections
            sections = self.extract_sections(&all_content);

            // Extract figures and equations
            figure_references = self.extract_figures(&all_content);
            equations = self.extract_equations(&all_content);

            println!("Successfully processed {files_read} TeX files");
        } else {
            eprintln!("No TeX files could be read for processing");
        }

        // Collect image file paths
        let image_files: Vec<String> = extracted
            .image_files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        // Extract paper ID from the output directory name
        let paper_id = extracted
            .extracted_dir
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        println!("Using paper ID: {paper_id}");

        Ok(ProcessedContent {
            paper_id,
            title,
            authors,
            abstract_text,
            sections,
            figure_references,
            equations,
            full_text,
            image_files,
        })
    }

    fn clean_tex_content(&self, content: &str) -> String {
        // Remove comments
        let re = Regex::new(r"(?m)%.*$").unwrap();
        let cleaned = re.replace_all(content, "");

        // Remove common LaTeX commands but preserve content within commands
        let re = Regex::new(r"\\(usepackage|documentclass|documentstyle|pagestyle|thispagestyle|geometry|hypersetup)\{[^}]*\}").unwrap();
        let cleaned = re.replace_all(&cleaned, "");

        // Remove begin and end but keep content
        let re = Regex::new(r"\\(begin|end)\{[^}]*\}").unwrap();
        let cleaned = re.replace_all(&cleaned, "");

        // Remove other common formatting commands but keep arguments
        let re = Regex::new(
            r"\\(textbf|textit|emph|texttt|small|large|Large|LARGE|huge|Huge)\{([^}]*)\}",
        )
        .unwrap();
        let cleaned = re.replace_all(&cleaned, "$2");

        // Clean up extra whitespace
        let cleaned = cleaned.replace("\n\n\n", "\n\n");
        let cleaned = cleaned.replace("  ", " ");
        let cleaned = Regex::new(r"\n\s*\n\s*\n")
            .unwrap()
            .replace_all(&cleaned, "\n\n");

        cleaned.trim().to_string()
    }

    fn extract_title(&self, content: &str) -> String {
        let re = Regex::new(r"\\title\{([^}]*)\}").unwrap();
        if let Some(caps) = re.captures(content) {
            let title = caps.get(1).map_or("", |m| m.as_str());
            // Clean up LaTeX formatting in title
            let cleaned = title.replace("\\", "");
            cleaned.trim().to_string()
        } else {
            "Untitled".to_string()
        }
    }

    fn extract_authors(&self, content: &str) -> Vec<String> {
        let re = Regex::new(r"\\author\{([^}]*)\}").unwrap();
        let mut authors = Vec::new();

        for caps in re.captures_iter(content) {
            if let Some(author) = caps.get(1) {
                // Split by commas and clean up
                let author_names: Vec<String> = author
                    .as_str()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                authors.extend(author_names);
            }
        }

        authors
    }

    fn extract_abstract(&self, content: &str) -> String {
        // Try different abstract patterns
        let patterns = [
            r"\\begin\{abstract\}(.*?)\\end\{abstract\}",
            r"\\abstract\{([^}]*)\}",
            r"\\section\*?\{abstract\}([^\\]*)",
        ];

        for pattern in &patterns {
            let re = Regex::new(pattern).unwrap();
            if let Some(caps) = re.captures(content) {
                let abstract_text = caps.get(1).map_or("", |m| m.as_str()).trim();
                if !abstract_text.is_empty() {
                    return self.clean_tex_content(abstract_text);
                }
            }
        }

        String::new()
    }

    fn extract_sections(&self, content: &str) -> Vec<Section> {
        let mut sections = Vec::new();

        // Extract sections and subsections
        let section_re = Regex::new(r"\\section\{([^}]*)\}").unwrap();
        let subsection_re = Regex::new(r"\\subsection\{([^}]*)\}").unwrap();

        // Find section boundaries
        let mut positions = Vec::new();

        for caps in section_re.captures_iter(content) {
            if let Some(m) = caps.get(0) {
                positions.push((m.start(), m.end(), caps.get(1).unwrap().as_str(), 1));
            }
        }

        for caps in subsection_re.captures_iter(content) {
            if let Some(m) = caps.get(0) {
                positions.push((m.start(), m.end(), caps.get(1).unwrap().as_str(), 2));
            }
        }

        // Sort by position
        positions.sort_by_key(|k| k.0);

        // Extract content between sections
        for i in 0..positions.len() {
            let (start, _, title, level) = positions[i];
            let end = if i + 1 < positions.len() {
                positions[i + 1].0
            } else {
                content.len()
            };

            let section_content = content[start..end].to_string();
            let cleaned_content = self.clean_tex_content(&section_content);

            sections.push(Section {
                title: title.to_string(),
                content: cleaned_content,
                level,
            });
        }

        sections
    }

    fn extract_figures(&self, content: &str) -> Vec<String> {
        let re = Regex::new(r"\\includegraphics(?:\[[^]]*\])?\{([^}]*)\}").unwrap();
        let mut figures = Vec::new();

        for caps in re.captures_iter(content) {
            if let Some(fig) = caps.get(1) {
                figures.push(fig.as_str().to_string());
            }
        }

        figures
    }

    fn extract_equations(&self, content: &str) -> Vec<String> {
        let mut equations = Vec::new();

        // Extract display equations
        let re = Regex::new(r"\\begin\{equation\}(.*?)\\end\{equation\}").unwrap();
        for caps in re.captures_iter(content) {
            if let Some(eq) = caps.get(1) {
                equations.push(eq.as_str().trim().to_string());
            }
        }

        // Extract inline math
        let re = Regex::new(r"\$([^$]+)\$").unwrap();
        for caps in re.captures_iter(content) {
            if let Some(eq) = caps.get(1) {
                equations.push(eq.as_str().trim().to_string());
            }
        }

        equations
    }
}

impl Default for PaperProcessor {
    fn default() -> Self {
        Self::new()
    }
}
