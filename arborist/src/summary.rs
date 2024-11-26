use crate::file_management::{FileMetadata, FileType, FolderMetadata};
use anyhow::{Context, Result};
use base64::Engine;
use calamine::{open_workbook, Reader, Xlsx};
use dotext::{pptx::Pptx, MsDoc};
use log::info;
use ollama_rs::{
    generation::{completion::request::GenerationRequest, images::Image},
    Ollama,
};
use pandoc::InputFormat;
use pdf_extract::extract_text;
use std::io::Read;
use std::process::Command;
use std::{fs::File, path::Path};
use tokio::fs::read;

pub async fn generate_file_summary(model: &str, file_metadata: &FileMetadata) -> Result<String> {
    info!("Processing: {}", file_metadata.path.clone());
    let content = match file_metadata.filetype {
        FileType::Document => read_document(file_metadata.path.clone()).await?,
        FileType::Image => return generate_image_summary(file_metadata.path.clone()).await,
        FileType::Audio => transcribe_audio(model, file_metadata.path.clone()).await?,
        FileType::Video => transcribe_video(model, file_metadata.path.clone()).await?,
        FileType::Archive => summarize_archive(file_metadata.path.clone()).await?,
        FileType::Other => "Summary not available for this file type.".to_string(),
    };

    let prompt = format!("Summarize the contents of file: {}", content);
    let system = "You are a helpful assistant who summarizes file contents.".to_string();

    let ollama = Ollama::default();
    let res = ollama
        .generate(GenerationRequest::new(model.to_string(), prompt).system(system))
        .await?;

    Ok(res.response)
}

pub async fn generate_folder_summary(
    model: &str,
    folder_metadata: &FolderMetadata,
) -> Result<String> {
    let mut folder_content = String::new();
    // Summarize each file in the folder and aggregate the summaries
    for file in &folder_metadata.files {
        let file_summary = generate_file_summary(model, file).await?;
        folder_content.push_str(&file_summary);
        folder_content.push('\n');
    }

    let prompt = format!("Summarize the contents of folder: {}", folder_content);
    let system = "You are a helpful assistant who summarizes folder contents.".to_string();

    let ollama = Ollama::default();
    let res = ollama
        .generate(GenerationRequest::new(model.to_string(), prompt).system(system))
        .await?;

    Ok(res.response)
}

fn detect_input_format(file_path: &str) -> InputFormat {
    if let Some(extension) = std::path::Path::new(file_path).extension() {
        match extension.to_str().unwrap_or("").to_lowercase().as_str() {
            // Add PDF as a special case
            "pdf" => InputFormat::Other("pdf".to_string()),
            "xlsx" => InputFormat::Other("xlsx".to_string()),
            "pptx" => InputFormat::Other("pptx".to_string()),
            // Well-supported formats with dedicated enum variants
            "md" | "markdown" => InputFormat::Markdown,
            "docx" => InputFormat::Docx,
            "epub" => InputFormat::Epub,
            "html" | "htm" => InputFormat::Html,
            "rtf" => InputFormat::Rtf,
            "tex" => InputFormat::Latex,
            "json" => InputFormat::Json,
            "rst" => InputFormat::Rst,
            "opml" => InputFormat::Opml,
            "org" => InputFormat::Org,
            "wiki" | "mediawiki" => InputFormat::MediaWiki,

            other => InputFormat::Other(other.to_string()),
        }
    } else {
        InputFormat::Other("plain".to_string())
    }
}

// Helper function to read plain text files
fn read_plain_text(file_path: &str) -> Result<String> {
    let mut file =
        File::open(file_path).with_context(|| format!("Failed to open file: {}", file_path))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .with_context(|| format!("Failed to read file: {}", file_path))?;
    Ok(content)
}

