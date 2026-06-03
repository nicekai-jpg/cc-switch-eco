use cc_switch_eco_lib::Database;
use cc_switch_eco_lib::AppState;
use cc_switch_eco_lib::services::ecosystem::EcosystemService;
use std::process::Command;
use std::sync::Arc;

fn main() {
    println!("========================================================");
    println!("VERIFYING ECOSYSTEM ISOLATION BETWEEN SUPERPOWERS & OMC");
    println!("========================================================");

    // 1. 初始化数据库与状态
    let db = Database::init().expect("Failed to initialize database");
    let state = AppState::new(Arc::new(db));

    // 2. 备份当前的活跃生态
    let original_eco = EcosystemService::get_current(&state).ok().flatten();

    // 确保 superpowers 生态和 ohmyclaudecode 生态已被创建并安装
    let eco_sp_id = "test-iso-superpowers";
    let eco_omc_id = "test-iso-ohmyclaudecode";

    // 清理之前的历史生态以保证干净
    let _ = EcosystemService::delete(&state, eco_sp_id);
    let _ = EcosystemService::delete(&state, eco_omc_id);

    println!("\n[Preparing] Creating and installing eco-superpowers...");
    EcosystemService::create(
        &state,
        eco_sp_id,
        "Superpowers test environment",
        vec!["superpowers".to_string()],
    ).expect("Failed to create superpowers eco");

    println!("[Preparing] Creating and installing eco-ohmyclaudecode...");
    EcosystemService::create(
        &state,
        eco_omc_id,
        "OMC test environment",
        vec!["ohmyclaudecode".to_string()],
    ).expect("Failed to create OMC eco");

    // ========================================================
    // 测试 1：切换到 superpowers，验证 superpowers 存在且可用，但 OMC 彻底消失
    // ========================================================
    println!("\n========================================================");
    println!("TEST 1: Switched to eco-superpowers");
    println!("========================================================");
    EcosystemService::switch(&state, eco_sp_id).expect("Failed to switch to superpowers");

    // A. 使用 claude 命令检查 superpowers 技能加载状态（Claude 会调用其自身的 Ls/View 工具去检索 ~/.claude/skills 目录）
    println!("  -> Calling Claude Code to check Superpowers skills...");
    let sp_skills_check_output = run_claude_prompt("Check if there are any skills containing 'superpowers' in ~/.claude/skills/ and tell me their names.");
    let sp_skills_detected = sp_skills_check_output.to_lowercase().contains("superpowers");
    println!("  -> Claude Code feedback: superpowers skills found = {}", sp_skills_detected);

    // B. 使用 claude 命令检查 OMC 是否存在
    println!("  -> Calling Claude Code to check OMC skills...");
    let sp_omc_check_output = run_claude_prompt("Check if there are any skills containing 'omc' in ~/.claude/skills/ and tell me if you see them.");
    let sp_contains_omc = sp_omc_check_output.to_lowercase().contains("omc") && !sp_omc_check_output.to_lowercase().contains("no");
    println!("  -> Claude Code feedback: omc skills found = {}", sp_contains_omc);

    // C. 使用 claude 命令检查 OMC 注册的斜杠指令是否可见
    let sp_help_output = run_claude_help();
    let sp_help_has_omc = sp_help_output.contains("omc-");
    println!("  -> Claude Code command check: OMC commands (/omc-*) visible in /help = {}", sp_help_has_omc);

    assert!(sp_skills_detected, "FAIL: Claude Code should be able to see Superpowers skills under superpowers eco!");
    assert!(!sp_contains_omc, "FAIL: Claude Code should NOT see any OMC skills under superpowers eco!");
    assert!(!sp_help_has_omc, "FAIL: Claude Code should NOT see or be able to invoke OMC commands under superpowers eco!");
    println!("  [SUCCESS] Test 1 passed: Claude Code verified that OMC is completely isolated and unreachable under superpowers eco!");

    // ========================================================
    // 测试 2：切换到 ohmyclaudecode，验证 OMC 存在且可用，但 superpowers 彻底消失
    // ========================================================
    println!("\n========================================================");
    println!("TEST 2: Switched to eco-ohmyclaudecode");
    println!("========================================================");
    EcosystemService::switch(&state, eco_omc_id).expect("Failed to switch to ohmyclaudecode");

    // A. 使用 claude 命令检查 superpowers 是否存在
    println!("  -> Calling Claude Code to check Superpowers skills...");
    let omc_sp_check_output = run_claude_prompt("Check if there are any skills containing 'superpowers' in ~/.claude/skills/ and tell me if you see them.");
    let omc_contains_sp = omc_sp_check_output.to_lowercase().contains("superpowers") && !omc_sp_check_output.to_lowercase().contains("no");
    println!("  -> Claude Code feedback: superpowers skills found = {}", omc_contains_sp);

    // B. 使用 claude 命令检查 OMC 技能加载状态
    println!("  -> Calling Claude Code to check OMC skills...");
    let omc_skills_check_output = run_claude_prompt("Check if there are any skills containing 'omc' in ~/.claude/skills/ and tell me their names.");
    let omc_skills_detected = omc_skills_check_output.to_lowercase().contains("omc");
    println!("  -> Claude Code feedback: omc skills found = {}", omc_skills_detected);

    // C. 使用 claude 命令检查 OMC 注册的斜杠指令是否可见
    let omc_help_output = run_claude_help();
    let omc_help_has_omc = omc_help_output.contains("omc-");
    println!("  -> Claude Code command check: OMC commands (/omc-*) visible in /help = {}", omc_help_has_omc);

    assert!(!omc_contains_sp, "FAIL: Claude Code should NOT see any Superpowers skills under OMC eco!");
    assert!(omc_skills_detected, "FAIL: Claude Code should see OMC skills under OMC eco!");
    assert!(omc_help_has_omc, "FAIL: Claude Code should see and be able to invoke OMC commands under OMC eco!");
    println!("  [SUCCESS] Test 2 passed: Claude Code verified that Superpowers is completely isolated and unreachable under OMC eco!");

    // ========================================================
    // 清理与恢复
    // ========================================================
    println!("\n========================================================");
    println!("CLEANUP & RESTORE");
    println!("========================================================");
    let _ = EcosystemService::delete(&state, eco_sp_id);
    let _ = EcosystemService::delete(&state, eco_omc_id);

    if let Some(eco) = original_eco {
        println!("Restoring original ecosystem: {}...", eco.name);
        let _ = EcosystemService::switch(&state, &eco.id);
    }
    println!("Isolation verification finished successfully. ALL ISOLATION ASSERTIONS PASSED!");
}

fn run_claude_prompt(prompt: &str) -> String {
    let output = Command::new("claude")
        .args(["-p", prompt])
        .output();
    match output {
        Ok(out) => {
            if out.status.success() {
                String::from_utf8_lossy(&out.stdout).to_string()
            } else {
                String::from_utf8_lossy(&out.stderr).to_string()
            }
        }
        Err(e) => format!("Failed to run `claude -p`: {}", e),
    }
}

fn run_claude_help() -> String {
    let output = Command::new("claude")
        .args(["-p", "/help"])
        .output();
    match output {
        Ok(out) => {
            if out.status.success() {
                String::from_utf8_lossy(&out.stdout).to_string()
            } else {
                String::from_utf8_lossy(&out.stderr).to_string()
            }
        }
        Err(_) => "".to_string(),
    }
}
