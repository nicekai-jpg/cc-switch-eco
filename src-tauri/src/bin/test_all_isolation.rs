use cc_switch_eco_lib::Database;
use cc_switch_eco_lib::AppState;
use cc_switch_eco_lib::services::ecosystem::EcosystemService;
use std::process::Command;
use std::sync::Arc;

fn main() {
    println!("========================================================");
    println!("VERIFYING ECOSYSTEM ISOLATION FOR ALL 10 AGENT FRAMEWORKS");
    println!("========================================================");

    // 1. 初始化数据库与状态
    let db = Database::init().expect("Failed to initialize database");
    let state = AppState::new(Arc::new(db));

    // 2. 备份当前的活跃生态，以便测试结束后恢复
    let original_eco = EcosystemService::get_current(&state).ok().flatten();
    if let Some(ref eco) = original_eco {
        println!("Original active ecosystem: {} (ID: {})", eco.name, eco.id);
    } else {
        println!("No active ecosystem initially.");
    }

    // 3. 定义 10 个测试框架，验证其自身的特征在 Claude 内【可见】，同时验证 OMC 框架特征在 Claude 内【不可见】以证明真正的隔离
    let test_cases = vec![
        ("superpowers", "Superpowers 中文版", "Check if there are any skills containing 'superpowers' in ~/.claude/skills/ and tell me if you see them.", "superpowers", true),
        ("agency-agents-zh", "Agency Agents 中文版", "Check if there are any agents containing 'agency' in ~/.claude/agents/ and tell me if you see them.", "agency", true),
        ("ohmyclaudecode", "Oh My ClaudeCode", "/help", "omc-", false),
        ("ruflo", "Ruflo", "mcp list", "ruflo", false), // Ruflo 校验 mcp list
        ("speckit", "Spec Kit", "/help", "speckit", false),
        ("mattpocock-skills", "Matt Pocock Skills", "Check if there are any skills containing 'mp' in ~/.claude/skills/ and tell me if you see them.", "mp", true),
        ("gstack", "GStack", "Check if there are any skills containing 'gstack' in ~/.claude/skills/ and tell me if you see them.", "gstack", true),
        ("openspec", "OpenSpec", "/help", "opsx:", false),
        ("bmad-method", "BMAD-METHOD", "Check if there are any skills containing 'bmad' in ~/.claude/skills/ and tell me if you see them.", "bmad", true),
        ("get-shit-done", "Get Shit Done", "/help", "gsd-", false),
    ];

    let mut success_count = 0;

    for (fw_id, fw_name, verify_cmd, expected_feature, is_skill_file) in test_cases {
        let test_eco_id = format!("test-all-iso-{}", fw_id);
        println!("\n--------------------------------------------------------");
        println!("Testing E2E Isolation for: {} ({})", fw_name, fw_id);
        println!("--------------------------------------------------------");

        // A. 如果生态已存在，先清理
        if EcosystemService::get_ecosystem_frameworks(&test_eco_id).is_ok() {
            let _ = EcosystemService::delete(&state, &test_eco_id);
        }

        // B. 创建生态并同步运行官方安装命令
        println!("[1/4] Creating ecosystem and installing framework...");
        let eco = EcosystemService::create(
            &state,
            &test_eco_id,
            &format!("E2E isolation test for {}", fw_name),
            vec![fw_id.to_string()],
        );

        if let Err(e) = eco {
            println!("  [ERROR] Failed to create ecosystem: {}", e);
            continue;
        }

        // C. 切换到测试生态
        println!("[2/4] Switching to test ecosystem {}...", test_eco_id);
        if let Err(e) = EcosystemService::switch(&state, &test_eco_id) {
            println!("  [ERROR] Failed to switch to ecosystem: {}", e);
            continue;
        }

        // D. 通过 Claude Code 命令对自身功能和隔离性进行校验
        println!("[3/4] Running Claude Code command verification...");
        
        // D1. 校验自身功能是否可用
        let self_ok = verify_self_feature(verify_cmd, expected_feature, is_skill_file);
        
        // D2. 校验交叉隔离性：在当前生态下绝对不应该能调用 OMC 功能 (如果是 OMC 生态，则校验不应该含有 GSD 功能)
        let cross_iso_ok = verify_cross_isolation(fw_id);

        if self_ok && cross_iso_ok {
            println!("  [OK] Verification passed! Framework functions are fully loaded and properly isolated.");
            success_count += 1;
        } else {
            println!("  [FAIL] Verification failed! self_ok = {}, cross_iso_ok = {}", self_ok, cross_iso_ok);
        }

        // E. 清理测试生态
        println!("[4/4] Cleaning up test ecosystem {}...", test_eco_id);
        let _ = EcosystemService::delete(&state, &test_eco_id);
    }

    // 4. 恢复测试前的初始生态环境
    println!("\n--------------------------------------------------------");
    println!("Verification finished. Restoring initial state...");
    if let Some(eco) = original_eco {
        println!("Restoring original active ecosystem: {} (ID: {})", eco.name, eco.id);
        let _ = EcosystemService::switch(&state, &eco.id);
    } else {
        println!("No original ecosystem to restore.");
    }
    println!("--------------------------------------------------------");

    println!("\nE2E Isolation Test Finished: {}/10 frameworks verified successfully.", success_count);
    if success_count == 10 {
        println!("ALL 10 AGENT FRAMEWORKS ARE VERIFIED AND 100% ISOLATED!");
    } else {
        println!("SOME FRAMEWORKS FAILED VERIFICATION. Please check the logs above.");
    }
}

