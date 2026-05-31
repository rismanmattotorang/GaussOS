//! Utility functions

use anyhow::Result;
use std::path::Path;

/// Format file size for display
#[allow(dead_code)]
pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Get file extension
#[allow(dead_code)]
pub fn get_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
}

/// Check if file is a GaussTwin simulation file
#[allow(dead_code)]
pub fn is_simulation_file(path: &Path) -> bool {
    matches!(
        get_extension(path).as_deref(),
        Some("gausstwin") | Some("gts") | Some("json")
    )
}

/// Generate a unique file path (adds number suffix if exists)
#[allow(dead_code)]
pub fn get_unique_path(base_path: &Path) -> std::path::PathBuf {
    if !base_path.exists() {
        return base_path.to_path_buf();
    }

    let stem = base_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let extension = base_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let parent = base_path.parent().unwrap_or(Path::new("."));

    let mut counter = 1;
    loop {
        let new_name = if extension.is_empty() {
            format!("{} ({})", stem, counter)
        } else {
            format!("{} ({}).{}", stem, counter, extension)
        };

        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return new_path;
        }
        counter += 1;
    }
}

/// Sanitize filename
#[allow(dead_code)]
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Calculate file checksum (BLAKE3)
#[allow(dead_code)]
pub fn calculate_checksum(path: &Path) -> Result<String> {
    use std::fs::File;
    use std::io::{BufReader, Read};

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}
