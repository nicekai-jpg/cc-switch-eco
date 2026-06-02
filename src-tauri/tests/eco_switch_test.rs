// 集成测试：验证 Eco 切换和 fragment 合并逻辑
//
// 运行方式: cargo test --test eco_switch_test -- --nocapture

use cc_switch_eco_lib::services::ecosystem::fragment;
use cc_switch_eco_lib::services::ecosystem::migration;
use serde_json::json;
use std::fs;
use std::path::Path;

/// 测试辅助：创建临时 Eco 目录结构
fn create_test_eco(
    base: &Path,
    eco_id: &str,
    frameworks: &[&str],
    settings_content: &serde_json::Value,
) {
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
    fs::write(
        eco_dir.join("eco.json"),
        serde_json::to_string_pretty(&eco_json).unwrap(),
    )
    .unwrap();

    // settings.json
    fs::write(
        rootfiles_dir.join("settings.json"),
        serde_json::to_string_pretty(settings_content).unwrap(),
    )
    .unwrap();
}

/// 测试辅助：创建 fragment 文件
fn create_fragment(
    base: &Path,
    eco_id: &str,
    file_name: &str,
    prefix: &str,
    content: &serde_json::Value,
) {
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
    create_test_eco(
        &base,
        "eco-a",
        &["ohmyclaudecode"],
        &json!({
            "defaultMode": "bypassPermissions",
            "effort": "high",
            "language": "English",
            "permissions": {"allow": ["Bash", "Read"], "deny": []}
        }),
    );

    // 为 eco-a 创建 omc fragment
    create_fragment(
        &base,
        "eco-a",
        "settings.json",
        "omc-",
        &json!({
            "defaultMode": "bypassPermissions",
            "effort": "high",
            "language": "English",
            "permissions": {"allow": ["Bash", "Read"], "deny": []}
        }),
    );

    // Eco B: ruflo 框架
    create_test_eco(
        &base,
        "eco-b",
        &["ruflo"],
        &json!({
            "defaultMode": "plan",
            "effort": "max",
            "language": "中文",
            "permissions": {"allow": ["Bash", "Write"], "deny": ["WebFetch"]}
        }),
    );

    // 为 eco-b 创建 ruflo fragment
    create_fragment(
        &base,
        "eco-b",
        "settings.json",
        "ruflo-",
        &json!({
            "defaultMode": "plan",
            "effort": "max",
            "language": "中文",
            "permissions": {"allow": ["Bash", "Write"], "deny": ["WebFetch"]}
        }),
    );

    println!("=== 初始状态 ===");
    println!(
        "eco-a settings: {}",
        serde_json::to_string_pretty(&read_settings(&base, "eco-a")).unwrap()
    );
    println!(
        "eco-b settings: {}",
        serde_json::to_string_pretty(&read_settings(&base, "eco-b")).unwrap()
    );

    // === 场景2: 用户在 eco-a 中修改了 language 为 "中文" ===
    let mut eco_a_settings = read_settings(&base, "eco-a");
    eco_a_settings["language"] = json!("中文");
    fs::write(
        base.join("eco-a/rootfiles/settings.json"),
        serde_json::to_string_pretty(&eco_a_settings).unwrap(),
    )
    .unwrap();

    println!("\n=== 用户修改 eco-a 的 language 为中文 ===");

    // === 场景3: 模拟 snapshot_user_preferences（切换前保存用户偏好）===
    let current_settings = read_settings(&base, "eco-a");
    create_fragment(&base, "eco-a", "settings.json", "user-", &current_settings);

    // === 场景4: rebuild_root_file（切换到 eco-a 时重建）===
    fragment::rebuild_root_file(
        &base.join("eco-a/rootfiles"),
        "settings.json",
        &["ohmyclaudecode".to_string()],
    )
    .unwrap();

    let rebuilt_a = read_settings(&base, "eco-a");
    println!("\n=== 重建后 eco-a settings ===");
    println!("{}", serde_json::to_string_pretty(&rebuilt_a).unwrap());

    // 验证：用户偏好 language=中文 应该覆盖框架默认 language=English
    assert_eq!(rebuilt_a["language"], "中文", "用户偏好 language 应该保留");
    assert_eq!(
        rebuilt_a["defaultMode"], "bypassPermissions",
        "defaultMode 应该保留"
    );
    assert_eq!(rebuilt_a["effort"], "high", "effort 应该保留");

    // === 场景5: eco-b 也保存用户偏好后重建 ===
    let eco_b_settings = read_settings(&base, "eco-b");
    create_fragment(&base, "eco-b", "settings.json", "user-", &eco_b_settings);

    fragment::rebuild_root_file(
        &base.join("eco-b/rootfiles"),
        "settings.json",
        &["ruflo".to_string()],
    )
    .unwrap();

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
    create_fragment(
        &base,
        "eco-multi",
        "settings.json",
        "omc-",
        &json!({
            "defaultMode": "bypassPermissions",
            "effort": "high",
            "language": "English",
            "permissions": {"allow": ["Bash", "Read"], "deny": []}
        }),
    );

    // ruflo fragment
    create_fragment(
        &base,
        "eco-multi",
        "settings.json",
        "ruflo-",
        &json!({
            "defaultMode": "plan",
            "effort": "max",
            "language": "中文",
            "permissions": {"allow": ["Bash", "Write", "Edit"], "deny": ["WebFetch"]}
        }),
    );

    // user-fragment: 用户偏好 defaultMode=bypassPermissions, language=中文
    create_fragment(
        &base,
        "eco-multi",
        "settings.json",
        "user-",
        &json!({
            "defaultMode": "bypassPermissions",
            "language": "中文"
        }),
    );

    // 重建
    fragment::rebuild_root_file(
        &base.join("eco-multi/rootfiles"),
        "settings.json",
        &["ohmyclaudecode".to_string(), "ruflo".to_string()],
    )
    .unwrap();

    let result = read_settings(&base, "eco-multi");
    println!(
        "多框架合并结果: {}",
        serde_json::to_string_pretty(&result).unwrap()
    );

    // 用户偏好优先
    assert_eq!(
        result["defaultMode"], "bypassPermissions",
        "用户偏好 defaultMode 应优先"
    );
    assert_eq!(result["language"], "中文", "用户偏好 language 应优先");
    // ruflo 最后覆盖框架值
    assert_eq!(result["effort"], "max", "effort 应为 ruflo 的 max");
    // 数组去重合并
    let allow = result["permissions"]["allow"].as_array().unwrap();
    let allow_strs: Vec<&str> = allow.iter().filter_map(|v| v.as_str()).collect();
    assert!(allow_strs.contains(&"Bash"), "Bash 应在 allow 中");
    assert!(
        allow_strs.contains(&"Read"),
        "Read 应在 allow 中（来自 omc）"
    );
    assert!(
        allow_strs.contains(&"Write"),
        "Write 应在 allow 中（来自 ruflo）"
    );
    assert!(
        allow_strs.contains(&"Edit"),
        "Edit 应在 allow 中（来自 ruflo）"
    );
    // deny
    assert_eq!(result["permissions"]["deny"], json!(["WebFetch"]));

    // 检查冲突记录
    let eco_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(base.join("eco-multi/eco.json")).unwrap())
            .unwrap();
    let conflicts = eco_json
        .get("mergeConflicts")
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
    create_test_eco(
        &base,
        "eco-legacy",
        &["ohmyclaudecode"],
        &json!({
            "defaultMode": "bypassPermissions",
            "effort": "high",
            "language": "中文"
        }),
    );

    // 运行迁移
    let isolation = fragment::EcoIsolation {
        dirs: vec![],
        files: vec!["settings.json".to_string()],
    };
    migration::migrate_legacy_rootfiles(&base.join("eco-legacy"), &isolation).unwrap();

    // 验证：旧版 settings.json 应该被保存为 user-fragment
    let user_frag_path = base.join("eco-legacy/rootfiles/settings.user-fragment.json");
    assert!(user_frag_path.exists(), "迁移后应创建 user-fragment");

    let user_frag: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&user_frag_path).unwrap()).unwrap();
    assert_eq!(user_frag["defaultMode"], "bypassPermissions");
    assert_eq!(user_frag["language"], "中文");

    // 重建后应保持用户偏好
    fragment::rebuild_root_file(
        &base.join("eco-legacy/rootfiles"),
        "settings.json",
        &["ohmyclaudecode".to_string()],
    )
    .unwrap();

    let result = read_settings(&base, "eco-legacy");
    assert_eq!(
        result["defaultMode"], "bypassPermissions",
        "迁移后重建应保留用户偏好"
    );
    assert_eq!(result["language"], "中文", "迁移后重建应保留用户偏好");

    println!("✅ 旧版迁移测试通过！");
}

