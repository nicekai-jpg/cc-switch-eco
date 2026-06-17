use std::fs;

use serde_json::{Map, Value};

use crate::config::{get_claude_settings_path, read_json_file, write_json_file};
use crate::database::Database;
use crate::error::AppError;
use crate::services::ecosystem::{ecosystem_dir, fragment};

pub const BYPASS_PERMISSIONS_MODE: &str = "bypassPermissions";

fn read_settings_object(path: &std::path::Path) -> Result<Map<String, Value>, AppError> {
    let settings: Value = if path.exists() {
        read_json_file(path)?
    } else {
        Value::Object(Map::new())
    };

    settings
        .as_object()
        .cloned()
        .ok_or_else(|| AppError::Config("settings.json 根必须是对象".into()))
}

fn write_settings_object(path: &std::path::Path, obj: &Map<String, Value>) -> Result<(), AppError> {
    write_json_file(path, &Value::Object(obj.clone()))
}

fn permissions_default_mode(obj: &Map<String, Value>) -> Option<&str> {
    obj.get("permissions")
        .and_then(|v| v.get("defaultMode"))
        .and_then(|v| v.as_str())
}

fn set_permissions_default_mode(obj: &mut Map<String, Value>, mode: &str) {
    let permissions = obj
        .entry("permissions".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if let Some(perms) = permissions.as_object_mut() {
        perms.insert("defaultMode".into(), Value::String(mode.into()));
    }
}

fn remove_permissions_default_mode(obj: &mut Map<String, Value>) {
    if let Some(perms) = obj.get_mut("permissions").and_then(|v| v.as_object_mut()) {
        perms.remove("defaultMode");
        if perms.is_empty() {
            obj.remove("permissions");
        }
    }
}

fn is_bypass_permissions_enabled(obj: &Map<String, Value>) -> bool {
    obj.get("defaultMode").and_then(|v| v.as_str()) == Some(BYPASS_PERMISSIONS_MODE)
        && permissions_default_mode(obj) == Some(BYPASS_PERMISSIONS_MODE)
}

fn enable_bypass_permissions_in_object(obj: &mut Map<String, Value>) {
    obj.insert(
        "defaultMode".into(),
        Value::String(BYPASS_PERMISSIONS_MODE.into()),
    );
    set_permissions_default_mode(obj, BYPASS_PERMISSIONS_MODE);
}

fn disable_bypass_permissions_in_object(obj: &mut Map<String, Value>) -> bool {
    let mut changed = false;

    if obj.get("defaultMode").and_then(|v| v.as_str()) == Some(BYPASS_PERMISSIONS_MODE) {
        obj.remove("defaultMode");
        changed = true;
    }

    if permissions_default_mode(obj) == Some(BYPASS_PERMISSIONS_MODE) {
        remove_permissions_default_mode(obj);
        changed = true;
    }

    changed
}

/// 在 ~/.claude/settings.json 启用 bypassPermissions 默认权限模式。
pub fn apply_bypass_permissions() -> Result<bool, AppError> {
    let path = get_claude_settings_path();
    let mut obj = read_settings_object(&path)?;

    if is_bypass_permissions_enabled(&obj) {
        return Ok(false);
    }

    enable_bypass_permissions_in_object(&mut obj);
    write_settings_object(&path, &obj)?;
    Ok(true)
}

/// 从 ~/.claude/settings.json 移除 bypassPermissions 默认权限模式。
pub fn clear_bypass_permissions() -> Result<bool, AppError> {
    let path = get_claude_settings_path();
    if !path.exists() {
        return Ok(false);
    }

    let mut obj = read_settings_object(&path)?;
    let changed = disable_bypass_permissions_in_object(&mut obj);
    if !changed {
        return Ok(false);
    }

    write_settings_object(&path, &obj)?;
    Ok(true)
}

/// 仅将 bypassPermissions 键写入当前生态的 user-fragment，避免整份覆盖 live 配置导致 hooks 丢失。
pub fn sync_bypass_permissions_to_eco(db: &Database, enabled: bool) -> Result<(), AppError> {
    let current_eco = match db.get_current_ecosystem()? {
        Some(eco) => eco,
        None => return Ok(()),
    };

    if !enabled {
        fragment::remove_user_preference(&current_eco.id, "settings.json", "defaultMode")?;
        fragment::remove_user_preference(
            &current_eco.id,
            "settings.json",
            "permissions.defaultMode",
        )?;
        return Ok(());
    }

    let eco_dir = ecosystem_dir(&current_eco.id);
    let rootfiles_dir = eco_dir.join("rootfiles");
    let user_frag_path = fragment::fragment_path(&rootfiles_dir, "settings.json", "user-");

    let mut obj = if user_frag_path.exists() {
        read_settings_object(&user_frag_path)?
    } else {
        Map::new()
    };
    enable_bypass_permissions_in_object(&mut obj);

    let content = fragment::write_json(&Value::Object(obj))?;
    fs::write(&user_frag_path, &content).map_err(|e| AppError::io(&user_frag_path, e))?;

    // 重建 settings.json，让 bypass 进入 rootfiles，但不把 live 全量快照写回 fragment
    fragment::rebuild_all_root_files(&eco_dir)?;

    Ok(())
}

/// 若设备级设置已开启 bypassPermissions，在生态切换或 settings 重建后重新写入 live 配置。
pub fn reapply_bypass_permissions_if_enabled(db: &Database) -> Result<(), AppError> {
    if !crate::settings::get_settings().claude_bypass_permissions {
        return Ok(());
    }

    apply_bypass_permissions()?;
    sync_bypass_permissions_to_eco(db, true)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enable_bypass_permissions_sets_both_keys() {
        let mut obj = Map::new();
        enable_bypass_permissions_in_object(&mut obj);

        assert_eq!(
            obj.get("defaultMode").and_then(|v| v.as_str()),
            Some(BYPASS_PERMISSIONS_MODE)
        );
        assert_eq!(permissions_default_mode(&obj), Some(BYPASS_PERMISSIONS_MODE));
    }

    #[test]
    fn disable_bypass_permissions_keeps_other_modes() {
        let mut obj = Map::new();
        obj.insert("defaultMode".into(), Value::String("auto".into()));
        set_permissions_default_mode(&mut obj, "auto");

        assert!(!disable_bypass_permissions_in_object(&mut obj));
        assert_eq!(obj.get("defaultMode").and_then(|v| v.as_str()), Some("auto"));
    }

    #[test]
    fn disable_bypass_permissions_removes_only_bypass_keys() {
        let mut obj = Map::new();
        obj.insert(
            "defaultMode".into(),
            Value::String(BYPASS_PERMISSIONS_MODE.into()),
        );
        set_permissions_default_mode(&mut obj, BYPASS_PERMISSIONS_MODE);
        obj.insert(
            "permissions".into(),
            Value::Object(Map::from_iter([
                (
                    "defaultMode".into(),
                    Value::String(BYPASS_PERMISSIONS_MODE.into()),
                ),
                (
                    "allow".into(),
                    Value::Array(vec![Value::String("Bash(*)".into())]),
                ),
            ])),
        );

        assert!(disable_bypass_permissions_in_object(&mut obj));
        assert!(obj.get("defaultMode").is_none());
        assert!(permissions_default_mode(&obj).is_none());
        assert!(obj
            .get("permissions")
            .and_then(|v| v.get("allow"))
            .is_some());
    }
}
