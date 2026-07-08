use std::fs;
use std::path::Path;

use crate::error::AppError;
use crate::services::ecosystem::fragment;
use crate::services::ecosystem::cmd_utils::{command_exists, get_command_path};

/// 自动完成 Claude HUD 的 setup 配置
///
/// HUD 的 /claude-hud:setup 命令需要用户手动运行，这里在安装时自动完成：
/// 1. 检测 runtime（优先 bun，回退 node）
/// 2. 生成 statusLine 命令并写入 HUD 的 settings fragment
/// 3. 创建 ~/.claude/plugins/claude-hud/config.json 默认配置
pub fn auto_setup_hud(
    eco_dir: &Path,
    cache_install_path: &Path,
) -> Result<(), AppError> {
    // 1. 检测 runtime：优先 bun（更快），回退 node
    let runtime = if command_exists("bun") {
        get_command_path("bun")
    } else if command_exists("node") {
        get_command_path("node")
    } else {
        log::warn!("HUD auto-setup: 未找到 bun 或 node，跳过 statusLine 配置");
        return Ok(());
    };

    let runtime_path = match runtime {
        Some(p) => p,
        None => {
            log::warn!("HUD auto-setup: 无法获取 runtime 路径，跳过 statusLine 配置");
            return Ok(());
        }
    };

    // 2. 确定源文件：bun 用 src/index.ts，node 用 dist/index.js
    let use_bun = runtime_path.contains("bun");
    let source = if use_bun {
        "src/index.ts"
    } else {
        "dist/index.js"
    };

    // 检查源文件是否存在于 cache 目录
    let source_file = cache_install_path.join(source);
    if !source_file.exists() {
        log::warn!(
            "HUD auto-setup: 源文件 {} 不存在于 {}，跳过 statusLine 配置",
            source,
            cache_install_path.display()
        );
        return Ok(());
    }

    // 3. 生成 statusLine 命令
    // macOS/Linux 格式（与 HUD setup.md Step 1 一致）
    // 使用动态版本查找，这样插件更新后无需重新 setup
    //
    // 原始命令中的 '"'"' 是 bash 单引号嵌套技巧：
    // 结束当前单引号 → 双引号包裹一个单引号 → 重新开始单引号
    // 在 Rust 字符串中直接写 '"'"' 即可
    let statusline_command = if use_bun {
        format!(
            "bash -c 'cols=${{COLUMNS:-}}; case \"$cols\" in \"\"|*[!0-9]*) cols=$(stty size </dev/tty 2>/dev/null | awk '\"'\"'{{print $2}}'\"'\"');; esac; case \"$cols\" in \"\"|*[!0-9]*) cols=120;; esac; export COLUMNS=$(( cols > 4 ? cols - 4 : 1 )); \
             plugin_dir=$(ls -d \"${{CLAUDE_CONFIG_DIR:-$HOME/.claude}}\"/plugins/cache/*/claude-hud/*/ 2>/dev/null | \
               awk -F/ '\"'\"'{{ print $(NF-1) \"\\t\" $(0) }}'\"'\"' | \
               grep -E '\"'\"'^[0-9]+\\.[0-9]+\\.[0-9]+[[:space:]]'\"'\"' | \
               sort -t. -k1,1n -k2,2n -k3,3n -k4,4n | tail -1 | cut -f2-); \
             exec \"{runtime_path}\" --env-file /dev/null \"${{plugin_dir}}{source}\"'"
        )
    } else {
        format!(
            "bash -c 'cols=${{COLUMNS:-}}; case \"$cols\" in \"\"|*[!0-9]*) cols=$(stty size </dev/tty 2>/dev/null | awk '\"'\"'{{print $2}}'\"'\"');; esac; case \"$cols\" in \"\"|*[!0-9]*) cols=120;; esac; export COLUMNS=$(( cols > 4 ? cols - 4 : 1 )); \
             plugin_dir=$(ls -d \"${{CLAUDE_CONFIG_DIR:-$HOME/.claude}}\"/plugins/cache/*/claude-hud/*/ 2>/dev/null | \
               awk -F/ '\"'\"'{{ print $(NF-1) \"\\t\" $(0) }}'\"'\"' | \
               grep -E '\"'\"'^[0-9]+\\.[0-9]+\\.[0-9]+[[:space:]]'\"'\"' | \
               sort -t. -k1,1n -k2,2n -k3,3n -k4,4n | tail -1 | cut -f2-); \
             exec \"{runtime_path}\" \"${{plugin_dir}}{source}\"'"
        )
    };

    // 4. 将 statusLine 写入 HUD 的 settings fragment
    // 这样 rebuild_all_root_files 会自动合并 statusLine 到 settings.json
    // 避免直接写入 ~/.claude/settings.json 导致 symlink 被破坏或 fragment 重建时丢失
    let rootfiles_dir = eco_dir.join("rootfiles");
    fs::create_dir_all(&rootfiles_dir).map_err(|e| AppError::io(&rootfiles_dir, e))?;

    // 确保 settings.json 在隔离列表中
    let isolation = fragment::collect_eco_isolation(eco_dir);
    if !isolation.files.contains(&"settings.json".to_string()) {
        let mut updated_files = isolation.files.clone();
        updated_files.push("settings.json".to_string());
        let updated_isolation = fragment::EcoIsolation {
            dirs: isolation.dirs,
            files: updated_files,
        };
        fragment::update_eco_json_isolation(eco_dir, &updated_isolation)?;
    }

    let hud_prefix = "hud-";
    let frag_path = fragment::fragment_path(&rootfiles_dir, "settings.json", hud_prefix);

    let statusline_fragment = serde_json::json!({
        "statusLine": {
            "type": "command",
            "command": statusline_command
        }
    });

    // 合并到已有 fragment（保留其他 HUD 写入的 settings 配置）
    if frag_path.exists() {
        let existing = fs::read_to_string(&frag_path).map_err(|e| AppError::io(&frag_path, e))?;
        let mut existing_json: serde_json::Value =
            fragment::parse_json(&existing, "解析 HUD fragment 失败")?;

        let mut conflicts = Vec::new();
        fragment::json_deep_merge_with_array_dedup(
            &mut existing_json,
            &statusline_fragment,
            "",
            hud_prefix,
            &mut conflicts,
        );

        fs::write(&frag_path, fragment::write_json(&existing_json)?)
            .map_err(|e| AppError::io(&frag_path, e))?;
    } else {
        fs::write(&frag_path, fragment::write_json(&statusline_fragment)?)
            .map_err(|e| AppError::io(&frag_path, e))?;
    }

    // 从 fragment 重建 settings.json
    fragment::rebuild_all_root_files(eco_dir)?;

    log::info!(
        "HUD auto-setup: 已写入 statusLine fragment (runtime: {}, source: {})",
        runtime_path,
        source
    );

    // 5. 创建 config.json 默认配置
    // HUD 读取 ~/.claude/plugins/claude-hud/config.json
    // ~/.claude/plugins/ 是 symlink → eco_dir/plugins/
    // 所以写入 eco_dir/plugins/claude-hud/config.json 即可
    let hud_config_dir = eco_dir.join("plugins").join("claude-hud");
    fs::create_dir_all(&hud_config_dir).map_err(|e| AppError::io(&hud_config_dir, e))?;

    let config_path = hud_config_dir.join("config.json");
    let full_config = serde_json::json!({
        "language": "en",
        "lineLayout": "expanded",
        "showSeparators": true,
        "gitStatus": {
            "enabled": true,
            "showDirty": true,
            "showAheadBehind": true,
            "showFileStats": true
        },
        "display": {
            "showModel": true,
            "showContextBar": true,
            "contextValue": "both",
            "showTools": true,
            "showSkills": true,
            "showMcp": true,
            "showAgents": true,
            "showTodos": true,
            "showProject": true,
            "showAddedDirs": true,
            "showConfigCounts": true,
            "showTokenBreakdown": true,
            "showSpeed": true,
            "showUsage": true,
            "usageValue": "percent",
            "usageBarEnabled": true,
            "usageCompact": false,
            "showResetLabel": true,
            "showCost": true,
            "showDuration": true,
            "showSessionName": true,
            "showSessionTokens": true,
            "showSessionStartDate": true,
            "showLastResponseAt": true,
            "showEffortLevel": true,
            "showOutputStyle": true,
            "showMemoryUsage": false,
            "showPromptCache": true,
            "showClaudeCodeVersion": true,
            "showCompactions": true,
            "showAdvisor": true,
            "showProvider": true,
            "modelFormat": "compact"
        }
    });

    let config_content = fragment::write_json(&full_config)?;
    fs::write(&config_path, config_content).map_err(|e| AppError::io(&config_path, e))?;
    log::info!("HUD auto-setup: 已写入 full 模式 config.json");

    Ok(())
}