/// 模拟 merge_hooks_json_to_fragment 的核心逻辑（测试 fragment 层面）
///
/// 验证：框架的 hooks/hooks.json 中的 hooks 配置
/// 被正确合并到 settings.<prefix>fragment.json 的 hooks 字段中。
fn simulate_merge_hooks_to_fragment(
    rootfiles_dir: &Path,
    prefix: &str,
    hooks_json: &serde_json::Value,
) {
    let hooks_field = hooks_json.get("hooks").cloned().unwrap_or(json!({}));
    let fragment_content = json!({ "hooks": hooks_field });

    let frag_path = fragment::fragment_path(rootfiles_dir, "settings.json", prefix);

    if frag_path.exists() {
        let existing = fs::read_to_string(&frag_path).unwrap();
        let mut existing_json: serde_json::Value = serde_json::from_str(&existing).unwrap();
        let mut conflicts = Vec::new();
        fragment::json_deep_merge_with_array_dedup(
            &mut existing_json,
            &fragment_content,
            "",
            prefix,
            &mut conflicts,
        );
        fs::write(
            &frag_path,
            serde_json::to_string_pretty(&existing_json).unwrap_or_default(),
        )
        .unwrap();
    } else {
        fs::write(
            &frag_path,
            serde_json::to_string_pretty(&fragment_content).unwrap_or_default(),
        )
        .unwrap();
    }
}

