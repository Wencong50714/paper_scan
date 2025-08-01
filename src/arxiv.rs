use anyhow::{anyhow, Result};
use regex::Regex;
use url::Url;

#[derive(Debug, Clone)]
pub struct ArxivUrl {
    pub paper_id: String,
    pub src_url: String,
}

impl ArxivUrl {
    pub fn parse(url: &str) -> Result<Self> {
        let url = Url::parse(url)?;

        // Extract paper ID from URL
        let paper_id = extract_paper_id(url.as_str())?;

        Ok(ArxivUrl {
            paper_id: paper_id.clone(),
            src_url: format!("https://arxiv.org/src/{paper_id}"),
        })
    }

    pub fn paper_id(&self) -> &str {
        &self.paper_id
    }
}

fn extract_paper_id(url: &str) -> Result<String> {
    let re = Regex::new(r"arxiv\.org/(?:abs|pdf)/([0-9]+\.?[0-9]+(?:v[0-9]+)?)")?;

    if let Some(captures) = re.captures(url) {
        if let Some(id_match) = captures.get(1) {
            return Ok(id_match.as_str().to_string());
        }
    }

    // Try another pattern for old-style arXiv IDs
    let old_re = Regex::new(r"arxiv\.org/(?:abs|pdf)/([a-z-]+/[0-9]+(?:v[0-9]+)?)")?;
    if let Some(captures) = old_re.captures(url) {
        if let Some(id_match) = captures.get(1) {
            return Ok(id_match.as_str().to_string());
        }
    }

    Err(anyhow!("Invalid arXiv URL format: {}", url))
}
