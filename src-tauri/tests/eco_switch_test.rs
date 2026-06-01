// 集成测试：验证 Eco 切换和 fragment 合并逻辑
//
// 运行方式: cargo test --test eco_switch_test -- --nocapture

use std::fs;
use std::path::Path;
use serde_json::json;
use cc_switch_eco_lib::services::ecosystem::fragment;
use cc_switch_eco_lib::services::ecosystem::migration;

/// 测试辅助：创建临时 Eco 目录结构
fn create_test_eco(base: &Path, eco_id: &str, frameworks: &[&str], settings_content: &serde_json::Value) {
    let eco_dir = base.join(eco_id);
    let rootfiles_dir = eco_dir.join("rootfiles");
    fs::create_dir_all(&rootfiles_dir).unwrap();

    // eco.json
    let eco_json = json!({
        "id": eco_id,
        "name": eco_id,
        "frameworks": frameworks,
        "isolatedDirs": [],
        "isolatedFiles": ["settings.json"]
    });
    fs::write(eco_dir.join("eco.json"), serde_json::to_string_pretty(&eco_json).unwrap()).unwrap();

    // settings.json
    fs::write(
        rootfiles_dir.join("settings.json"),
        serde_json::to_string_pretty(settings_content).unwrap(),
    ).unwrap();
}

/// 测试辅助：创建 fragment 文件
fn create_fragment(base: &Path, eco_id: &str, file_name: &str, prefix: &str, content: &serde_json::Value) {
    let rootfiles_dir = base.join(eco_id).join("rootfiles");
    let stem = file_name.strip_suffix(".json").unwrap_or(file_name);
    let frag_path = rootfiles_dir.join(format!("{stem}.{prefix}fragment.json"));
    fs::write(&frag_path, serde_json::to_string_pretty(content).unwrap()).unwrap();
}

/// 测试辅助：读取合并后的 settings.json
fn read_settings(base: &Path, eco_id: &str) -> serde_json::Value {
    let path = base.join(eco_id).join("rootfiles").join("settings.json");
    let content = fs::read_to_string(&path).unwrap();
    serde_json::from_str(&content).unwrap()
}

#[test]
fn test_full_switch_cycle() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().to_path_buf();

    // === 场景1: 创建两个 Eco，各自有不同的 fragment ===

    // Eco A: ohmyclaudecode 框架
    create_test_eco(&base, "eco-a", &["ohmyclaudecode"], &json!({
        "defaultMode": "bypassPermissions",
        "effort": "high",
        "language": "English",
        "permissions": {"allow": ["Bash", "Read"], "deny": []}
    }));

    // 为 eco-a 创建 omc fragment
    create_fragment(&base, "eco-a", "settings.json", "omc-", &json!({
        "defaultMode": "bypassPermissions",
        "effort": "high",
        "language": "English",
        "permissions": {"allow": ["Bash", "Read"], "deny": []}
    }));

    // Eco B: ruflo 框架
    create_test_eco(&base, "eco-b", &["ruflo"], &json!({
        "defaultMode": "plan",
        "effort": "max",
        "language": "中文",
        "permissions": {"allow": ["Bash", "Write"], "deny": ["WebFetch"]}
    }));

    // 为 eco-b 创建 ruflo fragment
    create_fragment(&base, "eco-b", "settings.json", "ruflo-", &json!({
        "defaultMode": "plan",
        "effort": "max",
        "language": "中文",
        "permissions": {"allow": ["Bash", "Write"], "deny": ["WebFetch"]}
    }));

    println!("=== 初始状态 ===");
    println!("eco-a settings: {}", serde_json::to_string_pretty(&read_settings(&base, "eco-a")).unwrap());
    println!("eco-b settings: {}", serde_json::to_string_pretty(&read_settings(&base, "eco-b")).unwrap());

    // === 场景2: 用户在 eco-a 中修改了 language 为 "中文" ===
    let mut eco_a_settings = read_settings(&base, "eco-a");
    eco_a_settings["language"] = json!("中文");
    fs::write(
        base.join("eco-a/rootfiles/settings.json"),
        serde_json::to_string_pretty(&eco_a_settings).unwrap(),
    ).unwrap();

    println!("\n=== 用户修改 eco-a 的 language 为中文 ===");

    // === 场景3: 模拟 snapshot_user_preferences（切换前保存用户偏好）===
    let current_settings = read_settings(&base, "eco-a");
    create_fragment(&base, "eco-a", "settings.json", "user-", &current_settings);

    // === 场景4: rebuild_root_file（切换到 eco-a 时重建）===
    fragment::rebuild_root_file(
        &base.join("eco-a/rootfiles"),
        "settings.json",
        &["ohmyclaudecode".to_string()],
    ).unwrap();

    let rebuilt_a = read_settings(&base, "eco-a");
    println!("\n=== 重建后 eco-a settings ===");
    println!("{}", serde_json::to_string_pretty(&rebuilt_a).unwrap());

    // 验证：用户偏好 language=中文 应该覆盖框架默认 language=English
    assert_eq!(rebuilt_a["language"], "中文", "用户偏好 language 应该保留");
    assert_eq!(rebuilt_a["defaultMode"], "bypassPermissions", "defaultMode 应该保留");
    assert_eq!(rebuilt_a["effort"], "high", "effort 应该保留");

    // === 场景5: eco-b 也保存用户偏好后重建 ===
    let eco_b_settings = read_settings(&base, "eco-b");
    create_fragment(&base, "eco-b", "settings.json", "user-", &eco_b_settings);

    fragment::rebuild_root_file(
        &base.join("eco-b/rootfiles"),
        "settings.json",
        &["ruflo".to_string()],
    ).unwrap();

    let rebuilt_b = read_settings(&base, "eco-b");
    println!("\n=== 重建后 eco-b settings ===");
    println!("{}", serde_json::to_string_pretty(&rebuilt_b).unwrap());

    assert_eq!(rebuilt_b["defaultMode"], "plan");
    assert_eq!(rebuilt_b["effort"], "max");
    assert_eq!(rebuilt_b["language"], "中文");

    println!("\n✅ 所有切换场景测试通过！");
}