#[test]
fn test_hooks_json_merged_to_settings_fragment() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().to_path_buf();

    // === 场景1: 单框架 Eco，hooks.json 合并到空的 fragment ===
    create_test_eco(
        &base,
        "eco-omc",
        &["ohmyclaudecode"],
        &json!({
            "defaultMode": "bypassPermissions",
            "effort": "high"
        }),
    );

    // 模拟框架安装命令写入了 settings.json 的非 hooks 部分
    create_fragment(
        &base,
        "eco-omc",
        "settings.json",
        "omc-",
        &json!({
            "defaultMode": "bypassPermissions",
            "effort": "high"
        }),
    );

    // 模拟框架的 hooks/hooks.json
    let hooks_json = json!({
        "description": "OMC orchestration hooks",
        "hooks": {
            "SessionStart": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "node \"$CLAUDE_PLUGIN_ROOT\"/scripts/run.cjs \"$CLAUDE_PLUGIN_ROOT\"/scripts/session-start.mjs",
                            "timeout": 5
                        }
                    ]
                }
            ],
            "PreToolUse": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "node \"$CLAUDE_PLUGIN_ROOT\"/scripts/run.cjs \"$CLAUDE_PLUGIN_ROOT\"/scripts/pre-tool-enforcer.mjs",
                            "timeout": 3
                        }
                    ]
                }
            ]
        }
    });

    // 执行合并
    simulate_merge_hooks_to_fragment(&base.join("eco-omc/rootfiles"), "omc-", &hooks_json);

    // 重建 settings.json
    fragment::rebuild_root_file(
        &base.join("eco-omc/rootfiles"),
        "settings.json",
        &["ohmyclaudecode".to_string()],
    )
    .unwrap();

    let result = read_settings(&base, "eco-omc");
    println!(
        "场景1 - 单框架合并结果: {}",
        serde_json::to_string_pretty(&result).unwrap()
    );

    // 验证：settings.json 包含 hooks 字段
    assert!(
        result.get("hooks").is_some(),
        "settings.json 应包含 hooks 字段"
    );
    assert!(
        result["hooks"].get("SessionStart").is_some(),
        "hooks 应包含 SessionStart"
    );
    assert!(
        result["hooks"].get("PreToolUse").is_some(),
        "hooks 应包含 PreToolUse"
    );
    // 验证：非 hooks 配置也保留
    assert_eq!(result["defaultMode"], "bypassPermissions");
    assert_eq!(result["effort"], "high");

    // === 场景2: 多框架 Eco，hooks 配置合并（数组去重拼接）===
    create_test_eco(&base, "eco-multi", &["ohmyclaudecode", "ruflo"], &json!({}));

    // omc fragment（含 hooks）
    create_fragment(
        &base,
        "eco-multi",
        "settings.json",
        "omc-",
        &json!({
            "defaultMode": "bypassPermissions",
            "effort": "high",
            "hooks": {
                "SessionStart": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "node \"$CLAUDE_PLUGIN_ROOT\"/scripts/session-start.mjs",
                                "timeout": 5
                            }
                        ]
                    }
                ]
            }
        }),
    );

    // ruflo fragment（含不同的 hooks 事件）
    create_fragment(
        &base,
        "eco-multi",
        "settings.json",
        "ruflo-",
        &json!({
            "defaultMode": "plan",
            "effort": "max",
            "hooks": {
                "PostToolUse": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "node \"$CLAUDE_PLUGIN_ROOT\"/scripts/post-tool.mjs",
                                "timeout": 3
                            }
                        ]
                    }
                ]
            }
        }),
    );

    // 重建
    fragment::rebuild_root_file(
        &base.join("eco-multi/rootfiles"),
        "settings.json",
        &["ohmyclaudecode".to_string(), "ruflo".to_string()],
    )
    .unwrap();

    let result = read_settings(&base, "eco-multi");
    println!(
        "场景2 - 多框架合并结果: {}",
        serde_json::to_string_pretty(&result).unwrap()
    );

    // 验证：两个框架的 hooks 事件都存在
    assert!(
        result["hooks"].get("SessionStart").is_some(),
        "应包含 OMC 的 SessionStart"
    );
    assert!(
        result["hooks"].get("PostToolUse").is_some(),
        "应包含 Ruflo 的 PostToolUse"
    );

    // === 场景3: 框架没有 hooks/hooks.json，不影响现有行为 ===
    create_test_eco(
        &base,
        "eco-no-hooks",
        &["mattpocock-skills"],
        &json!({
            "defaultMode": "bypassPermissions"
        }),
    );

    create_fragment(
        &base,
        "eco-no-hooks",
        "settings.json",
        "mp-",
        &json!({
            "defaultMode": "bypassPermissions"
        }),
    );

    // 不调用 simulate_merge_hooks_to_fragment（模拟没有 hooks.json）

    fragment::rebuild_root_file(
        &base.join("eco-no-hooks/rootfiles"),
        "settings.json",
        &["mattpocock-skills".to_string()],
    )
    .unwrap();

    let result = read_settings(&base, "eco-no-hooks");
    println!(
        "场景3 - 无 hooks 框架: {}",
        serde_json::to_string_pretty(&result).unwrap()
    );

    assert_eq!(result["defaultMode"], "bypassPermissions");
    // 没有 hooks 也不应出错
    assert!(
        result.get("hooks").is_none() || result["hooks"].as_object().is_some_and(|o| o.is_empty()),
        "无 hooks 框架不应产生 hooks 字段"
    );

    println!("\n✅ hooks.json 合并到 settings fragment 测试通过！");
}