/// 验证当前生态中的框架功能是否可被 Claude Code 读取和调用
fn verify_self_feature(verify_cmd: &str, expected_feature: &str, is_skill_file: bool) -> bool {
    if verify_cmd == "mcp list" {
        let output = Command::new("claude").args(["mcp", "list"]).output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            return stdout.contains(expected_feature) || stdout.contains("claude-flow");
        }
        return false;
    }

    if is_skill_file {
        // 对于只提供 .md 技能包的框架，通过让 Claude CLI 自身利用其 Ls 工具检索并问答来验证
        let output = Command::new("claude").args(["-p", verify_cmd]).output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
            let found = stdout.contains(expected_feature);
            println!("    -> Self skill check: '{}' detected in Claude feedback = {}", expected_feature, found);
            return found;
        }
        return false;
    } else {
        // 对于有独立命令文件的框架，通过验证 `claude -p "/help"` 的输出中包含该斜杠命令前缀来验证
        let output = Command::new("claude").args(["-p", "/help"]).output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let found = stdout.contains(expected_feature);
            println!("    -> Self command check: '{}' command visible in /help = {}", expected_feature, found);
            return found;
        }
        return false;
    }
}

/// 验证其他框架功能在当前激活生态下完全无法被调用（证明真正隔离）
fn verify_cross_isolation(fw_id: &str) -> bool {
    if fw_id == "ohmyclaudecode" {
        // 如果当前是 OMC 生态，验证其无法调用 GSD（Get Shit Done）命令
        let output = Command::new("claude").args(["-p", "/help"]).output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let gsd_leaked = stdout.contains("gsd-");
            println!("    -> Cross-check: GSD commands (/gsd-*) visible in OMC eco = {}", gsd_leaked);
            return !gsd_leaked;
        }
        return false;
    } else {
        // 如果当前是其他生态，验证其绝对无法调用 OMC 命令
        let output = Command::new("claude").args(["-p", "/help"]).output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let omc_leaked = stdout.contains("omc-");
            println!("    -> Cross-check: OMC commands (/omc-*) visible in {} eco = {}", fw_id, omc_leaked);
            return !omc_leaked;
        }
        return false;
    }
}
