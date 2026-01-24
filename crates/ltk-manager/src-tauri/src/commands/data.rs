use crate::error::{AppError, AppResult, IpcResult};
use crate::commands::merge_data::{prune_metadata, RawMetadata};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::fs;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

const SKIN_IDS_URL: &str =
    "https://github.com/Alban1911/LeagueSkins/raw/main/resources/vi/skin_ids.json";
const SKIN_IDS_FILENAME: &str = "skin_ids.json";
const VERSION_API_URL: &str = "https://ddragon.leagueoflegends.com/api/versions.json";
const VERSION_FILENAME: &str = "version.json";

const METADATA_URL_TEMPLATE: &str = 
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/vi_vn/v1/champions/{id}.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionInfo {
    version: String,
    last_updated: String,
    #[serde(default)]
    hash_1: String
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    pub success: bool,
    pub message: String,
    pub count: usize,
}

// Struct cho champion với skin collection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampionWithSkins {
    pub id: i32,
    pub name: String,
    pub skin_collection: BTreeMap<String, String>,
}

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

#[tauri::command]
pub async fn refresh_skin_database(app_handle: AppHandle) -> IpcResult<UpdateResult> {
    refresh_skin_database_inner(&app_handle).await.into()
}

async fn refresh_skin_database_inner(app_handle: &AppHandle) -> AppResult<UpdateResult> {
    tracing::info!("Fetching skin database and champion data...");

    // Ensure data directory exists
    let data_dir = get_data_dir(app_handle)?;
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .await
            .map_err(|e| AppError::Other(format!("Failed to create data dir: {}", e)))?;
    }

    tracing::info!("Fetching skin database from {}", SKIN_IDS_URL);
    let skins_response = reqwest::get(SKIN_IDS_URL)
        .await
        .map_err(|e| AppError::Other(format!("Failed to fetch skin data: {}", e)))?;

    if !skins_response.status().is_success() {
        return Err(AppError::Other(format!(
            "Failed to fetch skin data: HTTP {}",
            skins_response.status()
        )));
    }

    let skins_text = skins_response
        .text()
        .await
        .map_err(|e| AppError::Other(format!("Failed to fetch skin data text: {}", e)))?;

    // Validate JSON by parsing it and count entries
    let skins: HashMap<String, String> = serde_json::from_str(&skins_text)
        .map_err(|e| AppError::Other(format!("Failed to parse skin data JSON: {}", e)))?;

    let skins_count = skins.len();
    tracing::info!("Fetched {} skin entries", skins_count);

    // Save skin_ids.json
    let skins_file_path = data_dir.join(SKIN_IDS_FILENAME);
    fs::write(&skins_file_path, skins_text)
        .await
        .map_err(|e| AppError::Other(format!("Failed to write skin data file: {}", e)))?;

    // Organize và lưu champions_with_skins.json
    tracing::info!("Organizing champions with skins...");
    let organized_champions = organize_skins_by_champion(skins);
    let organized_count = organized_champions.len();
    
    let organized_json = serde_json::to_string_pretty(&organized_champions)
        .map_err(|e| AppError::Other(format!("Failed to serialize organized data: {}", e)))?;
    
    let organized_file_path = data_dir.join("champions_with_skins.json");
    fs::write(&organized_file_path, organized_json)
        .await
        .map_err(|e| AppError::Other(format!("Failed to write organized data file: {}", e)))?;

    tracing::info!("Saved {} champions with skins to file", organized_count);

    // Download champion icons


    // Initialize data folders and download metadata
    let metadata_count = download_champion_metadata(app_handle, &data_dir, &organized_champions).await?;

    Ok(UpdateResult {
        success: true,
        message: format!(
            "Updated {} skins, 0 champions (derived), {} metadata files checked",
            skins_count, metadata_count
        ),
        count: skins_count,
    })
}

