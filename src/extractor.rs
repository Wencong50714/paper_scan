use anyhow::Result;
use flate2::read::GzDecoder;
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::Archive;
use walkdir::WalkDir;
use zip::ZipArchive;

use crate::downloader::PaperData;

#[derive(Debug)]
#[allow(dead_code)]
pub struct ExtractedContent {
    pub tex_files: Vec<PathBuf>,
    pub bib_files: Vec<PathBuf>,
    pub image_files: Vec<PathBuf>,
    pub main_tex_file: Option<PathBuf>,
    pub extracted_dir: PathBuf,
}

pub struct ArchiveExtractor;

impl ArchiveExtractor {
    pub fn new() -> Self {
        Self
    }

    pub fn extract(&self, paper_data: PaperData) -> Result<ExtractedContent> {
        let archive_path = paper_data.archive_path;
        let extract_dir = paper_data.output_dir.join("extracted");
        std::fs::create_dir_all(&extract_dir)?;

        // Determine archive type and extract accordingly
        if archive_path.extension().and_then(|s| s.to_str()) == Some("gz") {
            self.extract_tar_gz(&archive_path, &extract_dir)?;
        } else if archive_path.extension().and_then(|s| s.to_str()) == Some("zip") {
            self.extract_zip(&archive_path, &extract_dir)?;
        } else {
            return Err(anyhow::anyhow!("Unsupported archive format"));
        }

        // Scan extracted directory for files
        self.scan_extracted_files(&extract_dir)
    }

    fn extract_tar_gz(&self, archive_path: &Path, extract_dir: &Path) -> Result<()> {
        let file = File::open(archive_path)?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        archive.unpack(extract_dir)?;

        println!("Extracted tar.gz archive to {}", extract_dir.display());
        Ok(())
    }

    fn extract_zip(&self, archive_path: &Path, extract_dir: &Path) -> Result<()> {
        let file = File::open(archive_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = extract_dir.join(file.name());

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(p)?;
                    }
                }
                let mut outfile = File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        println!("Extracted zip archive to {}", extract_dir.display());
        Ok(())
    }

    fn scan_extracted_files(&self, extract_dir: &Path) -> Result<ExtractedContent> {
        let mut tex_files = Vec::new();
        let mut bib_files = Vec::new();
        let mut image_files = Vec::new();

        println!("Scanning extracted directory: {}", extract_dir.display());

        // Check if directory exists and list contents
        if !extract_dir.exists() {
            return Err(anyhow::anyhow!(
                "Extracted directory does not exist: {}",
                extract_dir.display()
            ));
        }

        // List directory contents for debugging
        if let Ok(entries) = std::fs::read_dir(extract_dir) {
            for entry in entries.flatten() {
                println!("Found: {}", entry.path().display());
            }
        }

        for entry in WalkDir::new(extract_dir) {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                match path.extension().and_then(|s| s.to_str()) {
                    Some("tex") => {
                        println!("Found TeX file: {}", path.display());
                        tex_files.push(path.to_path_buf());
                    }
                    Some("bib") => {
                        println!("Found BibTeX file: {}", path.display());
                        bib_files.push(path.to_path_buf());
                    }
                    Some(ext)
                        if ["png", "jpg", "jpeg", "pdf", "eps", "gif"]
                            .contains(&ext.to_lowercase().as_str()) =>
                    {
                        println!("Found image file: {}", path.display());
                        image_files.push(path.to_path_buf());
                    }
                    _ => continue,
                }
            }
        }

        // Find main TeX file (usually the one with \documentclass)
        let main_tex_file = self.find_main_tex_file(&tex_files)?;

        Ok(ExtractedContent {
            tex_files,
            bib_files,
            image_files,
            main_tex_file,
            extracted_dir: extract_dir.to_path_buf(),
        })
    }

    fn find_main_tex_file(&self, tex_files: &[PathBuf]) -> Result<Option<PathBuf>> {
        if tex_files.is_empty() {
            return Ok(None);
        }

        // Look for documentclass in each file
        for tex_file in tex_files {
            if let Ok(content) = std::fs::read_to_string(tex_file) {
                if content.contains(r"\documentclass") || content.contains(r"\documentstyle") {
                    return Ok(Some(tex_file.clone()));
                }
            }
        }

        // Look for files with common main file names
        let main_names = [
            "main.tex",
            "paper.tex",
            "article.tex",
            "ms.tex",
            "template.tex",
        ];
        for tex_file in tex_files {
            if let Some(file_name) = tex_file.file_name().and_then(|n| n.to_str()) {
                if main_names
                    .iter()
                    .any(|name| file_name.eq_ignore_ascii_case(name))
                {
                    return Ok(Some(tex_file.clone()));
                }
            }
        }

        // If no documentclass found, return None to indicate no main file
        // The processor will handle reading all files
        Ok(None)
    }
}

impl Default for ArchiveExtractor {
    fn default() -> Self {
        Self::new()
    }
}