#[test]
fn test_multi_framework_merge_with_user_priority() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().to_path_buf();

    // 创建一个同时安装 ohmyclaudecode + ruflo 的 Eco
    create_test_eco(&base, "eco-multi", &["ohmyclaudecode", "ruflo"], &json!({}));

    // omc fragment
    create_fragment(&base, "eco-multi", "settings.json", "omc-", &json!({
        "defaultMode": "bypassPermissions",
        "effort": "high",
        "language": "English",
        "permissions": {"allow": ["Bash", "Read"], "deny": []}
    }));

    // ruflo fragment
    create_fragment(&base, "eco-multi", "settings.json", "ruflo-", &json!({
        "defaultMode": "plan",
        "effort": "max",
        "language": "中文",
        "permissions": {"allow": ["Bash", "Write", "Edit"], "deny": ["WebFetch"]}
    }));

    // user-fragment: 用户偏好 defaultMode=bypassPermissions, language=中文
    create_fragment(&base, "eco-multi", "settings.json", "user-", &json!({
        "defaultMode": "bypassPermissions",
        "language": "中文"
    }));

    // 重建
    fragment::rebuild_root_file(
        &base.join("eco-multi/rootfiles"),
        "settings.json",
        &["ohmyclaudecode".to_string(), "ruflo".to_string()],
    ).unwrap();

    let result = read_settings(&base, "eco-multi");
    println!("多框架合并结果: {}", serde_json::to_string_pretty(&result).unwrap());

    // 用户偏好优先
    assert_eq!(result["defaultMode"], "bypassPermissions", "用户偏好 defaultMode 应优先");
    assert_eq!(result["language"], "中文", "用户偏好 language 应优先");
    // ruflo 最后覆盖框架值
    assert_eq!(result["effort"], "max", "effort 应为 ruflo 的 max");
    // 数组去重合并
    let allow = result["permissions"]["allow"].as_array().unwrap();
    let allow_strs: Vec<&str> = allow.iter().filter_map(|v| v.as_str()).collect();
    assert!(allow_strs.contains(&"Bash"), "Bash 应在 allow 中");
    assert!(allow_strs.contains(&"Read"), "Read 应在 allow 中（来自 omc）");
    assert!(allow_strs.contains(&"Write"), "Write 应在 allow 中（来自 ruflo）");
    assert!(allow_strs.contains(&"Edit"), "Edit 应在 allow 中（来自 ruflo）");
    // deny
    assert_eq!(result["permissions"]["deny"], json!(["WebFetch"]));

    // 检查冲突记录
    let eco_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(base.join("eco-multi/eco.json")).unwrap()
    ).unwrap();
    let conflicts = eco_json.get("mergeConflicts")
        .and_then(|v| v.get("settings.json"))
        .and_then(|v| v.as_array());
    if let Some(conflicts) = conflicts {
        println!("框架间冲突: {:?}", conflicts);
        assert!(conflicts.len() >= 2, "应有至少2个框架间冲突");
    }

    println!("\n✅ 多框架合并测试通过！");
}

#[test]
fn test_legacy_migration() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().to_path_buf();

    // 模拟旧版 Eco：只有 settings.json，没有 fragment
    create_test_eco(&base, "eco-legacy", &["ohmyclaudecode"], &json!({
        "defaultMode": "bypassPermissions",
        "effort": "high",
        "language": "中文"
    }));

    // 运行迁移
    let isolation = fragment::EcoIsolation {
        dirs: vec![],
        files: vec!["settings.json".to_string()],
    };
    migration::migrate_legacy_rootfiles(&base.join("eco-legacy"), &isolation).unwrap();

    // 验证：旧版 settings.json 应该被保存为 user-fragment
    let user_frag_path = base.join("eco-legacy/rootfiles/settings.user-fragment.json");
    assert!(user_frag_path.exists(), "迁移后应创建 user-fragment");

    let user_frag: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&user_frag_path).unwrap()
    ).unwrap();
    assert_eq!(user_frag["defaultMode"], "bypassPermissions");
    assert_eq!(user_frag["language"], "中文");

    // 重建后应保持用户偏好
    fragment::rebuild_root_file(
        &base.join("eco-legacy/rootfiles"),
        "settings.json",
        &["ohmyclaudecode".to_string()],
    ).unwrap();

    let result = read_settings(&base, "eco-legacy");
    assert_eq!(result["defaultMode"], "bypassPermissions", "迁移后重建应保留用户偏好");
    assert_eq!(result["language"], "中文", "迁移后重建应保留用户偏好");

    println!("✅ 旧版迁移测试通过！");
}