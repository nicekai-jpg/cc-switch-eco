// 端到端测试：模拟完整的 Eco 切换流程
//
// 运行方式: cargo test --test eco_e2e_test -- --nocapture

use std::fs;
use serde_json::json;
use cc_switch_eco_lib::services::ecosystem::fragment;
use cc_switch_eco_lib::services::ecosystem::migration;

/// 模拟完整的 Eco 切换流程
#[test]
fn test_full_switch_workflow() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().to_path_buf();
    let claude_dir = base.join(".claude");
    let ecosystems_dir = base.join("ecosystems");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::create_dir_all(&ecosystems_dir).unwrap();

    // === 步骤1: 创建 Eco A (omc) ===
    let eco_a_dir = ecosystems_dir.join("eco-a");
    let eco_a_rootfiles = eco_a_dir.join("rootfiles");
    fs::create_dir_all(&eco_a_rootfiles).unwrap();

    // eco.json
    fs::write(
        eco_a_dir.join("eco.json"),
        serde_json::to_string_pretty(&json!({
            "id": "eco-a",
            "name": "Eco A",
            "frameworks": ["ohmyclaudecode"],
            "isolatedDirs": [],
            "isolatedFiles": ["settings.json"]
        })).unwrap(),
    ).unwrap();

    // omc fragment
    let omc_frag = json!({
        "defaultMode": "bypassPermissions",
        "effort": "high",
        "language": "English",
        "permissions": {"allow": ["Bash", "Read"], "deny": []}
    });
    fs::write(
        eco_a_rootfiles.join("settings.omc-fragment.json"),
        serde_json::to_string_pretty(&omc_frag).unwrap(),
    ).unwrap();

    // 初始 settings.json（从 omc fragment 重建）
    fragment::rebuild_root_file(
        &eco_a_rootfiles,
        "settings.json",
        &["ohmyclaudecode".to_string()],
    ).unwrap();

    println!("=== 步骤1: Eco A 初始 settings.json ===");
    let settings_a: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(eco_a_rootfiles.join("settings.json")).unwrap()
    ).unwrap();
    println!("{}", serde_json::to_string_pretty(&settings_a).unwrap());

    // 验证初始状态
    assert_eq!(settings_a["defaultMode"], "bypassPermissions");
    assert_eq!(settings_a["effort"], "high");
    assert_eq!(settings_a["language"], "English");

    // === 步骤2: 用户在 Eco A 中修改了 language 为 "中文" ===
    let mut user_modified = settings_a.clone();
    user_modified["language"] = json!("中文");
    fs::write(
        eco_a_rootfiles.join("settings.json"),
        serde_json::to_string_pretty(&user_modified).unwrap(),
    ).unwrap();

    println!("\n=== 步骤2: 用户修改 language 为中文 ===");

    // === 步骤3: 切换前 snapshot_user_preferences ===
    let _isolation = fragment::EcoIsolation {
        dirs: vec![],
        files: vec!["settings.json".to_string()],
    };
    // 模拟 snapshot: 保存当前 settings.json 到 user-fragment
    let current_settings: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(eco_a_rootfiles.join("settings.json")).unwrap()
    ).unwrap();
    fs::write(
        eco_a_rootfiles.join("settings.user-fragment.json"),
        serde_json::to_string_pretty(&current_settings).unwrap(),
    ).unwrap();

    println!("=== 步骤3: snapshot user-fragment 已创建 ===");

    // === 步骤4: 创建 Eco B (ruflo) ===
    let eco_b_dir = ecosystems_dir.join("eco-b");
    let eco_b_rootfiles = eco_b_dir.join("rootfiles");
    fs::create_dir_all(&eco_b_rootfiles).unwrap();

    fs::write(
        eco_b_dir.join("eco.json"),
        serde_json::to_string_pretty(&json!({
            "id": "eco-b",
            "name": "Eco B",
            "frameworks": ["ruflo"],
            "isolatedDirs": [],
            "isolatedFiles": ["settings.json"]
        })).unwrap(),
    ).unwrap();

    // ruflo fragment
    let ruflo_frag = json!({
        "defaultMode": "plan",
        "effort": "max",
        "language": "中文",
        "permissions": {"allow": ["Bash", "Write"], "deny": ["WebFetch"]}
    });
    fs::write(
        eco_b_rootfiles.join("settings.ruflo-fragment.json"),
        serde_json::to_string_pretty(&ruflo_frag).unwrap(),
    ).unwrap();

    // 重建 Eco B 的 settings.json
    fragment::rebuild_root_file(
        &eco_b_rootfiles,
        "settings.json",
        &["ruflo".to_string()],
    ).unwrap();

    println!("\n=== 步骤4: Eco B 初始 settings.json ===");
    let settings_b: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(eco_b_rootfiles.join("settings.json")).unwrap()
    ).unwrap();
    println!("{}", serde_json::to_string_pretty(&settings_b).unwrap());

    assert_eq!(settings_b["defaultMode"], "plan");
    assert_eq!(settings_b["effort"], "max");
    assert_eq!(settings_b["language"], "中文");

    // === 步骤5: 用户在 Eco B 中修改了 defaultMode 为 "bypassPermissions" ===
    let mut user_modified_b = settings_b.clone();
    user_modified_b["defaultMode"] = json!("bypassPermissions");
    fs::write(
        eco_b_rootfiles.join("settings.json"),
        serde_json::to_string_pretty(&user_modified_b).unwrap(),
    ).unwrap();

    // 保存 Eco B 的 user-fragment
    fs::write(
        eco_b_rootfiles.join("settings.user-fragment.json"),
        serde_json::to_string_pretty(&user_modified_b).unwrap(),
    ).unwrap();

    println!("\n=== 步骤5: Eco B 用户修改 defaultMode 为 bypassPermissions ===");

    // === 步骤6: 切换回 Eco A，重建 settings.json ===
    fragment::rebuild_root_file(
        &eco_a_rootfiles,
        "settings.json",
        &["ohmyclaudecode".to_string()],
    ).unwrap();

    let rebuilt_a: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(eco_a_rootfiles.join("settings.json")).unwrap()
    ).unwrap();
    println!("\n=== 步骤6: 切换回 Eco A，重建后 ===");
    println!("{}", serde_json::to_string_pretty(&rebuilt_a).unwrap());

    // 验证：用户偏好 language=中文 应该保留
    assert_eq!(rebuilt_a["language"], "中文", "切换回 Eco A 后，用户偏好 language=中文 应保留");
    assert_eq!(rebuilt_a["defaultMode"], "bypassPermissions", "defaultMode 应保留");
    assert_eq!(rebuilt_a["effort"], "high", "effort 应保留");

    // === 步骤7: 切换到 Eco B，重建 settings.json ===
    fragment::rebuild_root_file(
        &eco_b_rootfiles,
        "settings.json",
        &["ruflo".to_string()],
    ).unwrap();

    let rebuilt_b: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(eco_b_rootfiles.join("settings.json")).unwrap()
    ).unwrap();
    println!("\n=== 步骤7: 切换到 Eco B，重建后 ===");
    println!("{}", serde_json::to_string_pretty(&rebuilt_b).unwrap());

    // 验证：用户偏好 defaultMode=bypassPermissions 应该保留
    assert_eq!(rebuilt_b["defaultMode"], "bypassPermissions", "切换到 Eco B 后，用户偏好 defaultMode 应保留");
    assert_eq!(rebuilt_b["effort"], "max", "effort 应为 ruflo 的 max");
    assert_eq!(rebuilt_b["language"], "中文", "language 应保留");

    println!("\n✅ 完整切换流程测试通过！");
}

