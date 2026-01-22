use crate::error::{AppError, IpcResult};
use tauri::command;
use wadtools::commands::{extract, ExtractArgs};

#[command]
pub fn extract_wad(wad_path: String, output_dir: String) -> IpcResult<()> {
    // Basic extraction with default settings
    let args = ExtractArgs {
        input: wad_path,
        output: Some(output_dir),
        hashtable: None,
        filter_type: None,
        pattern: None,
        hashtable_dir: None,
    };

    match extract(args) {
        Ok(_) => IpcResult::ok(()),
        Err(e) => IpcResult::err(AppError::Other(e.to_string())),
    }
}
