use anyhow::Result;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::arxiv::ArxivUrl;

#[derive(Debug)]
#[allow(dead_code)]
pub struct PaperData {
    pub paper_id: String,
    pub archive_path: PathBuf,
    pub output_dir: PathBuf,
}

impl PaperData {
    pub fn new(paper_id: String, archive_path: PathBuf, output_dir: PathBuf) -> Self {
        Self {
            paper_id,
            archive_path,
            output_dir,
        }
    }
}

pub struct PaperDownloader {
    client: reqwest::Client,
}

impl PaperDownloader {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }

    pub async fn download(&self, arxiv_url: &ArxivUrl) -> Result<PaperData> {
        let download_url = &arxiv_url.src_url;
        println!("Downloading from: {download_url}");

        // Create output directory for this paper
        let paper_id = arxiv_url.paper_id().to_string();
        let output_dir = Path::new("output").join(&paper_id);
        std::fs::create_dir_all(&output_dir)?;

        // Create archive file path in output directory
        let archive_path = output_dir.join(format!("{paper_id}.tar.gz"));

        // Download the file
        let response = self.client.get(download_url.as_str()).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download paper: HTTP {}",
                response.status()
            ));
        }

        // Save the downloaded content to file
        let bytes = response.bytes().await?;
        let mut file = File::create(&archive_path)?;
        file.write_all(&bytes)?;

        println!(
            "Downloaded {} bytes to {}",
            bytes.len(),
            archive_path.display()
        );

        Ok(PaperData::new(paper_id, archive_path, output_dir))
    }
}

impl Default for PaperDownloader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arxiv::ArxivUrl;

    #[tokio::test]
    async fn test_download_structure() {
        let arxiv_url = ArxivUrl::parse("https://arxiv.org/abs/2401.08027").unwrap();
        let downloader = PaperDownloader::new();

        // Verify the URL structure
        assert_eq!(arxiv_url.src_url, "https://arxiv.org/src/2401.08027.tar.gz");

        // Ensure the download was successful
        let result = downloader.download(&arxiv_url).await;
        assert!(result.is_ok(), "Download failed: {:?}", result.err());

        // Check that the archive file exists
        let paper_data = result.unwrap();
        assert!(
            paper_data.archive_path.exists(),
            "Archive file does not exist"
        );

        // Ensure the downloaded file is not empty
        let metadata = std::fs::metadata(&paper_data.archive_path).unwrap();
        assert!(metadata.len() > 0, "Downloaded file is empty");
    }
}
