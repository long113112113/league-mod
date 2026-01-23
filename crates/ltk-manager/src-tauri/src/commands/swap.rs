use crate::state::SettingsState;
use camino::Utf8PathBuf;


use serde::Serialize;

use std::path::PathBuf;
use tauri::State;

use walkdir::WalkDir;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RemapResult {
    success: bool,
    processed_count: usize,
    output_path: Option<String>,
    error: Option<String>,
}

/// Extract skin WAD for a champion.
/// Uses spawn_blocking to avoid freezing the UI during heavy I/O.

/// The actual extraction logic, runs on a blocking thread.
/// The actual extraction logic, runs on a blocking thread.

/// Remap extracted skin files to Base skin.
#[tauri::command]
pub async fn remap_skin(
    app: tauri::AppHandle,
    champion: String,
    target_skin_id: u32,
    state: State<'_, SettingsState>,
) -> Result<RemapResult, String> {
    use tauri::Manager;

    let (input_dir, output_dir) = {
        let settings_guard = state
            .0
            .lock()
            .map_err(|e| format!("Failed to lock settings: {}", e))?;

        // Input: Extracted skins path / Champion
        let extracted_path = match &settings_guard.extracted_skins_path {
            Some(p) => p.join(&champion),
            None => match app.path().app_data_dir() {
                Ok(p) => p.join("extracted_skins").join(&champion),
                Err(e) => {
                    return Ok(RemapResult {
                        success: false,
                        processed_count: 0,
                        output_path: None,
                        error: Some(format!("Failed to determine app data dir: {}", e)),
                    })
                }
            },
        };

        // Output: Mod Storage / Champion_Swap
        let mod_storage = match &settings_guard.mod_storage_path {
            Some(p) => p.clone(),
            None => match app.path().app_data_dir() {
                Ok(p) => p.join("mods"),
                Err(e) => {
                    return Ok(RemapResult {
                        success: false,
                        processed_count: 0,
                        output_path: None,
                        error: Some(format!("Failed to determine mod storage dir: {}", e)),
                    })
                }
            },
        };

        (
            extracted_path,
            mod_storage.join(format!("{}_Swap", champion)),
        )
    };

    if !input_dir.exists() {
        return Ok(RemapResult {
            success: false,
            processed_count: 0,
            output_path: None,
            error: Some(format!("Input directory not found: {:?}", input_dir)),
        });
    }

    // Run remapping on blocking thread
    let result = tokio::task::spawn_blocking(move || {
        let engine = SwapEngine::new();
        let mut count = 0;

        for entry in WalkDir::new(&input_dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let path = entry.path();
                // Get relative path for remapping logic
                let relative_path = match path.strip_prefix(&input_dir) {
                    Ok(p) => p.to_string_lossy().replace('\\', "/"),
                    Err(_) => continue,
                };

                // Remap: Target Skin -> Base (0)
                if let Some(new_relative_path) =
                    engine.remap_path(&champion, target_skin_id, 0, &relative_path)
                {
                    let dest_path = output_dir.join(&new_relative_path);

                    if let Some(parent) = dest_path.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            return RemapResult {
                                success: false,
                                processed_count: count,
                                output_path: None,
                                error: Some(format!("Failed to create output dir: {}", e)),
                            };
                        }
                    }

                    if let Err(e) = std::fs::copy(path, &dest_path) {
                        return RemapResult {
                            success: false,
                            processed_count: count,
                            output_path: None,
                            error: Some(format!("Failed to copy file: {}", e)),
                        };
                    }

                    count += 1;
                }
            }
        }

        RemapResult {
            success: true,
            processed_count: count,
            output_path: Some(output_dir.to_string_lossy().to_string()),
            error: None,
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    Ok(result)
}
