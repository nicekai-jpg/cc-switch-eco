//! Ecosystem 生态隔离命令层

use crate::database::Ecosystem;
use crate::services::ecosystem::EcosystemService;
use crate::store::AppState;
use tauri::State;

#[tauri::command]
pub fn create_ecosystem(
    name: String,
    description: String,
    app_state: State<'_, AppState>,
) -> Result<Ecosystem, String> {
    EcosystemService::create(&app_state, &name, &description).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn switch_ecosystem(
    id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    EcosystemService::switch(&app_state, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_ecosystem(
    id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    EcosystemService::delete(&app_state, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_ecosystems(
    app_state: State<'_, AppState>,
) -> Result<Vec<Ecosystem>, String> {
    EcosystemService::list(&app_state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_current_ecosystem(
    app_state: State<'_, AppState>,
) -> Result<Option<Ecosystem>, String> {
    EcosystemService::get_current(&app_state).map_err(|e| e.to_string())
}