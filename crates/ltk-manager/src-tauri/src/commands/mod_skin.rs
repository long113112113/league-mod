use crate::{
    error::{AppError, AppResult, IpcResult},
    patcher::PatcherState,
};
use anyhow::Context;
use std::path::PathBuf;
use tauri::{command, Manager};
use tokio::io::AsyncWriteExt;
use tracing::{error, info, warn};

#[cfg(target_os = "windows")]

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
pub async fn download_skin(
    app_handle: tauri::AppHandle,
    champion_id: i32,
    skin_id: i32,
) -> IpcResult<String> {
    match download_skin_inner(app_handle, champion_id, skin_id).await {
        Ok(msg) => IpcResult::Ok { value: msg },
        Err(e) => IpcResult::Err {
            error: crate::error::AppErrorResponse::new(
                crate::error::ErrorCode::Unknown,
                format!("{:#}", e),
            ),
        },
    }
}

async fn download_skin_inner(
    app_handle: tauri::AppHandle,
    champion_id: i32,
    skin_id: i32,
) -> anyhow::Result<String> {
    info!(
        "Starting download_skin_inner: champion_id={}, skin_id={}",
        champion_id, skin_id
    );
    let data_dir_root = get_data_dir(&app_handle).context("Failed to get data directory")?;
    let champion_dir = data_dir_root.join("data").join(champion_id.to_string());

    if !champion_dir.exists() {
        tokio::fs::create_dir_all(&champion_dir)
            .await
            .context("Failed to create champion directory")?;
    }

    let extract_to = champion_dir.join(skin_id.to_string());
    if extract_to.exists() {
        info!("Skin {} already downloaded at {:?}", skin_id, extract_to);
        return Ok(format!(
            "Skin {} already downloaded at {:?}",
            skin_id, extract_to
        ));
    }
    let client = reqwest::Client::new();
    let extensions = ["zip", "fantome"];
    let mut final_response = None;
    let mut file_path = PathBuf::new();
    let mut worked_url = String::new();

    for ext in extensions {
        let url = format!(
            "https://github.com/Alban1911/LeagueSkins/raw/main/skins/{}/{}/{}.{}",
            champion_id, skin_id, skin_id, ext
        );
        info!("Checking URL: {}", url);

        // We use a match to safely handle potential network errors on a per-attempt basis if needed,
        // but here we primarily care about the status code.
        match client.get(&url).send().await {
            Ok(res) => {
                if res.status().is_success() {
                    final_response = Some(res);
                    file_path = champion_dir.join(format!("{}.{}", skin_id, ext));
                    worked_url = url;
                    break;
                }
            }
            Err(e) => {
                warn!("Failed to request {}: {}", url, e);
                // Continue to try the next extension
            }
        }
    }

    let response = final_response.ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to download skin (checked zip and fantome) for champion {} skin {}",
            champion_id,
            skin_id
        )
    })?;

    info!(
        "Download connection established: {}, status: {}",
        worked_url,
        response.status()
    );

    let bytes = response.bytes().await?;
    let mut file = tokio::fs::File::create(&file_path).await?;
    file.write_all(&bytes).await?;
    info!(
        "File downloaded to {:?}, size: {} bytes",
        file_path,
        bytes.len()
    );

    // Unzip logic
    let file = std::fs::File::open(&file_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // let extract_to = champion_dir.join(skin_id.to_string()); // Moved up
    if !extract_to.exists() {
        std::fs::create_dir_all(&extract_to)?;
    }

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => extract_to.join(path),
            None => continue,
        };

        if (*file.name()).ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)?;
                }
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    // Cleanup zip file
    tokio::fs::remove_file(&file_path).await?;
    info!("Extraction complete. Removed zip file.");

    Ok(format!(
        "Downloaded and extracted skin {} to {:?}",
        skin_id, extract_to
    ))
}

