use crate::error::{AppError, AppResult, IpcResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tokio::fs;

// ============================================================================
// INPUT STRUCTS (for deserializing raw metadata.json)
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawMetadata {
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub choose_vo_path: Option<String>,
    #[serde(default)]
    pub skins: Vec<RawSkin>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSkin {
    pub id: i32,
    #[serde(default)]
    pub is_base: bool,
    pub name: String,
    #[serde(default)]
    pub skin_classification: Option<String>,
    #[serde(default)]
    pub tile_path: Option<String>,
    #[serde(default)]
    pub rarity: Option<String>,
    #[serde(default)]
    pub chromas: Option<Vec<RawChroma>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawChroma {
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub tile_path: Option<String>,
}

// ============================================================================
// OUTPUT STRUCTS (for serializing pruned metadata)
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrunedMetadata {
    pub id: i32,
    pub name: String,
    pub choose_vo_path: Option<String>,
    pub skins: Vec<PrunedSkin>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrunedSkin {
    pub id: i32,
    pub is_base: bool,
    pub name: String,
    pub skin_classification: Option<String>,
    pub tile_path: Option<String>,
    pub rarity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chromas: Option<Vec<PrunedChroma>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrunedChroma {
    pub id: i32,
    pub name: String,
    pub tile_path: Option<String>,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn get_data_dir(app_handle: &AppHandle) -> AppResult<PathBuf> {
    let settings_state = app_handle.state::<crate::state::SettingsState>();
    let settings = settings_state
        .0
        .lock()
        .map_err(|e| AppError::InternalState(e.to_string()))?;

    match &settings.workspace_path {
        Some(path) => Ok(path.clone()),
        None => Err(AppError::Other(
            "Workspace path not configured. Please set it in Settings.".to_string(),
        )),
    }
}

/// Load champions_with_skins.json and extract all valid skin IDs
async fn load_valid_skin_ids(data_dir: &PathBuf) -> AppResult<HashMap<i32, HashSet<i32>>> {
    let file_path = data_dir.join("champions_with_skins.json");

    if !file_path.exists() {
        return Err(AppError::Other(
            "champions_with_skins.json not found. Please refresh the database first.".to_string(),
        ));
    }

    let content = fs::read_to_string(&file_path)
        .await
        .map_err(|e| AppError::Other(format!("Failed to read champions_with_skins.json: {}", e)))?;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ChampionWithSkins {
        id: i32,
        skin_collection: HashMap<String, String>,
    }

    let champions: Vec<ChampionWithSkins> = serde_json::from_str(&content).map_err(|e| {
        AppError::Other(format!("Failed to parse champions_with_skins.json: {}", e))
    })?;

    let mut result: HashMap<i32, HashSet<i32>> = HashMap::new();

    for champion in champions {
        let skin_ids: HashSet<i32> = champion
            .skin_collection
            .keys()
            .filter_map(|id_str| id_str.parse::<i32>().ok())
            .collect();
        result.insert(champion.id, skin_ids);
    }

    Ok(result)
}

/// Transform tile_path: replace base URL and lowercase
pub fn transform_tile_path(path: Option<String>) -> Option<String> {
    path.map(|p| {
        p.replace(
            "/lol-game-data/assets/",
            "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/",
        )
        .to_lowercase()
    })
}

/// Prune a single metadata file
pub fn prune_metadata(raw: RawMetadata, valid_skin_ids: &HashSet<i32>) -> PrunedMetadata {
    let pruned_skins: Vec<PrunedSkin> = raw
        .skins
        .into_iter()
        .filter(|skin| valid_skin_ids.contains(&skin.id))
        .map(|skin| {
            // Filter chromas to only those with valid IDs
            let pruned_chromas = skin.chromas.map(|chromas| {
                chromas
                    .into_iter()
                    .filter(|chroma| valid_skin_ids.contains(&chroma.id))
                    .map(|chroma| PrunedChroma {
                        id: chroma.id,
                        name: chroma.name,
                        tile_path: transform_tile_path(chroma.tile_path),
                    })
                    .collect::<Vec<_>>()
            });

            // Only include chromas if there are any after filtering
            let chromas = pruned_chromas.and_then(|c| if c.is_empty() { None } else { Some(c) });

            PrunedSkin {
                id: skin.id,
                is_base: skin.is_base,
                name: skin.name,
                skin_classification: skin.skin_classification,
                tile_path: transform_tile_path(skin.tile_path),
                rarity: skin.rarity,
                chromas,
            }
        })
        .collect();

    PrunedMetadata {
        id: raw.id,
        name: raw.name,
        choose_vo_path: raw.choose_vo_path,
        skins: pruned_skins,
    }
}

// ============================================================================
// TAURI COMMAND
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PruneResult {
    pub success: bool,
    pub message: String,
    pub pruned_count: usize,
    pub deleted_count: usize,
}

#[tauri::command]
pub async fn prune_all_metadata(app_handle: AppHandle) -> IpcResult<PruneResult> {
    prune_all_metadata_inner(&app_handle).await.into()
}

async fn prune_all_metadata_inner(app_handle: &AppHandle) -> AppResult<PruneResult> {
    tracing::info!("Starting metadata pruning...");

    let data_dir = get_data_dir(app_handle)?;
    let valid_skin_ids_map = load_valid_skin_ids(&data_dir).await?;

    let metadata_base_dir = data_dir.join("data");
    if !metadata_base_dir.exists() {
        return Ok(PruneResult {
            success: true,
            message: "No metadata directory found.".to_string(),
            pruned_count: 0,
            deleted_count: 0,
        });
    }

    let mut pruned_count = 0;
    let mut deleted_count = 0;

    // Read all champion folders
    let mut entries = fs::read_dir(&metadata_base_dir)
        .await
        .map_err(|e| AppError::Other(format!("Failed to read data directory: {}", e)))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| AppError::Other(format!("Failed to read directory entry: {}", e)))?
    {
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        // Get champion ID from folder name
        let folder_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        let champion_id: i32 = match folder_name.parse() {
            Ok(id) => id,
            Err(_) => continue, // Skip non-numeric folders
        };

        // Check if champion exists in champions_with_skins.json
        let valid_skin_ids = match valid_skin_ids_map.get(&champion_id) {
            Some(ids) => ids,
            None => {
                // Champion doesn't exist, delete the folder
                tracing::info!("Deleting metadata for unknown champion: {}", champion_id);
                if let Err(e) = fs::remove_dir_all(&path).await {
                    tracing::warn!("Failed to delete folder {}: {}", path.display(), e);
                } else {
                    deleted_count += 1;
                }
                continue;
            }
        };

        // Process metadata.json
        let metadata_path = path.join("metadata.json");
        if !metadata_path.exists() {
            continue;
        }

        let content = match fs::read_to_string(&metadata_path).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to read {}: {}", metadata_path.display(), e);
                continue;
            }
        };

        let raw_metadata: RawMetadata = match serde_json::from_str(&content) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Failed to parse {}: {}", metadata_path.display(), e);
                continue;
            }
        };

        // Prune the metadata
        let pruned = prune_metadata(raw_metadata, valid_skin_ids);

        // Write back
        let pruned_json = serde_json::to_string_pretty(&pruned)
            .map_err(|e| AppError::Other(format!("Failed to serialize pruned metadata: {}", e)))?;

        fs::write(&metadata_path, pruned_json)
            .await
            .map_err(|e| AppError::Other(format!("Failed to write pruned metadata: {}", e)))?;

        pruned_count += 1;
        tracing::info!(
            "Pruned metadata for champion {} ({})",
            champion_id,
            pruned.name
        );
    }

    Ok(PruneResult {
        success: true,
        message: format!(
            "Pruned {} metadata files, deleted {} invalid champion folders",
            pruned_count, deleted_count
        ),
        pruned_count,
        deleted_count,
    })
}
