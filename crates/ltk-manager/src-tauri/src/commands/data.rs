use crate::error::{AppError, AppResult, IpcResult};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const SKIN_IDS_URL: &str =
    "https://github.com/Alban1911/LeagueSkins/raw/main/resources/vi/skin_ids.json";
const SKIN_IDS_FILENAME: &str = "skin_ids.json";
const CHAMPION_DATA_URL: &str = 
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/champion-summary.json";
const CHAMPION_DATA_FILENAME: &str = "champion_data.json";
const VERSION_API_URL: &str = "https://ddragon.leagueoflegends.com/api/versions.json";
const VERSION_FILENAME: &str = "version.json";
const CHAMPION_ICONS_URL: &str = 
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/champion-icons/";

// ...

// Download champion icons từ Community Dragon
async fn download_champion_icons(
    data_dir: &PathBuf,
    champions: &[ChampionWithSkins],
) -> AppResult<usize> {
    let icons_dir = data_dir.join(CHAMPION_ICONS_DIR);
    
    // Tạo folder champion-icons nếu chưa có
    if !icons_dir.exists() {
        fs::create_dir_all(&icons_dir)
            .map_err(|e| AppError::Other(format!("Failed to create icons directory: {}", e)))?;
    }

    tracing::info!("Downloading champion icons to {:?}", icons_dir);

    let client = reqwest::Client::new();
    let mut downloaded = 0;
    let mut skipped = 0;

    for champion in champions {
        let icon_filename = format!("{}.png", champion.id);
        let icon_path = icons_dir.join(&icon_filename);

        // Skip nếu đã có file
        if icon_path.exists() {
            skipped += 1;
            continue;
        }

        let icon_url = format!("{}{}.png", CHAMPION_ICONS_URL, champion.id);
        
        match client.get(&icon_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.bytes().await {
                        Ok(bytes) => {
                            if let Err(e) = fs::write(&icon_path, &bytes) {
                                tracing::warn!("Failed to save icon for {}: {}", champion.name, e);
                            } else {
                                downloaded += 1;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to read icon bytes for {}: {}", champion.name, e);
                        }
                    }
                } else {
                    tracing::warn!(
                        "Failed to download icon for {} ({}): HTTP {}",
                        champion.name,
                        champion.id,
                        response.status()
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Failed to fetch icon for {}: {}", champion.name, e);
            }
        }
    }

    tracing::info!(
        "Champion icons: {} downloaded, {} skipped (already exist)",
        downloaded,
        skipped
    );

    Ok(downloaded)
}

// ... imports
use base64::Engine;

#[tauri::command]
pub fn get_champion_icon_data(
    app_handle: AppHandle,
    champion_id: i32,
) -> IpcResult<String> {
    get_champion_icon_data_inner(&app_handle, champion_id).into()
}

fn get_champion_icon_data_inner(
    app_handle: &AppHandle,
    champion_id: i32,
) -> AppResult<String> {
    let data_dir = get_data_dir(app_handle)?;
    // Try .png first (community dragon), then .jpg (ddragon if present)
    let mut icon_path = data_dir.join(CHAMPION_ICONS_DIR).join(format!("{}.png", champion_id));
    
    if !icon_path.exists() {
        icon_path = data_dir.join(CHAMPION_ICONS_DIR).join(format!("{}.jpg", champion_id));
    }

    if icon_path.exists() {
        let bytes = fs::read(&icon_path)
            .map_err(|e| AppError::Other(format!("Failed to read icon file: {}", e)))?;
        
        let base64_str = base64::engine::general_purpose::STANDARD.encode(&bytes);
        // Return data URI
        let mime_type = if icon_path.extension().and_then(|e| e.to_str()) == Some("jpg") {
            "image/jpeg"
        } else {
            "image/png"
        };
        Ok(format!("data:{};base64,{}", mime_type, base64_str))
    } else {
        Err(AppError::Other(format!(
            "Icon not found for champion {}. Please refresh the database.",
            champion_id
        )))
    }
}
const CHAMPION_ICONS_DIR: &str = "champion-icons";


#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionInfo {
    version: String,
    last_updated: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    pub success: bool,
    pub message: String,
    pub count: usize,
}