async fn download_champion_metadata(
    app_handle: &AppHandle,
    data_dir: &PathBuf,
    champions: &[ChampionWithSkins],
) -> AppResult<usize> {
    use tauri::Emitter;

    #[derive(Clone, Serialize)]
    struct ProgressPayload {
        processed: usize,
        total: usize,
        message: String,
    }

    let client = reqwest::Client::new();
    let mut join_set = JoinSet::new();
    let semaphore = Arc::new(Semaphore::new(50));
    let mut count = 0;

    // Count total work
    let total_work = champions.iter().filter(|c| c.id > 0).count();
    let _ = app_handle.emit("metadata-download-progress", ProgressPayload {
        processed: 0,
        total: total_work,
        message: "Downloading metadata...".into(),
    });

    // Build skin IDs map for pruning
    let skin_ids_map: HashMap<i32, HashSet<i32>> = champions
        .iter()
        .map(|c| {
            let skin_ids: HashSet<i32> = c
                .skin_collection
                .keys()
                .filter_map(|id_str| id_str.parse::<i32>().ok())
                .collect();
            (c.id, skin_ids)
        })
        .collect();

    for champion in champions {
        // Skip dummy/invalid champions if any
        if champion.id <= 0 {
            continue;
        }

        let client = client.clone();
        let champ_id = champion.id;
        let champ_name = champion.name.clone();
        let data_dir = data_dir.clone();
        let valid_skin_ids = skin_ids_map.get(&champ_id).cloned().unwrap_or_default();
        let permit = semaphore.clone().acquire_owned().await.map_err(|e| {
            AppError::Other(format!("Failed to acquire semaphore: {}", e))
        })?;

        join_set.spawn(async move {
            // Drop permit when the task completes
            let _permit = permit;

            // Create champion specific folder: data/{id}
            let champion_dir = data_dir.join("data").join(champ_id.to_string());
            
            // Check existence asynchronously
            if !tokio::fs::try_exists(&champion_dir).await.unwrap_or(false) {
                if let Err(e) = fs::create_dir_all(&champion_dir).await {
                     tracing::warn!(
                        "Failed to create directory for champion {}: {}",
                        champ_id, e
                    );
                    return 0;
                }
            }

            let metadata_path = champion_dir.join("metadata.json");

            // Skip if metadata already exists
            if tokio::fs::try_exists(&metadata_path).await.unwrap_or(false) {
                return 0;
            }

            let url = METADATA_URL_TEMPLATE.replace("{id}", &champ_id.to_string());
            tracing::info!("Downloading metadata for {} from {}", champ_name, url);

            match client.get(&url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.text().await {
                            Ok(text) => {
                                // Parse raw metadata
                                let raw_metadata: RawMetadata = match serde_json::from_str(&text) {
                                    Ok(m) => m,
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to parse metadata for {}: {}",
                                            champ_name, e
                                        );
                                        return 0;
                                    }
                                };

                                let pruned = prune_metadata(raw_metadata, &valid_skin_ids);

                                let pruned_json = match serde_json::to_string_pretty(&pruned) {
                                    Ok(j) => j,
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to serialize pruned metadata for {}: {}",
                                            champ_name, e
                                        );
                                        return 0;
                                    }
                                };

                                if let Err(e) = fs::write(&metadata_path, pruned_json).await {
                                    tracing::warn!(
                                        "Failed to write metadata for {}: {}",
                                        champ_name,
                                        e
                                    );
                                    0
                                } else {
                                    1
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to read metadata text for {}: {}",
                                    champ_name,
                                    e
                                );
                                0
                            }
                        }
                    } else {
                        tracing::warn!(
                            "Failed to download metadata for {} ({}): HTTP {}",
                            champ_name,
                            champ_id,
                            response.status()
                        );
                        0
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch metadata for {}: {}", champ_name, e);
                    0
                }
            }
        });
    }

    let mut processed = 0;
    while let Some(result) = join_set.join_next().await {
        processed += 1;
        let _ = app_handle.emit("metadata-download-progress", ProgressPayload {
            processed,
            total: total_work,
            message: format!("Downloading metadata... {}/{}", processed, total_work),
        });

        match result {
            Ok(downloaded) => count += downloaded,
            Err(e) => tracing::error!("Task join error: {}", e),
        }
    }

    Ok(count)
}

