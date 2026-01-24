use crate::error::{AppError, AppResult, IpcResult};
use anyhow::Context;
use serde::Deserialize;
use std::path::PathBuf;
use tauri::{command, Manager};
use tokio::io::AsyncWriteExt;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Metadata {
    skins: Vec<Skin>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Skin {
    id: i32,
    tile_path: String,
    chromas: Option<Vec<Chroma>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Chroma {
    id: i32,
    tile_path: String,
}

fn get_data_dir(app_handle: &tauri::AppHandle) -> AppResult<PathBuf> {
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

#[command]
pub async fn download_champion_images(
    app_handle: tauri::AppHandle,
    champion_id: i32,
) -> IpcResult<String> {
    match download_champion_images_inner(app_handle, champion_id).await {
        Ok(msg) => IpcResult::Ok { value: msg },
        Err(e) => IpcResult::Err {
            error: crate::error::AppErrorResponse::new(
                crate::error::ErrorCode::Unknown,
                format!("{:#}", e),
            ),
        },
    }
}

pub async fn download_champion_images_inner(
    app_handle: tauri::AppHandle,
    champion_id: i32,
) -> anyhow::Result<String> {
    // Construct path to metadata.json
    let data_dir_root = get_data_dir(&app_handle).context("Failed to get data directory")?;
    let data_dir = data_dir_root.join("data").join(champion_id.to_string());
    let metadata_path = data_dir.join("metadata.json");

    if !metadata_path.exists() {
        return Err(anyhow::anyhow!(
            "Metadata file not found at {:?}",
            metadata_path
        ));
    }

    let content = tokio::fs::read_to_string(&metadata_path)
        .await
        .context("Failed to read metadata.json")?;

    let metadata: Metadata =
        serde_json::from_str(&content).context("Failed to parse metadata.json")?;

    let images_dir = data_dir.join("images");
    if !images_dir.exists() {
        tokio::fs::create_dir_all(&images_dir)
            .await
            .context("Failed to create images directory")?;
    }

    let mut tasks = Vec::new();
    let client = reqwest::Client::new();

    for skin in metadata.skins {
        let (skin_id, skin_tile_path, skin_chromas) = (skin.id, skin.tile_path, skin.chromas);

        let client_clone = client.clone();
        let images_dir_clone = images_dir.clone();

        // Task for skin image
        tasks.push(tokio::spawn(async move {
            download_image(&client_clone, skin_id, &skin_tile_path, &images_dir_clone).await
        }));

        if let Some(chromas) = skin_chromas {
            for chroma in chromas {
                let client_clone = client.clone();
                let images_dir_clone = images_dir.clone();
                tasks.push(tokio::spawn(async move {
                    download_image(
                        &client_clone,
                        chroma.id,
                        &chroma.tile_path,
                        &images_dir_clone,
                    )
                    .await
                }));
            }
        }
    }

    // Wait for all downloads
    let mut success_count = 0;
    let mut failure_count = 0;

    for task in tasks {
        match task.await {
            Ok(result) => match result {
                Ok(_) => success_count += 1,
                Err(e) => {
                    eprintln!("Image download failed: {:?}", e);
                    failure_count += 1;
                }
            },
            Err(e) => {
                eprintln!("Task join error: {:?}", e);
                failure_count += 1;
            }
        }
    }

    Ok(format!(
        "Downloaded {} images, {} failed",
        success_count, failure_count
    ))
}

async fn download_image(
    client: &reqwest::Client,
    id: i32,
    url: &str,
    dir: &std::path::Path,
) -> anyhow::Result<()> {
    let extension = "jpg";

    let file_path = dir.join(format!("{}.{}", id, extension));

    // Optional: Check if exists to skip?
    // User said "down toàn bộ" (download all), implying force or ensure they are there.

    let bytes = client.get(url).send().await?.bytes().await?;
    let mut file = tokio::fs::File::create(file_path).await?;
    file.write_all(&bytes).await?;

    Ok(())
}

#[command]
pub async fn get_skin_image(
    app_handle: tauri::AppHandle,
    champion_id: i32,
    skin_id: i32,
) -> IpcResult<String> {
    match get_skin_image_inner(app_handle, champion_id, skin_id).await {
        Ok(data) => IpcResult::Ok { value: data },
        Err(e) => IpcResult::Err {
            error: crate::error::AppErrorResponse::new(
                crate::error::ErrorCode::Unknown,
                format!("{:#}", e),
            ),
        },
    }
}

async fn get_skin_image_inner(
    app_handle: tauri::AppHandle,
    champion_id: i32,
    skin_id: i32,
) -> anyhow::Result<String> {
    use base64::Engine;

    let data_dir_root = get_data_dir(&app_handle).context("Failed to get data directory")?;
    let images_dir = data_dir_root
        .join("data")
        .join(champion_id.to_string())
        .join("images");

    // Extensions to try/use. We generally save as jpg
    let file_path = images_dir.join(format!("{}.jpg", skin_id));

    // 1. If exists, return immediately
    if file_path.exists() {
        let data = tokio::fs::read(&file_path).await?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&data);
        return Ok(format!("data:image/jpeg;base64,{}", encoded));
    }

    // 2. If not exists, we need to find the URL from metadata
    let metadata_path = data_dir_root
        .join("data")
        .join(champion_id.to_string())
        .join("metadata.json");

    if !metadata_path.exists() {
        return Err(anyhow::anyhow!(
            "Metadata not found for champion {}",
            champion_id
        ));
    }

    let content = tokio::fs::read_to_string(&metadata_path)
        .await
        .context("Failed to read metadata.json")?;

    let metadata: Metadata = serde_json::from_str(&content).context("Failed to parse metadata")?;

    // Find the URL
    let mut target_url = None;
    for skin in metadata.skins {
        if skin.id == skin_id {
            target_url = Some(skin.tile_path);
            break;
        }
        if let Some(chromas) = skin.chromas {
            for chroma in chromas {
                if chroma.id == skin_id {
                    target_url = Some(chroma.tile_path);
                    break;
                }
            }
        }
        if target_url.is_some() {
            break;
        }
    }

    let url = target_url
        .ok_or_else(|| anyhow::anyhow!("Skin/Chroma ID {} not found in metadata", skin_id))?;

    // 3. Download and Save
    if !images_dir.exists() {
        tokio::fs::create_dir_all(&images_dir).await?;
    }

    let client = reqwest::Client::new();
    let image_data = client.get(&url).send().await?.bytes().await?;

    tokio::fs::write(&file_path, &image_data).await?;

    // 4. Return
    let encoded = base64::engine::general_purpose::STANDARD.encode(&image_data);
    Ok(format!("data:image/jpeg;base64,{}", encoded))
}
