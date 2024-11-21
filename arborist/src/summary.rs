use crate::file_management::{FileMetadata, FileType, FolderMetadata};
use anyhow::Result;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;
use pandoc::InputFormat;
use std::process::Command;

pub async fn generate_file_summary(model: &str, file_metadata: &FileMetadata) -> Result<String> {
    let content = match file_metadata.filetype {
        FileType::Document => read_document(file_metadata.path.clone()).await?,
        FileType::Image => generate_image_caption(model, file_metadata.path.clone()).await?,
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
        InputFormat::Other("plain".to_string()) // Default to plain for files without extensions
    }
}

// Helper function to read document content using pandoc
pub async fn read_document(file_path: String) -> Result<String> {
    let input_format = detect_input_format(&file_path);

    let output = Command::new("pandoc")
        .arg("-f")
        .arg(match input_format {
            // Convert InputFormat variants to strings for Pandoc CLI
            InputFormat::Other(format) => format,
            _ => input_format.to_string(),
        })
        .arg("-t")
        .arg("plain") // Convert to plain text
        .arg(&file_path)
        .output()?;

    println!("test");
    if output.status.success() {
        Ok(String::from_utf8(output.stdout)?)
    } else {
        Err(anyhow::anyhow!(
            "Failed to read document: {}",
            String::from_utf8(output.stderr)?
        ))
    }
}

async fn generate_image_caption(model: &str, file_path: String) -> Result<String> {
    // Placeholder implementation for generating image caption
    Ok(format!("Image caption for: {}", file_path))
}

async fn transcribe_audio(model: &str, file_path: String) -> Result<String> {
    // Placeholder implementation for transcribing audio
    Ok(format!("Audio transcription for: {}", file_path))
}

async fn transcribe_video(model: &str, file_path: String) -> Result<String> {
    // Placeholder implementation for transcribing video
    Ok(format!("Video transcription for: {}", file_path))
}

async fn summarize_archive(file_path: String) -> Result<String> {
    // Placeholder implementation for summarizing archive
    Ok(format!("Archive summary for: {}", file_path))
}
