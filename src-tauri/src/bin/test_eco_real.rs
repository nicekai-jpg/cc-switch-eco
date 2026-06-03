use cc_switch_eco_lib::Database;
use cc_switch_eco_lib::AppState;
use cc_switch_eco_lib::services::ecosystem::EcosystemService;
use std::fs;
use std::process::Command;
use std::sync::Arc;

fn main() {
    println!("========================================================");
    println!("Starting REAL E2E Integration Test with Claude Code");
    println!("========================================================");

    // 1. 初始化数据库与状态 (使用真实家目录下的数据库)
    let db = Database::init().expect("Failed to initialize database");
    let state = AppState::new(Arc::new(db));

    // 2. 备份当前的活跃生态，以便测试结束后恢复
    let original_eco = EcosystemService::get_current(&state).ok().flatten();
    if let Some(ref eco) = original_eco {
        println!("Original active ecosystem detected: {} (ID: {})", eco.name, eco.id);
    } else {
        println!("No active ecosystem detected initially.");
    }

    // 3. 定义要测试的框架列表以及要验证的 Claude Code 命令输出特征
    // 特征可以是包含的文件名前缀，或者在 `claude -p "/help"` 里的 slash 命令前缀
    let test_cases = vec![
        ("superpowers", "Superpowers 中文版", vec!["superpowers"]),
        ("agency-agents-zh", "Agency Agents 中文版", vec!["agency"]),
        ("ohmyclaudecode", "Oh My ClaudeCode", vec!["omc-"]),
        ("ruflo", "Ruflo", vec!["ruflo", "claude-flow"]),
        ("speckit", "Spec Kit", vec!["speckit"]),
        ("mattpocock-skills", "Matt Pocock Skills", vec!["mp-"]),
        ("gstack", "GStack", vec!["gstack-"]),
        ("openspec", "OpenSpec", vec!["opsx:", "init"]),
        ("bmad-method", "BMAD-METHOD", vec!["bmad-"]),
        ("get-shit-done", "Get Shit Done", vec!["gsd-"]),
    ];

    let mut success_count = 0;

    for (fw_id, fw_name, expected_features) in test_cases {
        let test_eco_id = format!("test-eco-{}", fw_id);
        println!("\n--------------------------------------------------------");
        println!("E2E Testing Framework: {} ({})", fw_name, fw_id);
        println!("--------------------------------------------------------");

        // 如果测试生态已存在，先删除它以确保全新安装
        if EcosystemService::get_ecosystem_frameworks(&test_eco_id).is_ok() {
            println!("Cleaning up existing test ecosystem: {}...", test_eco_id);
            let _ = EcosystemService::delete(&state, &test_eco_id);
        }

        // A. 创建生态并同步运行官方推荐的安装指令
        println!("[1/4] Creating ecosystem and installing framework...");
        let eco = EcosystemService::create(
            &state,
            &test_eco_id,
            &format!("E2E integration test for {}", fw_name),
            vec![fw_id.to_string()],
        );

        if let Err(e) = eco {
            println!("  [ERROR] Failed to create ecosystem: {}", e);
            continue;
        }

        // B. 切换生态（重新建立指向 ~/.claude/ 的软链接）
        println!("[2/4] Switching to test ecosystem {}...", test_eco_id);
        if let Err(e) = EcosystemService::switch(&state, &test_eco_id) {
            println!("  [ERROR] Failed to switch to ecosystem: {}", e);
            continue;
        }

        // C. 调用真实 Claude Code 命令进行验证
        println!("[3/4] Invoking Claude Code to verify features...");
        let (verified, log_msg) = verify_with_claude_cli(&test_eco_id, fw_id, &expected_features);
        if verified {
            println!("  [OK] Verification succeeded: {}", log_msg);
            success_count += 1;
        } else {
            println!("  [FAIL] Verification failed: {}", log_msg);
        }

        // D. 清理测试生态
        println!("[4/4] Cleaning up test ecosystem {}...", test_eco_id);
        let _ = EcosystemService::delete(&state, &test_eco_id);
    }

    // 4. 恢复之前的活跃生态
    println!("\n--------------------------------------------------------");
    println!("Test completed. Restoring state...");
    if let Some(eco) = original_eco {
        println!("Restoring original ecosystem: {} (ID: {})", eco.name, eco.id);
        let _ = EcosystemService::switch(&state, &eco.id);
    } else {
        println!("No original ecosystem to restore.");
    }
    println!("--------------------------------------------------------");

    println!("\nE2E Integration Test Finished. {}/10 frameworks verified successfully.", success_count);
    if success_count == 10 {
        println!("ALL TESTS PASSED!");
    } else {
        println!("SOME TESTS FAILED. Please review the logs above.");
    }
}

