use crate::state::SettingsState;
use camino::Utf8PathBuf;
use league_toolkit::wad::Wad;
use ltk_swapper::remap_path;
use serde::Serialize;
use std::fs::File;
use std::path::{Path, PathBuf};
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

#[derive(Debug, Serialize, Clone)]
pub struct SkinInfo {
    id: u32,
    name: String,
    path: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct SwapResult {
    success: bool,
    mod_path: Option<String>,
    file_count: usize,
    error: Option<String>,
}

/// Helper to get the extracted skins directory using Settings or AppData fallback.
fn get_extracted_skins_dir(
    app: &AppHandle,
    state: &State<'_, SettingsState>,
) -> Result<PathBuf, String> {
    let settings = state
        .0
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;

    if let Some(path) = &settings.extracted_skins_path {
        Ok(path.clone())
    } else {
        app.path()
            .app_data_dir()
            .map(|p| p.join("extracted_skins"))
            .map_err(|e| format!("Failed to resolve AppData dir: {}", e))
    }
}

/// List available extracted skins for a champion.
#[tauri::command]
pub async fn get_extracted_skins(
    app: AppHandle,
    champion: String,
    state: State<'_, SettingsState>,
) -> Result<Vec<SkinInfo>, String> {
    let base_dir = get_extracted_skins_dir(&app, &state)?;
    let champion_dir = base_dir
        .join(&champion)
        .join("assets")
        .join("characters")
        .join(&champion.to_lowercase())
        .join("skins");

    if !champion_dir.exists() {
        return Ok(vec![]);
    }

    let mut skins = Vec::new();

    // Iterate over directories in .../skins/
    if let Ok(entries) = std::fs::read_dir(champion_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    // Identify SkinXX folders
                    if name.to_lowercase().starts_with("skin") {
                        if let Ok(id) = name[4..].parse::<u32>() {
                            // Only list non-base skins (assuming skin00 is base) or maybe list all?
                            // Usually users want to swap FROM a skin TO base.
                            // Let's include all found skins.
                            skins.push(SkinInfo {
                                id,
                                name: format!("Skin {:02}", id),
                                path: entry.path().to_string_lossy().to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Sort by ID
    skins.sort_by_key(|s| s.id);

    Ok(skins)
}

/// Prepare swap: Create a mod that swaps `from_skin` to `to_skin` (usually 0/Base).
#[tauri::command]
pub async fn prepare_swap(
    app: AppHandle,
    champion: String,
    from_skin: u32,
    to_skin: u32,
    state: State<'_, SettingsState>,
) -> Result<SwapResult, String> {
    // 1. Resolve paths
    let extracted_dir = get_extracted_skins_dir(&app, &state)?;

    // Configured mod storage path or fallback
    let mod_storage_path = {
        let settings = state
            .0
            .lock()
            .map_err(|e| format!("Failed to lock settings: {}", e))?;
        if let Some(path) = &settings.mod_storage_path {
            path.clone()
        } else {
            app.path()
                .app_data_dir()
                .map(|p| p.join("installed_mods"))
                .map_err(|e| format!("Failed to resolve AppData dir: {}", e))?
        }
    };

    // 2. Heavy lifting in blocking task
    let result = tokio::task::spawn_blocking(move || {
        swap_blocking(
            &champion,
            from_skin,
            to_skin,
            &extracted_dir,
            &mod_storage_path,
        )
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    Ok(result)
}

fn swap_blocking(
    champion: &str,
    from_skin: u32,
    to_skin: u32,
    extracted_root: &Path,
    mod_root: &Path,
) -> SwapResult {
    // Source: .../extracted_skins/{Champion}
    let src_root = extracted_root.join(champion);
    // Target: .../installed_mods/{Champion}_Swap_{From}_to_{To}
    let mod_name = format!(
        "{}_Swap_Review_{:02}_to_{:02}",
        champion, from_skin, to_skin
    );
    let target_root = mod_root.join(&mod_name);

    if !src_root.exists() {
        return SwapResult {
            success: false,
            mod_path: None,
            file_count: 0,
            error: Some(format!("Source not found: {:?}", src_root)),
        };
    }

    // Clean target if exists
    if target_root.exists() {
        let _ = std::fs::remove_dir_all(&target_root);
    }
    if let Err(e) = std::fs::create_dir_all(&target_root) {
        return SwapResult {
            success: false,
            mod_path: None,
            file_count: 0,
            error: Some(format!("Failed to create mod dir: {}", e)),
        };
    }

    let mut file_count = 0;

    // Recursive traversal of source
    let walker = walkdir::WalkDir::new(&src_root);

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let path = entry.path();

            // Check if file belongs to Source Skin
            // We can rely on `remap_path` to decide if it needs remapping
            // But we should optimize: only process files that actally contain the Skin ID pattern in their path
            let relative_path = match path.strip_prefix(&src_root) {
                Ok(p) => p.to_string_lossy().to_string(), // Keep original separators? remap handles slash conversion
                Err(_) => continue,
            };

            // Try remapping
            if let Some(new_relative_path) =
                remap_path(champion, from_skin, to_skin, &relative_path)
            {
                // Determine destination path
                let dest_path = target_root.join(PathBuf::from(new_relative_path));

                // Create parent dir
                if let Some(parent) = dest_path.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        return SwapResult {
                            success: false,
                            mod_path: None,
                            file_count,
                            error: Some(format!(
                                "Failed to create parent dir for {:?}: {}",
                                dest_path, e
                            )),
                        };
                    }
                }

                // Copy file
                if let Err(e) = std::fs::copy(path, &dest_path) {
                    return SwapResult {
                        success: false,
                        mod_path: None,
                        file_count,
                        error: Some(format!("Failed to copy file {:?}: {}", path, e)),
                    };
                }

                file_count += 1;
            }
        }
    }

    SwapResult {
        success: true,
        mod_path: Some(target_root.to_string_lossy().to_string()),
        file_count,
        error: None,
    }
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