// Struct từ API Community Dragon
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChampionData {
    id: i32,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    alias: String,
}

// Struct cho champion với skin collection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampionWithSkins {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub alias: String,
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
            .map_err(|e| AppError::Other(format!("Failed to create data dir: {}", e)))?;
    }

    // 1. Fetch và lưu skin_ids.json
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
        .map_err(|e| AppError::Other(format!("Failed to write skin data file: {}", e)))?;

    // 2. Fetch và lưu champion_data.json
    tracing::info!("Fetching champion data from {}", CHAMPION_DATA_URL);
    let champions_response = reqwest::get(CHAMPION_DATA_URL)
        .await
        .map_err(|e| AppError::Other(format!("Failed to fetch champion data: {}", e)))?;

    if !champions_response.status().is_success() {
        return Err(AppError::Other(format!(
            "Failed to fetch champion data: HTTP {}",
            champions_response.status()
        )));
    }

    let champions_text = champions_response
        .text()
        .await
        .map_err(|e| AppError::Other(format!("Failed to fetch champion data text: {}", e)))?;

    // Validate JSON
    let champions: Vec<ChampionData> = serde_json::from_str(&champions_text)
        .map_err(|e| AppError::Other(format!("Failed to parse champion data JSON: {}", e)))?;

    let champions_count = champions.len();
    tracing::info!("Fetched {} champions", champions_count);

    // Save champion_data.json
    let champions_file_path = data_dir.join(CHAMPION_DATA_FILENAME);
    fs::write(&champions_file_path, champions_text)
        .map_err(|e| AppError::Other(format!("Failed to write champion data file: {}", e)))?;

    // Organize và lưu champions_with_skins.json
    tracing::info!("Organizing champions with skins...");
    let organized_champions = organize_skins_by_champion(skins, champions);
    let organized_count = organized_champions.len();
    
    let organized_json = serde_json::to_string_pretty(&organized_champions)
        .map_err(|e| AppError::Other(format!("Failed to serialize organized data: {}", e)))?;
    
    let organized_file_path = data_dir.join("champions_with_skins.json");
    fs::write(&organized_file_path, organized_json)
        .map_err(|e| AppError::Other(format!("Failed to write organized data file: {}", e)))?;

    tracing::info!("Saved {} champions with skins to file", organized_count);

    // Download champion icons
    let icons_downloaded = download_champion_icons(&data_dir, &organized_champions).await?;

    Ok(UpdateResult {
        success: true,
        message: format!(
            "Updated {} skins, {} champions, {} icons downloaded",
            skins_count, champions_count, icons_downloaded
        ),
        count: skins_count,
    })
}

#[tauri::command]
pub fn get_skin_database(app_handle: AppHandle) -> IpcResult<HashMap<String, String>> {
    get_skin_database_inner(&app_handle).into()
}

fn get_skin_database_inner(app_handle: &AppHandle) -> AppResult<HashMap<String, String>> {
    let data_dir = get_data_dir(app_handle)?;
    let file_path = data_dir.join(SKIN_IDS_FILENAME);

    if !file_path.exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(&file_path)
        .map_err(|e| AppError::Other(format!("Failed to read skin data file: {}", e)))?;

    let skins: HashMap<String, String> = serde_json::from_str(&content)
        .map_err(|e| AppError::Other(format!("Failed to parse skin data file: {}", e)))?;

    Ok(skins)
}