#[test]
fn test_hooks_merge_with_existing_fragment() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().to_path_buf();

    // 场景：框架安装命令已经写入了部分 settings.json（如 permissions），
    // 然后 hooks.json 合并进来，两者应深合并
    create_test_eco(&base, "eco-partial", &["ohmyclaudecode"], &json!({}));

    // 框架安装命令写入的 fragment（不含 hooks）
    create_fragment(
        &base,
        "eco-partial",
        "settings.json",
        "omc-",
        &json!({
            "defaultMode": "bypassPermissions",
            "permissions": {"allow": ["Bash", "Read"], "deny": []}
        }),
    );

    // 模拟 hooks.json 合并
    let hooks_json = json!({
        "hooks": {
            "SessionStart": [
                {
                    "matcher": "*",
                    "hooks": [{"type": "command", "command": "session-start.mjs", "timeout": 5}]
                }
            ]
        }
    });

    simulate_merge_hooks_to_fragment(&base.join("eco-partial/rootfiles"), "omc-", &hooks_json);

    // 验证 fragment 文件同时包含 permissions 和 hooks
    let frag_path =
        fragment::fragment_path(&base.join("eco-partial/rootfiles"), "settings.json", "omc-");
    let frag_content: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&frag_path).unwrap()).unwrap();

    println!(
        "合并后的 fragment: {}",
        serde_json::to_string_pretty(&frag_content).unwrap()
    );

    assert!(
        frag_content.get("permissions").is_some(),
        "fragment 应保留 permissions"
    );
    assert!(frag_content.get("hooks").is_some(), "fragment 应包含 hooks");
    assert_eq!(frag_content["defaultMode"], "bypassPermissions");

    // 重建 settings.json 验证最终结果
    fragment::rebuild_root_file(
        &base.join("eco-partial/rootfiles"),
        "settings.json",
        &["ohmyclaudecode".to_string()],
    )
    .unwrap();

    let result = read_settings(&base, "eco-partial");
    assert!(result.get("hooks").is_some());
    assert!(result.get("permissions").is_some());
    assert_eq!(result["defaultMode"], "bypassPermissions");

    println!("✅ hooks 与已有 fragment 深合并测试通过！");
}

