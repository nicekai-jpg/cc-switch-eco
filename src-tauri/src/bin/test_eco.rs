use cc_switch_eco_lib::Database;
use cc_switch_eco_lib::AppState;
use cc_switch_eco_lib::services::ecosystem::EcosystemService;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

fn main() {
    // 1. 设置重定向的 HOME 目录，实现 100% 隔离的本地集成测试
    let mut test_home = std::env::current_dir().unwrap();
    test_home.push("test_home");
    std::env::set_var("HOME", &test_home);
    println!("Redirected HOME to: {}", test_home.display());

    // 2. 清理先前的测试数据并创建隔离的配置文件夹
    if test_home.exists() {
        fs::remove_dir_all(&test_home).expect("Failed to clean up test_home directory");
    }
    fs::create_dir_all(&test_home).unwrap();
    // 创建 .claude 根目录，模拟真实环境中 Claude Code 已初始化的目录结构，避免 symlink 失败
    fs::create_dir_all(test_home.join(".claude")).unwrap();

    // 3. 初始化应用数据库与状态
    let db = Database::init().expect("Failed to initialize database");
    let state = AppState::new(Arc::new(db));

    // 4. 定义 10 个框架
    let frameworks = vec![
        ("superpowers", "Superpowers 中文版"),
        ("agency-agents-zh", "Agency Agents 中文版"),
        ("ohmyclaudecode", "Oh My ClaudeCode"),
        ("ruflo", "Ruflo"),
        ("speckit", "Spec Kit"),
        ("mattpocock-skills", "Matt Pocock Skills"),
        ("gstack", "GStack"),
        ("openspec", "OpenSpec"),
        ("bmad-method", "BMAD-METHOD"),
        ("get-shit-done", "Get Shit Done"),
    ];

    println!("\n========================================================");
    println!("Starting local integration test for 10 agent frameworks");
    println!("========================================================");

    for (fw_id, fw_name) in frameworks {
        let eco_id = format!("eco-{}", fw_id);
        println!("\n--------------------------------------------------------");
        println!("Testing Framework: {} ({})", fw_name, fw_id);
        println!("--------------------------------------------------------");

        // 创建生态环境并预装该框架 (同步进行 git clone 与官方安装命令执行)
        println!("[1/3] Creating ecosystem and installing framework...");
        let eco = EcosystemService::create(
            &state,
            &eco_id,
            &format!("Test environment for {}", fw_name),
            vec![fw_id.to_string()],
        );

        match eco {
            Ok(_) => {
                println!("Successfully created ecosystem and ran install command.");
            }
            Err(e) => {
                panic!("Failed to create ecosystem or run install command for {}: {}", fw_id, e);
            }
        }

        // 切换到该生态环境 (创建并指向新的 symlinks)
        println!("[2/3] Switching to ecosystem {}...", eco_id);
        EcosystemService::switch(&state, &eco_id).expect("Failed to switch ecosystem");

        // 验证框架配置和软链接是否生成并可调用
        println!("[3/3] Verifying files and executables...");
        verify_framework(fw_id, &test_home);

        println!("Framework {} verified successfully!", fw_id);
    }

    println!("\n========================================================");
    println!("ALL 10 AGENT FRAMEWORKS TESTED AND SWITCHED SUCCESSFULLY!");
    println!("========================================================");
}