/// 测试旧版 Eco 迁移后切换
#[test]
fn test_legacy_eco_migration_then_switch() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().to_path_buf();
    let ecosystems_dir = base.join("ecosystems");
    fs::create_dir_all(&ecosystems_dir).unwrap();

    // === 创建旧版 Eco（没有 fragment）===
    let eco_dir = ecosystems_dir.join("eco-legacy");
    let rootfiles_dir = eco_dir.join("rootfiles");
    fs::create_dir_all(&rootfiles_dir).unwrap();

    fs::write(
        eco_dir.join("eco.json"),
        serde_json::to_string_pretty(&json!({
            "id": "eco-legacy",
            "name": "Legacy Eco",
            "frameworks": ["ohmyclaudecode"],
            "isolatedDirs": [],
            "isolatedFiles": ["settings.json"]
        })).unwrap(),
    ).unwrap();

    // 旧版 settings.json（直接写入，没有 fragment）
    let legacy_settings = json!({
        "defaultMode": "bypassPermissions",
        "effort": "high",
        "language": "中文",
        "permissions": {"allow": ["Bash", "Read"], "deny": []}
    });
    fs::write(
        rootfiles_dir.join("settings.json"),
        serde_json::to_string_pretty(&legacy_settings).unwrap(),
    ).unwrap();

    println!("=== 旧版 Eco settings.json ===");
    println!("{}", serde_json::to_string_pretty(&legacy_settings).unwrap());

    // === 运行迁移 ===
    let isolation = fragment::EcoIsolation {
        dirs: vec![],
        files: vec!["settings.json".to_string()],
    };
    migration::migrate_legacy_rootfiles(&eco_dir, &isolation).unwrap();

    println!("\n=== 迁移后 rootfiles ===");
    for entry in fs::read_dir(&rootfiles_dir).unwrap() {
        let entry = entry.unwrap();
        println!("  {}", entry.file_name().to_string_lossy());
    }

    // 验证 user-fragment 已创建
    assert!(rootfiles_dir.join("settings.user-fragment.json").exists());

    // === 重建 settings.json ===
    fragment::rebuild_root_file(
        &rootfiles_dir,
        "settings.json",
        &["ohmyclaudecode".to_string()],
    ).unwrap();

    let rebuilt: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(rootfiles_dir.join("settings.json")).unwrap()
    ).unwrap();
    println!("\n=== 重建后 settings.json ===");
    println!("{}", serde_json::to_string_pretty(&rebuilt).unwrap());

    // 验证：用户偏好应保留
    assert_eq!(rebuilt["defaultMode"], "bypassPermissions");
    assert_eq!(rebuilt["effort"], "high");
    assert_eq!(rebuilt["language"], "中文");

    println!("\n✅ 旧版迁移后切换测试通过！");
}