#[tauri::command]
pub async fn get_skin_database(app_handle: AppHandle) -> IpcResult<HashMap<String, String>> {
    get_skin_database_inner(&app_handle).await.into()
}

async fn get_skin_database_inner(app_handle: &AppHandle) -> AppResult<HashMap<String, String>> {
    let data_dir = get_data_dir(app_handle)?;
    let file_path = data_dir.join(SKIN_IDS_FILENAME);

    if !file_path.exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(&file_path)
        .await
        .map_err(|e| AppError::Other(format!("Failed to read skin data file: {}", e)))?;

    let skins: HashMap<String, String> = serde_json::from_str(&content)
        .map_err(|e| AppError::Other(format!("Failed to parse skin data file: {}", e)))?;

    Ok(skins)
}


// Organize skins theo champion
fn organize_skins_by_champion(
    skins: HashMap<String, String>,
) -> Vec<ChampionWithSkins> {
    let mut champions_map: HashMap<i32, String> = HashMap::new();
    let mut champion_skins: HashMap<i32, BTreeMap<String, String>> = HashMap::new();

    for (skin_id_str, skin_name) in &skins {
        if let Ok(id) = skin_id_str.parse::<i32>() {
            let champion_id = id / 1000;
            if id % 1000 == 0 {
                champions_map.insert(champion_id, skin_name.clone());
            }
            champion_skins
                .entry(champion_id)
                .or_default()
                .insert(skin_id_str.clone(), skin_name.clone());
        }
    }

    let mut result: Vec<ChampionWithSkins> = champions_map
        .into_iter()
        .map(|(champion_id, name)| {
            let skin_collection = champion_skins.remove(&champion_id).unwrap_or_default();
            ChampionWithSkins {
                id: champion_id,
                name,
                skin_collection,
            }
        })
        .collect();

    result.sort_by_key(|c| c.id);

    result
}

#[tauri::command]
pub async fn get_champions_with_skins(
    app_handle: AppHandle,
) -> IpcResult<Vec<ChampionWithSkins>> {
    get_champions_with_skins_inner(&app_handle).await.into()
}

async fn get_champions_with_skins_inner(
    app_handle: &AppHandle,
) -> AppResult<Vec<ChampionWithSkins>> {
    let data_dir = get_data_dir(app_handle)?;
    let file_path = data_dir.join("champions_with_skins.json");

    if !file_path.exists() {
        return Err(AppError::Other(
            "Champions data not found. Please refresh the database first.".to_string(),
        ));
    }

    let content = fs::read_to_string(&file_path)
        .await
        .map_err(|e| AppError::Other(format!("Failed to read champions data file: {}", e)))?;

    let champions: Vec<ChampionWithSkins> = serde_json::from_str(&content)
        .map_err(|e| AppError::Other(format!("Failed to parse champions data file: {}", e)))?;

    tracing::info!("Loaded {} champions with skins from file", champions.len());

    Ok(champions)
}

async fn fetch_latest_version() -> AppResult<String> {
    tracing::info!("Fetching latest version from {}", VERSION_API_URL);
    
    let response = reqwest::get(VERSION_API_URL)
        .await
        .map_err(|e| AppError::Other(format!("Failed to fetch version: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::Other(format!(
            "Failed to fetch version: HTTP {}",
            response.status()
        )));
    }

    let versions: Vec<String> = response
        .json()
        .await
        .map_err(|e| AppError::Other(format!("Failed to parse version JSON: {}", e)))?;

    versions
        .first()
        .cloned()
        .ok_or_else(|| AppError::Other("Version list is empty".to_string()))
}

async fn load_saved_version(app_handle: &AppHandle) -> AppResult<Option<VersionInfo>> {
    let data_dir = get_data_dir(app_handle)?;
    let file_path = data_dir.join(VERSION_FILENAME);

    if !file_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&file_path)
        .await
        .map_err(|e| AppError::Other(format!("Failed to read version file: {}", e)))?;

    let version_info: VersionInfo = serde_json::from_str(&content)
        .map_err(|e| AppError::Other(format!("Failed to parse version file: {}", e)))?;

    Ok(Some(version_info))
}