/// 在真实环境下运行 claude 命令，验证框架的功能是否加载并可被 claude 识别
fn verify_with_claude_cli(_eco_id: &str, fw_id: &str, features: &[&str]) -> (bool, String) {
    // 运行 `claude -p "/help"` 打印可用斜杠命令
    let output = Command::new("claude")
        .args(["-p", "/help"])
        .output();

    let stdout_content = match output {
        Ok(out) => {
            if out.status.success() {
                String::from_utf8_lossy(&out.stdout).to_string()
            } else {
                String::from_utf8_lossy(&out.stderr).to_string()
            }
        }
        Err(e) => return (false, format!("Failed to run `claude` CLI: {}", e)),
    };

    // 验证特定框架的标志特征在 Claude Code 帮助中已加载
    match fw_id {
        "superpowers" | "agency-agents-zh" | "mattpocock-skills" | "gstack" | "bmad-method" => {
            // 这些主要是 Skills（.md 提示词文件），被 Claude Code 静默加载，但不在帮助文档中列为 slash 独立命令。
            // 故验证在对应的生态技能目录下已正确克隆并生成带有特定前缀/名字的 .md 文件。
            let claude_home = dirs::home_dir().unwrap().join(".claude");
            let skills_dir = claude_home.join("skills");
            let agents_dir = claude_home.join("agents");

            let mut has_feature = false;
            let dir_to_check = if fw_id == "agency-agents-zh" { &agents_dir } else { &skills_dir };

            if dir_to_check.exists() {
                if let Ok(entries) = fs::read_dir(dir_to_check) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        for feature in features {
                            if name.contains(feature) {
                                has_feature = true;
                                break;
                            }
                        }
                    }
                }
            }

            if has_feature {
                (true, format!("Successfully verified skill files cloned to ~/.claude/ for {}", fw_id))
            } else {
                (false, format!("No expected files containing {:?} in target directory for {}", features, fw_id))
            }
        }
        "ruflo" => {
            // Ruflo 主要验证其 MCP 关联
            let output_mcp = Command::new("claude")
                .args(["mcp", "list"])
                .output();
            
            let mcp_content = match output_mcp {
                Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
                Err(_) => "".to_string(),
            };

            if mcp_content.contains("ruflo") || mcp_content.contains("claude-flow") {
                (true, "Ruflo MCP server detected in `claude mcp list`".to_string())
            } else {
                (false, "Ruflo MCP server NOT found in `claude mcp list`".to_string())
            }
        }
        _ => {
            // 其他框架提供自定义命令文件，应该显示在 Claude Code 的 `/help` 结果中
            let mut found_feature = false;
            let mut detected_cmd = String::new();
            for feature in features {
                if stdout_content.contains(feature) {
                    found_feature = true;
                    detected_cmd = feature.to_string();
                    break;
                }
            }

            if found_feature {
                (true, format!("Slash command containing '{}' is loaded and visible in Claude Code /help", detected_cmd))
            } else {
                (false, format!("No slash command matching features {:?} found in Claude Code /help output. Output:\n{}", features, stdout_content))
            }
        }
    }
}