/// 测试多框架 Eco 的冲突检测和用户偏好优先
#[test]
fn test_multi_framework_conflict_detection() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().to_path_buf();
    let ecosystems_dir = base.join("ecosystems");
    fs::create_dir_all(&ecosystems_dir).unwrap();

    let eco_dir = ecosystems_dir.join("eco-multi");
    let rootfiles_dir = eco_dir.join("rootfiles");
    fs::create_dir_all(&rootfiles_dir).unwrap();

    fs::write(
        eco_dir.join("eco.json"),
        serde_json::to_string_pretty(&json!({
            "id": "eco-multi",
            "name": "Multi Framework",
            "frameworks": ["ohmyclaudecode", "ruflo"],
            "isolatedDirs": [],
            "isolatedFiles": ["settings.json"]
        })).unwrap(),
    ).unwrap();

    // omc fragment
    fs::write(
        rootfiles_dir.join("settings.omc-fragment.json"),
        serde_json::to_string_pretty(&json!({
            "defaultMode": "bypassPermissions",
            "effort": "high",
            "language": "English",
            "permissions": {"allow": ["Bash", "Read"], "deny": []}
        })).unwrap(),
    ).unwrap();

    // ruflo fragment
    fs::write(
        rootfiles_dir.join("settings.ruflo-fragment.json"),
        serde_json::to_string_pretty(&json!({
            "defaultMode": "plan",
            "effort": "max",
            "language": "中文",
            "permissions": {"allow": ["Bash", "Write", "Edit"], "deny": ["WebFetch"]}
        })).unwrap(),
    ).unwrap();

    // user-fragment
    fs::write(
        rootfiles_dir.join("settings.user-fragment.json"),
        serde_json::to_string_pretty(&json!({
            "defaultMode": "bypassPermissions",
            "language": "中文"
        })).unwrap(),
    ).unwrap();

    // 重建
    fragment::rebuild_root_file(
        &rootfiles_dir,
        "settings.json",
        &["ohmyclaudecode".to_string(), "ruflo".to_string()],
    ).unwrap();

    let result: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(rootfiles_dir.join("settings.json")).unwrap()
    ).unwrap();
    println!("多框架合并结果: {}", serde_json::to_string_pretty(&result).unwrap());

    // 用户偏好优先
    assert_eq!(result["defaultMode"], "bypassPermissions", "用户偏好 defaultMode 应优先");
    assert_eq!(result["language"], "中文", "用户偏好 language 应优先");
    // ruflo 最后覆盖框架值
    assert_eq!(result["effort"], "max", "effort 应为 ruflo 的 max");
    // 数组去重合并
    let allow = result["permissions"]["allow"].as_array().unwrap();
    let allow_strs: Vec<&str> = allow.iter().filter_map(|v| v.as_str()).collect();
    assert!(allow_strs.contains(&"Bash"));
    assert!(allow_strs.contains(&"Read"));
    assert!(allow_strs.contains(&"Write"));
    assert!(allow_strs.contains(&"Edit"));

    // 检查冲突记录
    let eco_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(eco_dir.join("eco.json")).unwrap()
    ).unwrap();
    let conflicts = eco_json.get("mergeConflicts")
        .and_then(|v| v.get("settings.json"))
        .and_then(|v| v.as_array());
    if let Some(conflicts) = conflicts {
        println!("框架间冲突: {:?}", conflicts);
        // 应该有 defaultMode 和 language 的冲突（框架间）
        // 但不应包含用户覆盖的冲突
        let conflict_strs: Vec<String> = conflicts.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        // 框架间冲突应包含 defaultMode 和 language
        assert!(conflict_strs.iter().any(|c| c.contains("defaultMode")), "应有 defaultMode 冲突");
        assert!(conflict_strs.iter().any(|c| c.contains("language")), "应有 language 冲突");
        // 不应包含用户覆盖的冲突
        assert!(!conflict_strs.iter().any(|c| c.contains("user-")), "不应有用户覆盖冲突");
    }

    println!("\n✅ 多框架冲突检测测试通过！");
}