#[test]
fn test_ruflo_claude_plugin_hooks_path() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().to_path_buf();

    // 模拟 ruflo 框架：hooks 在 .claude-plugin/hooks/hooks.json
    create_test_eco(
        &base,
        "eco-ruflo",
        &["ruflo"],
        &json!({
            "defaultMode": "plan",
            "effort": "max"
        }),
    );

    // ruflo fragment（非 hooks 部分）
    create_fragment(
        &base,
        "eco-ruflo",
        "settings.json",
        "ruflo-",
        &json!({
            "defaultMode": "plan",
            "effort": "max"
        }),
    );

    // 模拟 .claude-plugin/hooks/hooks.json（ruflo 的实际路径）
    let ruflo_hooks = json!({
        "hooks": {
            "PostToolUse": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "node \"$CLAUDE_PLUGIN_ROOT\"/scripts/ruflo-post-tool.mjs",
                            "timeout": 3
                        }
                    ]
                }
            ],
            "Stop": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "node \"$CLAUDE_PLUGIN_ROOT\"/scripts/ruflo-stop.mjs",
                            "timeout": 5
                        }
                    ]
                }
            ]
        }
    });

    // 使用 simulate_merge_hooks_to_fragment 模拟合并
    simulate_merge_hooks_to_fragment(&base.join("eco-ruflo/rootfiles"), "ruflo-", &ruflo_hooks);

    // 重建 settings.json
    fragment::rebuild_root_file(
        &base.join("eco-ruflo/rootfiles"),
        "settings.json",
        &["ruflo".to_string()],
    )
    .unwrap();

    let result = read_settings(&base, "eco-ruflo");
    println!(
        "ruflo hooks 合并结果: {}",
        serde_json::to_string_pretty(&result).unwrap()
    );

    // 验证 hooks 字段存在
    assert!(
        result.get("hooks").is_some(),
        "settings.json 应包含 hooks 字段"
    );
    assert!(
        result["hooks"].get("PostToolUse").is_some(),
        "hooks 应包含 PostToolUse"
    );
    assert!(result["hooks"].get("Stop").is_some(), "hooks 应包含 Stop");
    // 非 hooks 配置也保留
    assert_eq!(result["defaultMode"], "plan");
    assert_eq!(result["effort"], "max");

    println!("✅ ruflo .claude-plugin/hooks/hooks.json 路径测试通过！");
}

