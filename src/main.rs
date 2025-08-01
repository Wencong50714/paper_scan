use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::Path;
use futures::future::join_all;

mod arxiv;
mod downloader;
mod extractor;
mod processor;
mod llm_client;
mod note_generator;

use arxiv::ArxivUrl;
use downloader::PaperDownloader;
use processor::PaperProcessor;
use note_generator::NoteGenerator;

#[derive(Parser)]
#[command(name = "paper_scan")]
#[command(about = "arXiv paper automation note generator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process a single arXiv paper URL
    Single {
        /// arXiv paper URL
        url: String,
    },
    /// Process multiple arXiv paper URLs from a file
    Batch {
        /// Path to file containing URLs (one per line)
        file_path: String,
    },
    /// Collect PDF files from tex folder to pdfs folder
    CollectPdf {
        /// Optional source directory (defaults to "tex")
        #[arg(short, long)]
        source: Option<String>,
        /// Optional destination directory (defaults to "pdfs")
        #[arg(short, long)]
        destination: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Single { url } => {
            process_single_paper(&url).await?;
        }
        Commands::Batch { file_path } => {
            process_batch_papers(&file_path).await?;
        }
        Commands::CollectPdf { source, destination } => {
            collect_pdf_files(source, destination).await?;
        }
    }

    Ok(())
}

async fn process_single_paper(url: &str) -> Result<()> {
    println!("Processing single paper: {}", url);
    
    let arxiv_url = ArxivUrl::parse(url)?;
    let paper_id = arxiv_url.paper_id().to_string();
    
    // Check if tex file already exists
    let tex_path = Path::new("tex").join(format!("{}", paper_id));
    if tex_path.exists() {
        println!("[Exist]: generated note existed, skip.");
        return Ok(());
    }
    
    let downloader = PaperDownloader::new();
    let processor = PaperProcessor::new();
    
    let paper_data = downloader.download(&arxiv_url).await?;
    let processed_content = processor.process(paper_data).await?;

    let note_generator = NoteGenerator::new()?;
    let generated_note = note_generator.generate_note(&processed_content).await?;
    
    // Save the generated note
    let output_dir = Path::new("tex").join(format!("{}", paper_id));
    std::fs::create_dir_all(&output_dir)?;
    let output_filename = format!("{}.tex", processed_content.paper_id);
    let output_path = output_dir.join(output_filename);
    note_generator.save_note(&generated_note, &output_path).await?;

    println!("Successfully processed paper: {}", processed_content.title);
    println!("Generated note saved to: {}", output_path.display());
    
    Ok(())
}

async fn process_batch_papers(file_path: &str) -> Result<()> {
    println!("Processing batch papers from: {}", file_path);
    
    let content = std::fs::read_to_string(file_path)?;
    let urls: Vec<String> = content.lines().filter(|line| !line.trim().is_empty()).map(|s| s.to_string()).collect();
    
    let mut tasks = vec![];
    for url in urls {
        tasks.push(tokio::spawn(async move {
            if let Err(e) = process_single_paper(&url).await {
                eprintln!("Error processing {}: {}", url, e);
            }
        }));
    }
    
    join_all(tasks).await;
    
    Ok(())
}

async fn collect_pdf_files(source: Option<String>, destination: Option<String>) -> Result<()> {
    let source_dir = source.unwrap_or_else(|| "tex".to_string());
    let dest_dir = destination.unwrap_or_else(|| "pdfs".to_string());
    
    println!("Collecting PDF files from '{}' to '{}'", source_dir, dest_dir);
    
    // Create destination directory if it doesn't exist
    if !Path::new(&dest_dir).exists() {
        std::fs::create_dir_all(&dest_dir)?;
        println!("Created destination directory: {}", dest_dir);
    }
    
    let source_path = Path::new(&source_dir);
    if !source_path.exists() {
        anyhow::bail!("Source directory '{}' does not exist", source_dir);
    }
    
    let mut pdf_count = 0;
    
    // Walk through all subdirectories in source
    for entry in std::fs::read_dir(source_path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let dir_path = entry.path();
            
            // Look for PDF files in this directory
            for file in std::fs::read_dir(&dir_path)? {
                let file = file?;
                let file_path = file.path();
                
                if file.file_type()?.is_file() {
                    if let Some(extension) = file_path.extension() {
                        if extension.to_string_lossy().to_lowercase() == "pdf" {
                            let file_name = file.file_name();
                            let dest_path = Path::new(&dest_dir).join(&file_name);
                            
                            // Copy the PDF file
                            std::fs::copy(&file_path, &dest_path)?;
                            println!("Copied: {} -> {}", file_path.display(), dest_path.display());
                            pdf_count += 1;
                        }
                    }
                }
            }
        }
    }
    
    if pdf_count == 0 {
        println!("No PDF files found in '{}' directory", source_dir);
    } else {
        println!("Successfully collected {} PDF file(s)", pdf_count);
    }
    
    Ok(())
}