/// 清理 HUD 的 statusLine fragment 和 config.json
///
/// 卸载 claude-hud 时需要：
/// 1. 删除 HUD 的 settings fragment（rootfiles/settings.hud-fragment.json）
/// 2. 重建 settings.json（statusLine 会自动消失）
/// 3. 删除 ~/.claude/plugins/claude-hud/config.json（通过 symlink 在 eco 目录下）
pub fn cleanup_hud_settings(eco_dir: &Path) -> Result<(), AppError> {
    // 删除 HUD 的 settings fragment
    let rootfiles_dir = eco_dir.join("rootfiles");
    let hud_frag = fragment::fragment_path(&rootfiles_dir, "settings.json", "hud-");
    if hud_frag.exists() {
        fs::remove_file(&hud_frag).map_err(|e| AppError::io(&hud_frag, e))?;
    }

    // 重建 settings.json（statusLine 会自动消失）
    fragment::rebuild_all_root_files(eco_dir)?;

    // 移除 config.json（在 eco 的 plugins 目录下，通过 symlink 映射到 ~/.claude/plugins/）
    let hud_config_dir = eco_dir.join("plugins").join("claude-hud");
    let config_path = hud_config_dir.join("config.json");
    if config_path.exists() {
        if let Err(e) = fs::remove_file(&config_path) {
            log::warn!("清理 HUD config.json 失败 {}: {e}", config_path.display());
        }
    }

    log::info!("已清理 HUD statusLine fragment 和 config.json");
    Ok(())
}
