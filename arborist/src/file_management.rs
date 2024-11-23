//use mime_guess::from_path;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::{path::Path, time::SystemTime};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FileType {
    Document,
    Image,
    Audio,
    Video,
    Archive,
    Other,
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileMetadata {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub filetype: FileType,
    #[serde_as(as = "serde_with::TimestampSeconds<i64>")]
    pub created_at: SystemTime,
    #[serde_as(as = "serde_with::TimestampSeconds<i64>")]
    pub modified_at: SystemTime,
    pub summary: String,
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FolderMetadata {
    pub name: String,
    pub path: String,
    pub size: u64,
    #[serde_as(as = "serde_with::TimestampSeconds<i64>")]
    pub created_at: SystemTime,
    #[serde_as(as = "serde_with::TimestampSeconds<i64>")]
    pub modified_at: SystemTime,
    pub file_count: u32,
    pub files: Vec<FileMetadata>,
    pub folder_count: u32,
    pub summary: String,
}

impl FileType {
    pub fn from_path(path: &str) -> FileType {
        let file_extension = match Path::new(path).extension() {
            Some(ext) => ext.to_string_lossy().to_lowercase(),
            None => return FileType::Other,
        };

        match file_extension.as_ref() {
            "epub" | "pdf" | "txt" | "docx" | "md" | "epage" | "rtf" | "fb2" | "azw3" | "mobi"
            | "doc" | "xlsx" | "csv" | "tex" | "bib" | "json" | "xml" | "html" | "conf"
            | "pptx" | "settings" | "prop" | "log" | "djvu" | "cls" | "pkt" | "sav" | "set"
            | "bin" | "backup" | "bundle" | "typ" | "scpt" | "ePub" | "PDF" | "DOCX" | "XLSX" => {
                FileType::Document
            }
            "jpg" | "png" | "jpeg" | "gif" | "bmp" | "tiff" | "webp" | "svg" | "heic" | "avif"
            | "pgm" | "opf" | "icon" => FileType::Image,
            "mp3" | "wav" | "m4b" | "ogg" | "flac" | "aac" | "wma" | "amr" => FileType::Audio,
            "mp4" | "mkv" | "webm" | "avi" | "mov" | "wmv" | "flv" | "mpeg" | "3gp" | "m4v" => {
                FileType::Video
            }
            "zip" | "tar" | "rar" | "7z" | "gz" | "bz2" | "iso" | "dmg" | "cab" | "jar" | "war"
            | "ear" | "pkg" | "deb" | "rpm" | "apk" | "cpio" => FileType::Archive,
            _ => FileType::Other,
        }
    }
}