// Organize skins theo champion
fn organize_skins_by_champion(
    skins: HashMap<String, String>,
    champions: Vec<ChampionData>,
) -> Vec<ChampionWithSkins> {
    // Tạo HashMap để tra cứu champion theo ID
    let champion_map: HashMap<i32, ChampionData> = champions
        .into_iter()
        .filter(|c| c.id > 0) // Bỏ champion với id <= 0
        .map(|c| (c.id, c))
        .collect();

    // Nhóm skins theo champion ID
    let mut champion_skins: HashMap<i32, BTreeMap<String, String>> = HashMap::new();

    for (skin_id, skin_name) in skins {
        // Parse skin ID thành số
        if let Ok(id) = skin_id.parse::<i32>() {
            // Lấy champion ID: bỏ 3 số cuối
            let champion_id = id / 1000;

            champion_skins
                .entry(champion_id)
                .or_insert_with(BTreeMap::new)
                .insert(skin_id, skin_name);
        }
    }

    // Tạo danh sách champions với skins
    let mut result: Vec<ChampionWithSkins> = champion_skins
        .into_iter()
        .filter_map(|(champion_id, skin_collection)| {
            champion_map.get(&champion_id).map(|champion| ChampionWithSkins {
                id: champion.id,
                name: champion.name.clone(),
                description: champion.description.clone(),
                alias: champion.alias.clone(),
                skin_collection,
            })
        })
        .collect();

    // Sắp xếp theo ID
    result.sort_by_key(|c| c.id);

    result
}

#[tauri::command]
pub fn get_champions_with_skins(
    app_handle: AppHandle,
) -> IpcResult<Vec<ChampionWithSkins>> {
    get_champions_with_skins_inner(&app_handle).into()
}

fn get_champions_with_skins_inner(
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
        .map_err(|e| AppError::Other(format!("Failed to read champions data file: {}", e)))?;

    let champions: Vec<ChampionWithSkins> = serde_json::from_str(&content)
        .map_err(|e| AppError::Other(format!("Failed to parse champions data file: {}", e)))?;

    tracing::info!("Loaded {} champions with skins from file", champions.len());

    Ok(champions)
}



// Lấy version mới nhất từ API
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

// Đọc version đã lưu
fn load_saved_version(app_handle: &AppHandle) -> AppResult<Option<VersionInfo>> {
    let data_dir = get_data_dir(app_handle)?;
    let file_path = data_dir.join(VERSION_FILENAME);

    if !file_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&file_path)
        .map_err(|e| AppError::Other(format!("Failed to read version file: {}", e)))?;

    let version_info: VersionInfo = serde_json::from_str(&content)
        .map_err(|e| AppError::Other(format!("Failed to parse version file: {}", e)))?;

    Ok(Some(version_info))
}

// Lưu version
fn save_version(app_handle: &AppHandle, version: &str) -> AppResult<()> {
    let data_dir = get_data_dir(app_handle)?;
    
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .map_err(|e| AppError::Other(format!("Failed to create data dir: {}", e)))?;
    }

    let version_info = VersionInfo {
        version: version.to_string(),
        last_updated: chrono::Utc::now().to_rfc3339(),
    };

    let json = serde_json::to_string_pretty(&version_info)
        .map_err(|e| AppError::Other(format!("Failed to serialize version: {}", e)))?;

    let file_path = data_dir.join(VERSION_FILENAME);
    fs::write(&file_path, json)
        .map_err(|e| AppError::Other(format!("Failed to write version file: {}", e)))?;

    Ok(())
}

/// Check version và tự động update nếu cần
/// Gọi function này khi app khởi động
#[tauri::command]
pub async fn check_and_update_database(app_handle: AppHandle) -> IpcResult<UpdateResult> {
    check_and_update_database_inner(&app_handle).await.into()
}

async fn check_and_update_database_inner(app_handle: &AppHandle) -> AppResult<UpdateResult> {
    tracing::info!("Checking for database updates...");

    // Lấy version mới nhất
    let latest_version = fetch_latest_version().await?;
    tracing::info!("Latest version: {}", latest_version);

    // Kiểm tra version đã lưu
    let saved_version = load_saved_version(app_handle)?;

    let should_update = match saved_version {
        None => {
            tracing::info!("No saved version found, will update database");
            true
        }
        Some(saved) => {
            if saved.version != latest_version {
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
        // Update database
        let result = refresh_skin_database_inner(app_handle).await?;

        // Lưu version mới
        save_version(app_handle, &latest_version)?;

        Ok(result)
    } else {
        Ok(UpdateResult {
            success: true,
            message: format!("Database is up to date (version: {})", latest_version),
            count: 0,
        })
    }
}

