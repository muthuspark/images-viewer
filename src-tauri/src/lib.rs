use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FolderEntry {
    name: String,
    path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryContents {
    folders: Vec<FolderEntry>,
    images: Vec<String>,
}

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp", "svg"];

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[tauri::command]
fn list_roots() -> Vec<FolderEntry> {
    #[cfg(target_os = "windows")]
    {
        return (b'A'..=b'Z')
            .map(|letter| format!("{}:\\", letter as char))
            .filter(|path| Path::new(path).exists())
            .map(|path| FolderEntry {
                name: path.clone(),
                path,
            })
            .collect();
    }

    #[cfg(not(target_os = "windows"))]
    {
        vec![FolderEntry {
            name: "/".to_string(),
            path: "/".to_string(),
        }]
    }
}

#[tauri::command]
fn list_directory(path: String) -> Result<DirectoryContents, String> {
    let directory = PathBuf::from(&path);
    if !directory.is_dir() {
        return Err("This folder is unavailable.".to_string());
    }

    let entries =
        fs::read_dir(&directory).map_err(|_| "This folder cannot be opened.".to_string())?;

    let mut folders = Vec::new();
    let mut images = Vec::new();

    for entry in entries.flatten() {
        let entry_path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            folders.push(FolderEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: display_path(&entry_path),
            });
            continue;
        }

        if file_type.is_file()
            && entry_path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    IMAGE_EXTENSIONS
                        .iter()
                        .any(|supported| extension.eq_ignore_ascii_case(supported))
                })
        {
            images.push(display_path(&entry_path));
        }
    }

    folders.sort_by_key(|folder| folder.name.to_lowercase());
    images.sort_by_key(|image| image.to_lowercase());

    Ok(DirectoryContents { folders, images })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_roots, list_directory])
        .run(tauri::generate_context!())
        .expect("error while running image viewer");
}
