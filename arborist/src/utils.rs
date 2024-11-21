use crate::file_management::{FileMetadata, FileType, FolderMetadata};
use anyhow::Result;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;
use qdrant_client::qdrant::SearchPoints;
use qdrant_client::Qdrant;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::time::Instant;
use tokio::fs::metadata;
use walkdir::{DirEntry, WalkDir};

// Helper function to calculate folder size
async fn calculate_folder_size(path: &Path) -> Result<u64> {
    let mut total_size = 0;
    for entry in WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let meta = metadata(entry.path()).await?;
            total_size += meta.len();
        }
    }
    Ok(total_size)
}

// Helper function to count files in a folder
fn count_files_in_folder(path: &Path) -> u32 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count() as u32
}

// Helper function to count folders in a folder
fn count_folders_in_folder(path: &Path) -> u32 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
        .count() as u32
}

// DirScanConfig struct and its implementation
#[derive(Serialize, Deserialize, Debug)]
pub struct DirScanConfig {
    path: String,
    skip_hidden: bool,
    paths_to_skip: Option<Vec<String>>,
}

impl Default for DirScanConfig {
    fn default() -> Self {
        DirScanConfig {
            path: ".".to_string(),
            skip_hidden: true,
            paths_to_skip: Some(vec![
                "node_modules".to_string(),
                "downloaded-torrents".to_string(),
                "target".to_string(),
                "build".to_string(),
                "dist".to_string(),
                "downloaded-torrents".to_string(),
                ".git".to_string(),
            ]),
        }
    }
}

impl DirScanConfig {
    pub fn new(path: String) -> Self {
        DirScanConfig {
            path,
            ..Default::default()
        }
    }

    pub async fn scan_dir(&self) -> Result<DirScanResult> {
        let mut file_count = 0;
        let mut folder_count = 0;
        let mut extension_map: HashMap<String, usize> = HashMap::new();
        let mut file_list = Vec::new();
        let mut folder_list = Vec::new();
        let mut file_metadata_list = Vec::new();
        let mut folder_metadata_list = Vec::new();

        let start_time = Instant::now();

        for entry in WalkDir::new(&self.path)
            .max_depth(10)
            .into_iter()
            .filter_entry(|e| {
                (!self.skip_hidden || !is_hidden(e)) && !should_skip(e, &self.paths_to_skip)
            })
        {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_dir() {
                        folder_count += 1;
                        folder_list.push(entry.path().to_string_lossy().into_owned());

                        // Collect folder metadata
                        let meta = metadata(&entry.path()).await?;
                        let folder_size = calculate_folder_size(entry.path()).await?;
                        let created_at = meta.created()?;
                        let modified_at = meta.modified()?;
                        let file_count_folder = count_files_in_folder(entry.path());
                        let folder_count_folder = count_folders_in_folder(entry.path());

                        folder_metadata_list.push(FolderMetadata {
                            name: entry.file_name().to_string_lossy().into_owned(),
                            path: entry.path().to_string_lossy().into_owned(),
                            size: folder_size,
                            created_at,
                            modified_at,
                            file_count: file_count_folder,
                            files: file_metadata_list.clone(),
                            folder_count: folder_count_folder,
                            summary: String::new(), // To be filled later
                        });
                    } else if entry.file_type().is_file() {
                        file_count += 1;
                        file_list.push(entry.path().to_string_lossy().into_owned());

                        // Update extension map
                        if let Some(extension) = entry.path().extension() {
                            let extension_str = extension.to_string_lossy().to_string();
                            *extension_map.entry(extension_str).or_insert(0) += 1;
                        }

                        // Collect file metadata
                        let file_name = entry.file_name().to_string_lossy().into_owned();
                        let file_size = metadata(&entry.path()).await?.len();
                        let file_type = FileType::from_path(&entry.path().to_string_lossy());
                        let created_at = metadata(&entry.path()).await?.created()?;
                        let modified_at = metadata(&entry.path()).await?.modified()?;

                        file_metadata_list.push(FileMetadata {
                            name: file_name,
                            path: entry.path().to_string_lossy().into_owned(),
                            size: file_size,
                            filetype: file_type,
                            created_at,
                            modified_at,
                        });
                    }
                }
                Err(e) => eprintln!("error reading entry: {:?}", e),
            }
        }

        let elapsed_time = start_time.elapsed();

        // Sort extensions by count in descending order
        let mut extension_count: Vec<(String, usize)> = extension_map.into_iter().collect();
        extension_count.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(DirScanResult {
            file_count,
            folder_count,
            extension_count,
            elapsed_time,
            file_list,
            folder_list,
            file_metadata_list,
            folder_metadata_list,
        })
    }
}

// DirScanResult struct and its implementation
#[derive(Serialize, Deserialize, Debug)]
pub struct DirScanResult {
    pub file_count: u16,
    pub folder_count: u16,
    pub extension_count: Vec<(String, usize)>,
    pub elapsed_time: std::time::Duration,
    pub file_list: Vec<String>,
    pub folder_list: Vec<String>,
    pub file_metadata_list: Vec<FileMetadata>,
    pub folder_metadata_list: Vec<FolderMetadata>,
}

impl fmt::Display for DirScanResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Directory Scan Results:")?;
        writeln!(f, "-----------------------")?;
        writeln!(f, "Total Files: {}", self.file_count)?;
        writeln!(f, "Total Folders: {}", self.folder_count)?;
        writeln!(f, "File Extensions (sorted by count):")?;

        for (ext, count) in &self.extension_count {
            writeln!(f, "  - .{}: {}", ext, count)?;
        }

        writeln!(f, "Scan Took: {:?}", self.elapsed_time)?;
        writeln!(f, "Show file and folder lists? (y/n)")?;

        Ok(())
    }
}

// Function to generate folder summary using LLM
pub async fn gen_folder_summary(model: String, folder_metadata: &FolderMetadata) -> Result<String> {
    let prompt = format!("Summarize the contents of folder: {}", folder_metadata.path);
    let system = "You are a helpful assistant who summarizes folder contents.".to_string();

    let ollama = Ollama::default();
    let res = ollama
        .generate(GenerationRequest::new(model, prompt).system(system))
        .await?;

    Ok(res.response)
}

// Placeholder function for embedding generation
pub fn gen_embedding(_summary: &str) -> Vec<f32> {
    // Placeholder for embedding generation logic
    vec![0.0; 1536]
}

// Function to search summaries in Qdrant
pub async fn search_summaries(
    client: &Qdrant,
    query: &str,
) -> Result<Vec<qdrant_client::qdrant::ScoredPoint>> {
    let query_embedding = gen_embedding(query);

    let search_params = SearchPoints {
        collection_name: "summaries".to_string(),
        vector: query_embedding,
        limit: 5,
        with_payload: Some(true.into()),
        ..Default::default()
    };

    let search_result = client.search_points(search_params).await?;

    Ok(search_result.result)
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn should_skip(entry: &DirEntry, paths_to_skip: &Option<Vec<String>>) -> bool {
    if let Some(paths) = paths_to_skip {
        if let Some(name) = entry.file_name().to_str() {
            return paths.iter().any(|p| name == p);
        }
    }
    false
}