async fn save_version(app_handle: &AppHandle, version: &str) -> AppResult<()> {
    let data_dir = get_data_dir(app_handle)?;
    
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .await
            .map_err(|e| AppError::Other(format!("Failed to create data dir: {}", e)))?;
    }

    let current_info = load_saved_version(app_handle).await?.unwrap_or(VersionInfo {
        version: "".to_string(),
        last_updated: "".to_string(),
        hash_1: "".to_string()
    });

    let version_info = VersionInfo {
        version: version.to_string(),
        last_updated: chrono::Utc::now().to_rfc3339(),
        hash_1: current_info.hash_1
    };

    let json = serde_json::to_string_pretty(&version_info)
        .map_err(|e| AppError::Other(format!("Failed to serialize version: {}", e)))?;

    let file_path = data_dir.join(VERSION_FILENAME);
    fs::write(&file_path, json)
        .await
        .map_err(|e| AppError::Other(format!("Failed to write version file: {}", e)))?;

    Ok(())
}

#[tauri::command]
pub async fn check_and_update_database(app_handle: AppHandle) -> IpcResult<UpdateResult> {
    check_and_update_database_inner(&app_handle).await.into()
}

async fn check_and_update_database_inner(app_handle: &AppHandle) -> AppResult<UpdateResult> {
    tracing::info!("Checking for database updates...");

    let latest_version = fetch_latest_version().await?;
    tracing::info!("Latest version: {}", latest_version);

    let saved_version = load_saved_version(app_handle).await?;

    let should_update = match saved_version {
        None => {
            tracing::info!("No saved version found, will update database");
            true
        }
        Some(saved) => {
            let data_folder = get_data_dir(app_handle)?.join("data");
            let champions_file = get_data_dir(app_handle)?.join("champions_with_skins.json");
            
            if !data_folder.exists() || !champions_file.exists() {
                tracing::info!("Data directory or champions file missing, forcing update");
                true
            } else if saved.version != latest_version {
                tracing::info!(
                    "Version changed: {} -> {}, will update database",
                    saved.version,
                    latest_version
                );
                true
            } else {
                tracing::info!("Version unchanged: {}, skipping update", latest_version);
                false
            }
        }
    };

    if should_update {
        let result = refresh_skin_database_inner(app_handle).await?;

        save_version(app_handle, &latest_version).await?;

        Ok(result)
    } else {
        Ok(UpdateResult {
            success: true,
            message: format!("Database is up to date (version: {})", latest_version),
            count: 0,
        })
    }
}





#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChromaData {
    pub id: i32,
    pub name: String,
    pub tile_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinData {
    pub id: i32,
    pub name: String,
    pub tile_path: String,
    pub rarity: String,
    pub is_base: bool,
    #[serde(default)]
    pub chromas: Vec<ChromaData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChampionMetadata {
    pub id: i32,
    pub name: String,
    pub skins: Vec<SkinData>,
}

#[tauri::command]
pub async fn get_champion_skins(
    app_handle: AppHandle,
    champion_id: i32,
) -> IpcResult<Vec<SkinData>> {
    get_champion_skins_inner(&app_handle, champion_id).await.into()
}

async fn get_champion_skins_inner(
    app_handle: &AppHandle,
    champion_id: i32,
) -> AppResult<Vec<SkinData>> {
    let data_dir = get_data_dir(app_handle)?;
    // Path: data/{id}/metadata.json
    let metadata_path = data_dir
        .join("data")
        .join(champion_id.to_string())
        .join("metadata.json");

    if !tokio::fs::try_exists(&metadata_path).await.unwrap_or(false) {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&metadata_path)
        .await
        .map_err(|e| AppError::Other(format!("Failed to read metadata file: {}", e)))?;

    let metadata: ChampionMetadata = serde_json::from_str(&content)
        .map_err(|e| AppError::Other(format!("Failed to parse metadata file: {}", e)))?;

    Ok(metadata.skins)
}


