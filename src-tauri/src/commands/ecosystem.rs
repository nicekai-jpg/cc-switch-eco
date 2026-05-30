use crate::services::ecosystem::EcosystemService;
use crate::services::ecosystem_framework;
use crate::store::AppState;
use tauri::State;

#[tauri::command]
pub async fn create_ecosystem(
    state: State<'_, AppState>,
    name: String,
    description: String,
    frameworks: Vec<String>,
) -> Result<crate::database::Ecosystem, String> {
    EcosystemService::create(&state, &name, &description, frameworks).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn switch_ecosystem(state: State<'_, AppState>, id: String) -> Result<(), String> {
    EcosystemService::switch(&state, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_ecosystem(state: State<'_, AppState>, id: String) -> Result<(), String> {
    EcosystemService::delete(&state, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_ecosystems(
    state: State<'_, AppState>,
) -> Result<Vec<crate::database::Ecosystem>, String> {
    EcosystemService::list(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_current_ecosystem(
    state: State<'_, AppState>,
) -> Result<Option<crate::database::Ecosystem>, String> {
    EcosystemService::get_current(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_frameworks() -> Result<Vec<ecosystem_framework::FrameworkRegistry>, String> {
    Ok(ecosystem_framework::get_all_frameworks())
}

#[tauri::command]
pub async fn install_framework_to_ecosystem(
    state: State<'_, AppState>,
    eco_id: String,
    framework_id: String,
) -> Result<(), String> {
    EcosystemService::install_framework(&state, &eco_id, &framework_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn uninstall_framework_from_ecosystem(
    state: State<'_, AppState>,
    eco_id: String,
    framework_id: String,
) -> Result<(), String> {
    EcosystemService::uninstall_framework(&state, &eco_id, &framework_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_framework_in_ecosystem(
    state: State<'_, AppState>,
    eco_id: String,
    framework_id: String,
) -> Result<(), String> {
    EcosystemService::update_framework(&state, &eco_id, &framework_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_ecosystem_frameworks(eco_id: String) -> Result<Vec<String>, String> {
    EcosystemService::get_ecosystem_frameworks(&eco_id).map_err(|e| e.to_string())
}