/// 测试两个多框架 Eco 之间互相切换
///
/// 场景：
/// - Eco A: 安装了 ohmyclaudecode + ruflo，各自有 hooks
/// - Eco B: 安装了 superpowers + mattpocock-skills，superpowers 有 hooks
/// - 切换 A → B → A，验证 hooks 不串、不丢、不重复
#[test]
fn test_multi_framework_eco_switch_back_and_forth() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().to_path_buf();

    // === Eco A: omc + ruflo ===
    create_test_eco(&base, "eco-a", &["ohmyclaudecode", "ruflo"], &json!({}));

    // omc fragment（含 hooks）
    create_fragment(
        &base,
        "eco-a",
        "settings.json",
        "omc-",
        &json!({
            "defaultMode": "bypassPermissions",
            "effort": "high",
            "language": "English",
            "permissions": {"allow": ["Bash", "Read"], "deny": []},
            "hooks": {
                "SessionStart": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "node \"$CLAUDE_PLUGIN_ROOT\"/scripts/session-start.mjs",
                                "timeout": 5
                            }
                        ]
                    }
                ],
                "PreToolUse": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "node \"$CLAUDE_PLUGIN_ROOT\"/scripts/pre-tool-enforcer.mjs",
                                "timeout": 3
                            }
                        ]
                    }
                ]
            }
        }),
    );

    // ruflo fragment（含 hooks）
    create_fragment(
        &base,
        "eco-a",
        "settings.json",
        "ruflo-",
        &json!({
            "defaultMode": "plan",
            "effort": "max",
            "language": "中文",
            "permissions": {"allow": ["Bash", "Write", "Edit"], "deny": ["WebFetch"]},
            "hooks": {
                "PostToolUse": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "node \"$CLAUDE_PLUGIN_ROOT\"/scripts/ruflo-post-tool.mjs",
                                "timeout": 3
                            }
                        ]
                    }
                ],
                "Stop": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "node \"$CLAUDE_PLUGIN_ROOT\"/scripts/ruflo-stop.mjs",
                                "timeout": 5
                            }
                        ]
                    }
                ]
            }
        }),
    );

    // 重建 Eco A 的 settings.json
    fragment::rebuild_root_file(
        &base.join("eco-a/rootfiles"),
        "settings.json",
        &["ohmyclaudecode".to_string(), "ruflo".to_string()],
    )
    .unwrap();

    let settings_a_initial = read_settings(&base, "eco-a");
    println!("=== Eco A 初始 settings ===");
    println!(
        "{}",
        serde_json::to_string_pretty(&settings_a_initial).unwrap()
    );

    // 验证 Eco A 初始状态
    assert!(
        settings_a_initial["hooks"].get("SessionStart").is_some(),
        "Eco A 应有 SessionStart"
    );
    assert!(
        settings_a_initial["hooks"].get("PreToolUse").is_some(),
        "Eco A 应有 PreToolUse"
    );
    assert!(
        settings_a_initial["hooks"].get("PostToolUse").is_some(),
        "Eco A 应有 PostToolUse"
    );
    assert!(
        settings_a_initial["hooks"].get("Stop").is_some(),
        "Eco A 应有 Stop"
    );

    // === Eco B: superpowers + mattpocock-skills ===
    create_test_eco(
        &base,
        "eco-b",
        &["superpowers", "mattpocock-skills"],
        &json!({}),
    );

    // superpowers fragment（含 hooks）
    create_fragment(
        &base,
        "eco-b",
        "settings.json",
        "superpowers-",
        &json!({
            "defaultMode": "bypassPermissions",
            "effort": "max",
            "language": "中文",
            "permissions": {"allow": ["Bash", "Read", "Write", "Edit"], "deny": []},
            "hooks": {
                "UserPromptSubmit": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "node \"$CLAUDE_PLUGIN_ROOT\"/scripts/keyword-detector.mjs",
                                "timeout": 5
                            }
                        ]
                    }
                ],
                "SessionStart": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "node \"$CLAUDE_PLUGIN_ROOT\"/scripts/sp-session-start.mjs",
                                "timeout": 5
                            }
                        ]
                    }
                ]
            }
        }),
    );

    // mattpocock-skills fragment（无 hooks）
    create_fragment(
        &base,
        "eco-b",
        "settings.json",
        "mp-",
        &json!({
            "permissions": {"allow": ["Bash", "Read"], "deny": []}
        }),
    );

    // 重建 Eco B 的 settings.json
    fragment::rebuild_root_file(
        &base.join("eco-b/rootfiles"),
        "settings.json",
        &["superpowers".to_string(), "mattpocock-skills".to_string()],
    )
    .unwrap();

    let settings_b_initial = read_settings(&base, "eco-b");
    println!("\n=== Eco B 初始 settings ===");
    println!(
        "{}",
        serde_json::to_string_pretty(&settings_b_initial).unwrap()
    );

    // 验证 Eco B 初始状态
    assert!(
        settings_b_initial["hooks"]
            .get("UserPromptSubmit")
            .is_some(),
        "Eco B 应有 UserPromptSubmit"
    );
    assert!(
        settings_b_initial["hooks"].get("SessionStart").is_some(),
        "Eco B 应有 SessionStart"
    );
    // Eco B 不应有 Eco A 的 hooks
    assert!(
        settings_b_initial["hooks"].get("PreToolUse").is_none(),
        "Eco B 不应有 PreToolUse（来自 Eco A 的 omc）"
    );
    assert!(
        settings_b_initial["hooks"].get("PostToolUse").is_none(),
        "Eco B 不应有 PostToolUse（来自 Eco A 的 ruflo）"
    );
    assert!(
        settings_b_initial["hooks"].get("Stop").is_none(),
        "Eco B 不应有 Stop（来自 Eco A 的 ruflo）"
    );

    // === 模拟切换 A → B：先 snapshot Eco A 的用户偏好 ===
    let snapshot_a = read_settings(&base, "eco-a");
    create_fragment(&base, "eco-a", "settings.json", "user-", &snapshot_a);

    // 重建 Eco B（模拟切换到 B）
    fragment::rebuild_root_file(
        &base.join("eco-b/rootfiles"),
        "settings.json",
        &["superpowers".to_string(), "mattpocock-skills".to_string()],
    )
    .unwrap();

    let settings_b_after_switch = read_settings(&base, "eco-b");
    println!("\n=== 切换到 Eco B 后 ===");
    println!(
        "{}",
        serde_json::to_string_pretty(&settings_b_after_switch).unwrap()
    );

    // Eco B 应该只有自己的 hooks
    assert!(settings_b_after_switch["hooks"]
        .get("UserPromptSubmit")
        .is_some());
    assert!(settings_b_after_switch["hooks"]
        .get("SessionStart")
        .is_some());
    assert!(
        settings_b_after_switch["hooks"].get("PreToolUse").is_none(),
        "Eco B 不应串入 Eco A 的 PreToolUse"
    );
    assert!(
        settings_b_after_switch["hooks"]
            .get("PostToolUse")
            .is_none(),
        "Eco B 不应串入 Eco A 的 PostToolUse"
    );

    // === 模拟切换 B → A：先 snapshot Eco B 的用户偏好 ===
    let snapshot_b = read_settings(&base, "eco-b");
    create_fragment(&base, "eco-b", "settings.json", "user-", &snapshot_b);

    // 重建 Eco A（模拟切换回 A）
    fragment::rebuild_root_file(
        &base.join("eco-a/rootfiles"),
        "settings.json",
        &["ohmyclaudecode".to_string(), "ruflo".to_string()],
    )
    .unwrap();

    let settings_a_after_switch_back = read_settings(&base, "eco-a");
    println!("\n=== 切换回 Eco A 后 ===");
    println!(
        "{}",
        serde_json::to_string_pretty(&settings_a_after_switch_back).unwrap()
    );

    // Eco A 应该只有自己的 hooks
    assert!(
        settings_a_after_switch_back["hooks"]
            .get("SessionStart")
            .is_some(),
        "Eco A 应有 SessionStart"
    );
    assert!(
        settings_a_after_switch_back["hooks"]
            .get("PreToolUse")
            .is_some(),
        "Eco A 应有 PreToolUse"
    );
    assert!(
        settings_a_after_switch_back["hooks"]
            .get("PostToolUse")
            .is_some(),
        "Eco A 应有 PostToolUse"
    );
    assert!(
        settings_a_after_switch_back["hooks"].get("Stop").is_some(),
        "Eco A 应有 Stop"
    );
    // Eco A 不应有 Eco B 的 hooks
    assert!(
        settings_a_after_switch_back["hooks"]
            .get("UserPromptSubmit")
            .is_none(),
        "Eco A 不应串入 Eco B 的 UserPromptSubmit"
    );

    // === 关键验证：hooks 数组不应重复 ===
    // user-fragment 包含了之前合并后的完整 hooks，重建时框架 fragment + user-fragment 再次合并
    // 数组去重应该防止重复条目
    let session_start_hooks = settings_a_after_switch_back["hooks"]["SessionStart"]
        .as_array()
        .unwrap();
    // SessionStart 应该只有 omc 的条目（1个 matcher 组）
    assert_eq!(
        session_start_hooks.len(),
        1,
        "SessionStart 不应重复，实际有 {} 个条目",
        session_start_hooks.len()
    );

    let pre_tool_hooks = settings_a_after_switch_back["hooks"]["PreToolUse"]
        .as_array()
        .unwrap();
    assert_eq!(
        pre_tool_hooks.len(),
        1,
        "PreToolUse 不应重复，实际有 {} 个条目",
        pre_tool_hooks.len()
    );

    let post_tool_hooks = settings_a_after_switch_back["hooks"]["PostToolUse"]
        .as_array()
        .unwrap();
    assert_eq!(
        post_tool_hooks.len(),
        1,
        "PostToolUse 不应重复，实际有 {} 个条目",
        post_tool_hooks.len()
    );

    // === 再切换一次 B → A → B，验证稳定性 ===
    // snapshot Eco A
    let snapshot_a2 = read_settings(&base, "eco-a");
    create_fragment(&base, "eco-a", "settings.json", "user-", &snapshot_a2);

    // 重建 Eco B
    fragment::rebuild_root_file(
        &base.join("eco-b/rootfiles"),
        "settings.json",
        &["superpowers".to_string(), "mattpocock-skills".to_string()],
    )
    .unwrap();

    let settings_b_second_switch = read_settings(&base, "eco-b");
    println!("\n=== 第二次切换到 Eco B 后 ===");
    println!(
        "{}",
        serde_json::to_string_pretty(&settings_b_second_switch).unwrap()
    );

    // Eco B 仍然只有自己的 hooks
    assert!(settings_b_second_switch["hooks"]
        .get("UserPromptSubmit")
        .is_some());
    assert!(settings_b_second_switch["hooks"]
        .get("SessionStart")
        .is_some());
    assert!(
        settings_b_second_switch["hooks"]
            .get("PreToolUse")
            .is_none(),
        "Eco B 不应串入 Eco A 的 hooks"
    );
    assert!(
        settings_b_second_switch["hooks"]
            .get("PostToolUse")
            .is_none(),
        "Eco B 不应串入 Eco A 的 hooks"
    );

    // Eco B 的 hooks 不应重复
    let sp_session_start = settings_b_second_switch["hooks"]["SessionStart"]
        .as_array()
        .unwrap();
    assert_eq!(
        sp_session_start.len(),
        1,
        "Eco B SessionStart 不应重复，实际有 {} 个条目",
        sp_session_start.len()
    );

    println!("\n✅ 多框架 Eco 互相切换测试通过！");
}