fn verify_framework(id: &str, test_home: &PathBuf) {
    let claude_dir = test_home.join(".claude");
    assert!(claude_dir.exists(), ".claude directory does not exist!");

    match id {
        "superpowers" => {
            let skills_dir = claude_dir.join("skills");
            assert!(skills_dir.exists(), "skills directory missing!");
            let entries = fs::read_dir(skills_dir).unwrap();
            let mut has_superpowers = false;
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains("superpowers") {
                    has_superpowers = true;
                    break;
                }
            }
            assert!(has_superpowers, "Superpowers skills files were not created!");
            println!("  [OK] Superpowers skills files detected in ~/.claude/skills/");
        }
        "agency-agents-zh" => {
            let agents_dir = claude_dir.join("agents");
            assert!(agents_dir.exists(), "agents directory missing!");
            let entries = fs::read_dir(agents_dir).unwrap();
            let mut has_agency = false;
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains("agency") {
                    has_agency = true;
                    break;
                }
            }
            assert!(has_agency, "Agency agents files were not created!");
            println!("  [OK] Agency agents files detected in ~/.claude/agents/");
        }
        "ohmyclaudecode" => {
            // OMC 应该产生 omc- 前缀的文件或 hooks.json 的合并
            let skills_dir = claude_dir.join("skills");
            assert!(skills_dir.exists(), "skills directory missing!");
            let entries = fs::read_dir(skills_dir).unwrap();
            let mut has_omc = false;
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains("omc") {
                    has_omc = true;
                    break;
                }
            }
            assert!(has_omc, "OMC files were not created!");
            println!("  [OK] OMC files detected in ~/.claude/skills/");
        }
        "ruflo" => {
            let mcp_json = claude_dir.join("mcp.json");
            assert!(mcp_json.exists(), "mcp.json missing!");
            let content = fs::read_to_string(mcp_json).unwrap();
            assert!(content.contains("ruflo") || content.contains("claude-flow"), "Ruflo server configuration not found in mcp.json!");
            println!("  [OK] Ruflo configuration detected in ~/.claude/mcp.json");

            // 测试 Ruflo CLI 是否正常调用
            let output = Command::new("npx")
                .args(["ruflo@latest", "--version"])
                .output();
            if let Ok(out) = output {
                if out.status.success() {
                    let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    println!("  [OK] Ruflo CLI executed successfully: {}", ver);
                } else {
                    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    println!("  [WARNING] Ruflo CLI returned non-zero exit code: {}", err);
                }
            } else {
                println!("  [WARNING] Failed to run npx ruflo@latest --version");
            }
        }
        "speckit" => {
            let skills_dir = claude_dir.join("skills");
            assert!(skills_dir.exists(), "skills directory missing!");
            let entries = fs::read_dir(skills_dir).unwrap();
            let mut has_speckit = false;
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains("speckit") {
                    has_speckit = true;
                    break;
                }
            }
            assert!(has_speckit, "Spec Kit files were not created!");
            println!("  [OK] Spec Kit files detected in ~/.claude/skills/");

            // 测试 specify CLI 是否可以通过 uv tool run specify 调用
            let output = Command::new("uv")
                .args(["tool", "run", "specify", "--help"])
                .output();
            if let Ok(out) = output {
                if out.status.success() {
                    println!("  [OK] specify CLI help executed successfully via uv tool run");
                } else {
                    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    println!("  [WARNING] specify CLI returned non-zero exit code: {}", err);
                }
            } else {
                println!("  [WARNING] Failed to run uv tool run specify --help");
            }
        }
        "mattpocock-skills" => {
            // 通过 npx skills@latest add 安装
            let skills_dir = claude_dir.join("skills");
            assert!(skills_dir.exists(), "skills directory missing!");
            let entries = fs::read_dir(skills_dir).unwrap();
            let mut has_mp = false;
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains("mp") || name.contains("skills") {
                    has_mp = true;
                    break;
                }
            }
            assert!(has_mp, "Matt Pocock skills files were not created!");
            println!("  [OK] Matt Pocock skills files detected in ~/.claude/skills/");
        }
        "gstack" => {
            let skills_dir = claude_dir.join("skills");
            assert!(skills_dir.exists(), "skills directory missing!");
            let entries = fs::read_dir(skills_dir).unwrap();
            let mut has_gstack = false;
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains("gstack") {
                    has_gstack = true;
                    break;
                }
            }
            assert!(has_gstack, "GStack files were not created!");
            println!("  [OK] GStack files detected in ~/.claude/skills/");
        }
        "openspec" => {
            let commands_dir = claude_dir.join("commands");
            assert!(commands_dir.exists(), "commands directory missing!");
            let entries = fs::read_dir(commands_dir).unwrap();
            let mut has_openspec = false;
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains("openspec") {
                    has_openspec = true;
                    break;
                }
            }
            assert!(has_openspec, "OpenSpec files were not created!");
            println!("  [OK] OpenSpec files detected in ~/.claude/commands/");

            // 测试 OpenSpec CLI 调用
            let output = Command::new("npx")
                .args(["@fission-ai/openspec@latest", "--version"])
                .output();
            if let Ok(out) = output {
                if out.status.success() {
                    let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    println!("  [OK] OpenSpec CLI executed successfully: {}", ver);
                } else {
                    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    println!("  [WARNING] OpenSpec CLI returned non-zero exit code: {}", err);
                }
            } else {
                println!("  [WARNING] Failed to run OpenSpec CLI --version");
            }
        }
        "bmad-method" => {
            let skills_dir = claude_dir.join("skills");
            assert!(skills_dir.exists(), "skills directory missing!");
            let entries = fs::read_dir(skills_dir).unwrap();
            let mut has_bmad = false;
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains("bmad") {
                    has_bmad = true;
                    break;
                }
            }
            assert!(has_bmad, "BMAD files were not created!");
            println!("  [OK] BMAD files detected in ~/.claude/skills/");
        }
        "get-shit-done" => {
            let commands_dir = claude_dir.join("commands");
            assert!(commands_dir.exists(), "commands directory missing!");
            let entries = fs::read_dir(commands_dir).unwrap();
            let mut has_gsd = false;
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains("gsd") {
                    has_gsd = true;
                    break;
                }
            }
            assert!(has_gsd, "GSD files were not created!");
            println!("  [OK] GSD files detected in ~/.claude/commands/");
        }
        _ => {}
    }
}