#[command]
pub async fn run_skin(
    app_handle: tauri::AppHandle,
    champion_id: i32,
    skin_id: i32,
) -> IpcResult<String> {
    {
        let child_process_to_kill = {
            let patcher_state_arc = app_handle.state::<PatcherState>();
            let mut patcher_state = match patcher_state_arc.0.lock() {
                Ok(state) => state,
                Err(e) => {
                    return IpcResult::Err {
                        error: crate::error::AppErrorResponse::new(
                            crate::error::ErrorCode::Unknown,
                            format!("Failed to lock patcher state: {}", e),
                        ),
                    };
                }
            };

            // Cancel token
            if let Some(token) = patcher_state.cancel_token.take() {
                token.cancel();
                info!("Cancelled previous operation token");
            }

            patcher_state.child_process.take()
        };

        // Kill child process outside lock
        if let Some(mut child) = child_process_to_kill {
            let _ = child.start_kill(); // fast kill
            let _ = child.wait().await; // Ensure it's dead
            info!("Killed previous mod-tools process");
        }
    }

    // Create new cancel token
    let cancel_token = tokio_util::sync::CancellationToken::new();
    {
        let patcher_state_arc = app_handle.state::<PatcherState>();
        let mut patcher_state = match patcher_state_arc.0.lock() {
            Ok(state) => state,
            Err(e) => {
                return IpcResult::Err {
                    error: crate::error::AppErrorResponse::new(
                        crate::error::ErrorCode::Unknown,
                        format!("Failed to lock patcher state: {}", e),
                    ),
                };
            }
        };
        patcher_state.cancel_token = Some(cancel_token.clone());
    }

    match run_skin_inner(app_handle, champion_id, skin_id, cancel_token).await {
        Ok(msg) => IpcResult::Ok { value: msg },
        Err(e) => {
            error!("run_skin error: {:#}", e);
            IpcResult::Err {
                error: crate::error::AppErrorResponse::new(
                    crate::error::ErrorCode::Unknown,
                    format!("{:#}", e),
                ),
            }
        }
    }
}

#[command]
pub async fn stop_all_mods(app_handle: tauri::AppHandle) -> IpcResult<String> {
    let patcher_state_arc = app_handle.state::<PatcherState>();

    // We need to take the child process out of the state to kill it
    let child_proc = {
        let mut patcher_state = match patcher_state_arc.0.lock() {
            Ok(state) => state,
            Err(e) => {
                return IpcResult::Err {
                    error: crate::error::AppErrorResponse::new(
                        crate::error::ErrorCode::Unknown,
                        format!("Failed to lock patcher state: {}", e),
                    ),
                };
            }
        };

        // Cancel token
        if let Some(token) = patcher_state.cancel_token.take() {
            token.cancel();
            info!("Cancelled operation token");
        }

        patcher_state.child_process.take()
    };

    if let Some(mut child) = child_proc {
        let _ = child.start_kill();
        let _ = child.wait().await;
        info!("Stopped mod-tools process");
        IpcResult::Ok {
            value: "Mods stopped".to_string(),
        }
    } else {
        IpcResult::Ok {
            value: "No mods running".to_string(),
        }
    }
}