/// 测试同一 Eco 内两个框架共享同一个 hook 事件名但命令不同
///
/// 场景：omc 和 superpowers 都有 SessionStart hook，但命令不同
/// 合并后应该有两个 SessionStart 条目（数组去重拼接）
/// 切换回来后，user-fragment 包含合并后的两个条目，重建时不应变成 4 个
#[test]
fn test_shared_hook_event_different_commands() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().to_path_buf();

    // === Eco: omc + superpowers（都有 SessionStart）===
    create_test_eco(
        &base,
        "eco-shared",
        &["ohmyclaudecode", "superpowers"],
        &json!({}),
    );

    // omc fragment（含 SessionStart）
    create_fragment(
        &base,
        "eco-shared",
        "settings.json",
        "omc-",
        &json!({
            "defaultMode": "bypassPermissions",
            "hooks": {
                "SessionStart": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "node \"$CLAUDE_PLUGIN_ROOT\"/scripts/omc-session-start.mjs",
                                "timeout": 5
                            }
                        ]
                    }
                ]
            }
        }),
    );

    // superpowers fragment（也有 SessionStart，但命令不同）
    create_fragment(
        &base,
        "eco-shared",
        "settings.json",
        "superpowers-",
        &json!({
            "defaultMode": "plan",
            "hooks": {
                "SessionStart": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "node \"$CLAUDE_PLUGIN_ROOT\"/scripts/sp-session-start.mjs",
                                "timeout": 5
                            }
                        ]
                    }
                ]
            }
        }),
    );

    // 重建 settings.json
    fragment::rebuild_root_file(
        &base.join("eco-shared/rootfiles"),
        "settings.json",
        &["ohmyclaudecode".to_string(), "superpowers".to_string()],
    )
    .unwrap();

    let settings = read_settings(&base, "eco-shared");
    println!("=== 两个框架都有 SessionStart，合并后 ===");
    println!(
        "{}",
        serde_json::to_string_pretty(&settings["hooks"]).unwrap()
    );

    // 验证：SessionStart 应该有 2 个条目（omc 的 + superpowers 的）
    let session_start = settings["hooks"]["SessionStart"].as_array().unwrap();
    assert_eq!(
        session_start.len(),
        2,
        "SessionStart 应有 2 个条目（来自两个框架），实际有 {}",
        session_start.len()
    );

    // 验证两个条目的命令不同
    let commands: Vec<&str> = session_start
        .iter()
        .filter_map(|entry| {
            entry
                .get("hooks")?
                .as_array()?
                .first()?
                .get("command")?
                .as_str()
        })
        .collect();
    assert!(
        commands.iter().any(|c| c.contains("omc-session-start")),
        "应有 omc 的 SessionStart 命令"
    );
    assert!(
        commands.iter().any(|c| c.contains("sp-session-start")),
        "应有 superpowers 的 SessionStart 命令"
    );

    // === 模拟切换：snapshot 用户偏好 ===
    let snapshot = read_settings(&base, "eco-shared");
    create_fragment(&base, "eco-shared", "settings.json", "user-", &snapshot);

    // 重建（模拟切换回来）
    fragment::rebuild_root_file(
        &base.join("eco-shared/rootfiles"),
        "settings.json",
        &["ohmyclaudecode".to_string(), "superpowers".to_string()],
    )
    .unwrap();

    let settings_after_switch = read_settings(&base, "eco-shared");
    println!("\n=== 切换回来后，SessionStart ===");
    println!(
        "{}",
        serde_json::to_string_pretty(&settings_after_switch["hooks"]).unwrap()
    );

    // 关键验证：SessionStart 仍然只有 2 个条目，不会变成 4 个
    let session_start_after = settings_after_switch["hooks"]["SessionStart"]
        .as_array()
        .unwrap();
    assert_eq!(
        session_start_after.len(),
        2,
        "切换后 SessionStart 仍应有 2 个条目，实际有 {}（可能重复了）",
        session_start_after.len()
    );

    println!("\n✅ 共享 hook 事件名测试通过！");
}
