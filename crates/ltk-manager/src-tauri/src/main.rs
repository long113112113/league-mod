#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::Manager;
use tauri_plugin_fs::FsExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;
mod error;
pub mod patcher;
mod state;
mod utils;

use error::IpcResult;
use patcher::PatcherState;
use state::SettingsState;

/// Perform first-run initialization:
/// - If league_path is not set, attempt auto-detection
/// - If auto-detection succeeds, save the path
fn initialize_first_run(app_handle: &tauri::AppHandle, settings_state: &SettingsState) {
    let mut settings = match settings_state.0.lock() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to lock settings: {}", e);
            return;
        }
    };

    // Skip if league path is already configured
    if settings.league_path.is_some() {
        tracing::info!("League path already configured, skipping auto-detection");
        return;
    }

    tracing::info!("Attempting auto-detection of League installation...");

    // Use ltk_mod_core for detection
    if let Some(exe_path) = ltk_mod_core::auto_detect_league_path() {
        let path = std::path::Path::new(exe_path.as_str());

        // Navigate from "Game/League of Legends.exe" to installation root
        if let Some(install_root) = path.parent().and_then(|p| p.parent()) {
            tracing::info!("Auto-detected League at: {:?}", install_root);
            settings.league_path = Some(install_root.to_path_buf());
            settings.first_run_complete = true;

            // Persist the detected path
            if let Err(e) = state::save_settings_to_disk(app_handle, &settings) {
                tracing::error!("Failed to save auto-detected settings: {}", e);
            }
        }
    } else {
        tracing::info!("Auto-detection did not find League installation");
    }
}

fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ltk_manager=debug,tauri=info".into()),
        )
        .init();

    tracing::info!("Starting LTK Manager");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let app_handle = app.handle();

            // Create individual states
            let settings_state = SettingsState::new(app_handle);
            let patcher_state = PatcherState::new();

            // Run first-run initialization (auto-detect League path)
            initialize_first_run(app_handle, &settings_state);

            // Dynamically allow workspace path in fs scope
            if let Ok(settings) = settings_state.0.lock() {
                if let Some(path) = &settings.workspace_path {
                    let _ = app.fs_scope().allow_directory(path, true);
                    tracing::info!("Allowed workspace path in fs scope: {:?}", path);
                }
            }

            // Manage each state separately
            app.manage(settings_state);
            app.manage(patcher_state);

            // Auto-check for database updates in background
            let app_handle_clone = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                match commands::check_and_update_database(app_handle_clone.clone()).await {
                    IpcResult::Ok { value } => {
                        tracing::info!("Database check complete: {}", value.message);
                    }
                    IpcResult::Err { error } => {
                        tracing::warn!("Database check failed: {:?}", error);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // App
            commands::get_app_info,
            // Settings
            commands::get_settings,
            commands::save_settings,
            commands::auto_detect_league_path,
            commands::validate_league_path,
            commands::check_setup_required,
            // Patcher
            commands::start_patcher,
            commands::stop_patcher,
            commands::get_patcher_status,
            // Data
            commands::refresh_skin_database,
            commands::get_skin_database,
            commands::get_champions_with_skins,
            commands::check_and_update_database,
            commands::get_champion_skins,
            // Merge Data
            commands::prune_all_metadata,
            // Images
            commands::download_champion_images,
            commands::get_skin_image,
            // Mod Skin
            commands::mod_skin::download_skin,
            commands::mod_skin::run_skin,
            commands::mod_skin::stop_all_mods,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