fn resolve_tool_path(app_handle: &tauri::AppHandle, tool_name: &str) -> anyhow::Result<PathBuf> {
    let resource_path = app_handle
        .path()
        .resource_dir()
        .context("Failed to get resource directory")?
        .join(tool_name);

    if resource_path.exists() {
        return Ok(resource_path);
    }

    // Fallback for development
    let dev_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .map(|p| p.join(tool_name));

    if let Some(path) = &dev_path {
        if path.exists() {
            info!("Found tool in dev path: {:?}", path);
            return Ok(path.clone());
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        let source_path = cwd.join("src-tauri").join("resources").join(tool_name);
        if source_path.exists() {
            info!("Found tool in source resources: {:?}", source_path);
            return Ok(source_path);
        }
        let workspace_source_path = cwd
            .join("crates")
            .join("ltk-manager")
            .join("src-tauri")
            .join("resources")
            .join(tool_name);
        if workspace_source_path.exists() {
            info!(
                "Found tool in workspace source resources: {:?}",
                workspace_source_path
            );
            return Ok(workspace_source_path);
        }
    }

    warn!(
        "Tool {} not found in resource dir ({:?}) or dev dev path ({:?})",
        tool_name, resource_path, dev_path
    );
    Err(anyhow::anyhow!("Tool not found: {}", tool_name))
}

async fn run_skin_inner(
    app_handle: tauri::AppHandle,
    champion_id: i32,
    skin_id: i32,
    cancel_token: tokio_util::sync::CancellationToken,
) -> anyhow::Result<String> {
    info!(
        "Starting run_skin_inner: champion_id={}, skin_id={}",
        champion_id, skin_id
    );

    let (workspace_path, league_path) = {
        let settings_state = app_handle.state::<crate::state::SettingsState>();
        let settings = settings_state
            .0
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock settings: {}", e))?;

        let workspace_path = settings
            .workspace_path
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Workspace path not configured"))?;
        let league_path = settings
            .league_path
            .clone()
            .ok_or_else(|| anyhow::anyhow!("League path not configured"))?;
        (workspace_path, league_path)
    };

    info!(
        "Settings loaded. Workspace: {:?}, League: {:?}",
        workspace_path, league_path
    );

    let mods_base_dir = workspace_path.join("data").join(champion_id.to_string());
    let skin_dir = mods_base_dir.join(skin_id.to_string());

    if !skin_dir.exists() {
        return Err(anyhow::anyhow!(
            "Skin directory not found. Please download first: {:?}",
            skin_dir
        ));
    }

    let overlay_dir = workspace_path.join("data").join("overlay");
    if overlay_dir.exists() {
        tokio::fs::remove_dir_all(&overlay_dir)
            .await
            .context("Failed to clean overlay dir")?;
    }
    tokio::fs::create_dir_all(&overlay_dir)
        .await
        .context("Failed to create overlay dir")?;
    let mod_tools_path = resolve_tool_path(&app_handle, "mod-tools.exe")?;
    info!("Using mod-tools at: {:?}", mod_tools_path);
    let game_dir = league_path.join("Game");
    let game_path_str = if game_dir.exists() {
        game_dir.to_string_lossy().to_string()
    } else {
        league_path.to_string_lossy().to_string()
    };

    info!("Using game path: {}", game_path_str);

    let args_mk = vec![
        "mkoverlay".to_string(),
        mods_base_dir.to_string_lossy().to_string(),
        overlay_dir.to_string_lossy().to_string(),
        format!("--game:{}", game_path_str),
        format!("--mods:{}", skin_id),
        "--noTFT".to_string(),
        "--ignoreConflict".to_string(),
    ];

    // Check cancellation before mkoverlay
    if cancel_token.is_cancelled() {
        return Err(anyhow::anyhow!("Operation cancelled"));
    }

    info!("Running mkoverlay: {:?}", args_mk);

    let output_mk = tokio::process::Command::new(&mod_tools_path)
        .args(&args_mk)
        .creation_flags(0x08000000)
        .output()
        .await?;

    if !output_mk.status.success() {
        error!(
            "mkoverlay failed: stderr: {}",
            String::from_utf8_lossy(&output_mk.stderr)
        );
        return Err(anyhow::anyhow!(
            "mkoverlay failed with status: {}",
            output_mk.status
        ));
    }
    info!("mkoverlay success");

    // 4. Run runoverlay
    // Command: runoverlay <overlay_dir> <config_path> --game:<game_dir> --opts:configless
    let config_path = overlay_dir.join("cslol-config.json");

    let args_run = vec![
        "runoverlay".to_string(),
        overlay_dir.to_string_lossy().to_string(),
        config_path.to_string_lossy().to_string(),
        format!("--game:{}", game_path_str),
        "--opts:configless".to_string(),
    ];

    info!("Running runoverlay: {:?}", args_run);

    // Spawn with piped output
    let mut command = tokio::process::Command::new(&mod_tools_path);
    command
        .args(&args_run)
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = command.spawn()?;

    // Take stdout/stderr handles here so we can move them to the logger task
    let child_stdout = child.stdout.take();
    let child_stderr = child.stderr.take();

    // Check cancellation before registering
    if cancel_token.is_cancelled() {
        let _ = child.start_kill();
        return Err(anyhow::anyhow!("Operation cancelled"));
    }

    // Store child in state with strict cancellation check
    {
        let patcher_state_arc = app_handle.state::<PatcherState>();
        let mut patcher_state = patcher_state_arc
            .inner()
            .0
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock patcher state: {}", e))?;

        if cancel_token.is_cancelled() {
            let _ = child.start_kill();
            return Err(anyhow::anyhow!("Operation cancelled during spawn"));
        }

        patcher_state.child_process = Some(child);
    }

    // Spawn task to stream logs
    tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;

        let mut handles = Vec::new();

        if let Some(stdout) = child_stdout {
            handles.push(tokio::spawn(async move {
                let mut reader = tokio::io::BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    info!("[mod-tools stdout] {}", line);
                }
            }));
        }

        if let Some(stderr) = child_stderr {
            handles.push(tokio::spawn(async move {
                let mut reader = tokio::io::BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    error!("[mod-tools stderr] {}", line);
                }
            }));
        }

        // Wait for both
        for h in handles {
            let _ = h.await;
        }
    });

    Ok("Skin run active".to_string())
}
