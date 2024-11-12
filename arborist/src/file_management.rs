use anyhow::Result;
use walkdir::{DirEntry, WalkDir};

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

pub fn scan_dir(path: String) -> Result<()> {
    let mut file_count = 0;
    let mut folder_count = 0;

    for entry in WalkDir::new(path)
        .max_depth(10)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
    {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_dir() {
                    folder_count += 1;
                } else if entry.file_type().is_file() {
                    file_count += 1;
                }
            }
            Err(e) => eprintln!("Error reading entry: {:?}", e),
        }
    }

    println!("File count: {}, Folder count: {}", file_count, folder_count);

    Ok(())
}