fn read_xlsx(file_path: &str) -> Result<String> {
    let mut workbook: Xlsx<_> = open_workbook(file_path)
        .with_context(|| format!("Failed to open Excel file: {}", file_path))?;

    let mut content = String::new();

    // Process each sheet in the workbook
    for sheet_name in workbook.sheet_names() {
        match workbook.worksheet_range(&sheet_name) {
            Ok(range) => {
                content.push_str(&format!("\nSheet: {}\n", sheet_name));

                // Convert cells to text, handling different data types
                for row in range.rows() {
                    let row_content: Vec<String> =
                        row.iter().map(|cell| cell.to_string()).collect();
                    content.push_str(&row_content.join("\t"));
                    content.push('\n');
                }
            }
            Err(e) => {
                // Handle worksheet access error
                return Err(anyhow::anyhow!(
                    "Failed to access sheet {}: {}",
                    sheet_name,
                    e
                ));
            }
        }
    }

    if content.is_empty() {
        return Err(anyhow::anyhow!("No readable content found in Excel file"));
    }

    Ok(content)
}

// Helper function to read PPTX files
fn read_pptx(file_path: &str) -> Result<String> {
    // Open the PPTX file using the `open` method
    let mut pptx = Pptx::open(Path::new(file_path))
        .with_context(|| format!("Failed to open PowerPoint file: {}", file_path))?;

    // Buffer to store the extracted text
    let mut content = String::new();

    // Extract text from the slides
    let mut buffer = Vec::new();
    pptx.read_to_end(&mut buffer)
        .with_context(|| "Failed to read PPTX content")?;
    let text =
        String::from_utf8(buffer).with_context(|| "Failed to convert PPTX content to UTF-8")?;

    content.push_str(&text);

    Ok(content)
}

// Helper function to read PDF files
fn read_pdf(file_path: &str) -> Result<String> {
    extract_text(file_path)
        .with_context(|| format!("Failed to extract text from PDF: {}", file_path))
}

// Updated read_document function with exhaustive pattern matching
pub async fn read_document(file_path: String) -> Result<String> {
    info!("Processing Document: {file_path}");
    let input_format = detect_input_format(&file_path);

    match input_format {
        // Handle PDF files separately
        InputFormat::Other(format) if format == "pdf" => read_pdf(&file_path),
        InputFormat::Other(format) if format == "xlsx" => read_xlsx(&file_path),
        InputFormat::Other(format) if format == "pptx" => read_pptx(&file_path),
        // For supported formats, use pandoc
        InputFormat::Markdown
        | InputFormat::Docx
        | InputFormat::Epub
        | InputFormat::Html
        | InputFormat::Rtf
        | InputFormat::Latex
        | InputFormat::Json
        | InputFormat::Rst
        | InputFormat::Opml
        | InputFormat::Org
        | InputFormat::MediaWiki => {
            let output = Command::new("pandoc")
                .arg("-f")
                .arg(input_format.to_string())
                .arg("-t")
                .arg("plain")
                .arg(&file_path)
                .output()
                .with_context(|| "Failed to execute pandoc command")?;

            if output.status.success() {
                String::from_utf8(output.stdout)
                    .with_context(|| "Failed to convert pandoc output to UTF-8")
            } else {
                Err(anyhow::anyhow!(
                    "Pandoc conversion failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ))
            }
        }
        // For other unsupported formats and future variants, fall back to plain text
        _ => read_plain_text(&file_path),
    }
}

pub async fn generate_image_summary(image_path: String) -> Result<String> {
    // Read the image file
    let bytes = read(&image_path).await?;

    // Encode the image bytes to base64
    let base64_image = base64::engine::general_purpose::STANDARD.encode(&bytes);

    // Create an Image from the base64 string
    let image = Image::from_base64(&base64_image);

    // Define the prompt
    let prompt = "Describe this image.";

    // Create a GenerationRequest with the model, prompt, and image
    let request =
        GenerationRequest::new("minicpm-v".to_string(), prompt.to_string()).add_image(image);

    // Send the request to Ollama
    let ollama = Ollama::default();
    let response = ollama.generate(request).await?;

    // Return the response
    Ok(response.response)
}

async fn transcribe_audio(_model: &str, file_path: String) -> Result<String> {
    // Placeholder implementation for transcribing audio
    Ok(format!("Audio transcription for: {}", file_path))
}

async fn transcribe_video(_model: &str, file_path: String) -> Result<String> {
    // Placeholder implementation for transcribing video
    Ok(format!("Video transcription for: {}", file_path))
}

async fn summarize_archive(file_path: String) -> Result<String> {
    // Placeholder implementation for summarizing archive
    Ok(format!("Archive summary for: {}", file_path))
}
