use crate::state::SettingsState;
use camino::Utf8PathBuf;
use league_toolkit::wad::Wad;
use serde::Serialize;
use std::fs::File;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};
use wadtools::{
    commands::{download_hashes, DownloadHashesArgs},
    extractor::Extractor,
    utils::{default_hashtable_dir, WadHashtable},
};

#[derive(Debug, Serialize, Clone)]
pub struct ExtractResult {
    success: bool,
    path: Option<String>,
    error: Option<String>,
    files_count: Option<usize>,
}

/// Extract base skin WAD for a champion.
/// Uses spawn_blocking to avoid freezing the UI during heavy I/O.
#[tauri::command]
pub async fn extract_base_skin(
    app: AppHandle,
    champion: String,
    state: State<'_, SettingsState>,
) -> Result<ExtractResult, String> {
    // 1. Get configuration (quick, on main thread)
    let (league_path, output_base_dir): (PathBuf, PathBuf) = {
        let settings_guard = state
            .0
            .lock()
            .map_err(|e| format!("Failed to lock settings: {}", e))?;

        let league = match &settings_guard.league_path {
            Some(p) => p.clone(),
            None => {
                return Ok(ExtractResult {
                    success: false,
                    path: None,
                    error: Some("League path not configured via Settings".to_string()),
                    files_count: None,
                })
            }
        };

        // Determine output directory: User Configured -> AppData (Default)
        // Default: C:\Users\{User}\AppData\Roaming\dev.leaguetoolkit.manager\extracted_skins
        let output = if let Some(path) = &settings_guard.extracted_skins_path {
            path.clone()
        } else {
            match app.path().app_data_dir() {
                Ok(app_data) => app_data.join("extracted_skins"),
                Err(e) => {
                    return Ok(ExtractResult {
                        success: false,
                        path: None,
                        error: Some(format!("Failed to resolve AppData dir: {}", e)),
                        files_count: None,
                    })
                }
            }
        };

        (league, output)
    };

    // 2. Move heavy work to blocking thread pool
    let result = tokio::task::spawn_blocking(move || {
        extract_blocking(&champion, &league_path, &output_base_dir)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    Ok(result)
}

/// The actual extraction logic, runs on a blocking thread.
fn extract_blocking(
    champion: &str,
    league_path: &PathBuf,
    output_base_dir: &PathBuf,
) -> ExtractResult {
    // Locate the WAD file
    let wad_path = league_path
        .join("Game")
        .join("DATA")
        .join("FINAL")
        .join("Champions")
        .join(format!("{}.wad.client", champion));

    if !wad_path.exists() {
        return ExtractResult {
            success: false,
            error: Some(format!("WAD file not found at: {:?}", wad_path)),
            path: None,
            files_count: None,
        };
    }

    // Load Hash Table
    let hashtable_dir = default_hashtable_dir().unwrap_or_else(|| Utf8PathBuf::from("wadtools"));
    let hashtable_path = hashtable_dir.join("hashes.game.txt");

    // Auto-download hashes if missing
    if !hashtable_path.as_std_path().exists() {
        let args = DownloadHashesArgs {
            hashtable_dir: Some(hashtable_dir.to_string()),
        };

        if let Err(e) = download_hashes(args) {
            return ExtractResult {
                success: false,
                error: Some(format!("Failed to auto-download hashes: {}", e)),
                path: None,
                files_count: None,
            };
        }
    }

    // Load hashtable
    let mut hashtable = match WadHashtable::new() {
        Ok(h) => h,
        Err(e) => {
            return ExtractResult {
                success: false,
                error: Some(format!("Failed to init hashtable: {}", e)),
                path: None,
                files_count: None,
            }
        }
    };

    if let Err(e) = hashtable.add_from_dir(&hashtable_dir) {
        return ExtractResult {
            success: false,
            error: Some(format!("Failed to load hashtable: {}", e)),
            path: None,
            files_count: None,
        };
    }

    // Open WAD file
    let source = match File::open(&wad_path) {
        Ok(f) => f,
        Err(e) => {
            return ExtractResult {
                success: false,
                error: Some(format!("Failed to open WAD: {}", e)),
                path: None,
                files_count: None,
            }
        }
    };

    let mut wad = match Wad::mount(&source) {
        Ok(w) => w,
        Err(e) => {
            return ExtractResult {
                success: false,
                error: Some(format!("Failed to mount WAD: {}", e)),
                path: None,
                files_count: None,
            }
        }
    };

    let (mut decoder, chunks) = wad.decode();

    // Extract to destination
    // Output structure: {output_base_dir}/{Champion}
    let output_dir = output_base_dir.join(champion);

    // Create output directory
    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        return ExtractResult {
            success: false,
            error: Some(format!("Failed to create output dir: {}", e)),
            path: None,
            files_count: None,
        };
    }

    // Convert to Utf8Path for wadtools
    let output_dir_utf8 = match Utf8PathBuf::from_path_buf(output_dir.clone()) {
        Ok(p) => p,
        Err(_) => {
            return ExtractResult {
                success: false,
                error: Some("Output path contains non-UTF8 characters".to_string()),
                path: None,
                files_count: None,
            }
        }
    };

    // Create extractor and extract
    let mut extractor = Extractor::new(&mut decoder, &hashtable);

    match extractor.extract_chunks(chunks, &output_dir_utf8, None) {
        Ok(count) => ExtractResult {
            success: true,
            path: Some(output_dir.to_string_lossy().to_string()),
            files_count: Some(count),
            error: None,
        },
        Err(e) => ExtractResult {
            success: false,
            error: Some(format!("Extraction failed: {}", e)),
            path: None,
            files_count: None,
        },
    }
}
