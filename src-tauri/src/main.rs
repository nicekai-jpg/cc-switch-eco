// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 在 Linux 上设置 WebKit 环境变量以解决 DMA-BUF 渲染问题
    // 某些 Linux 系统（如 Debian 13.2、Nvidia GPU）上 WebKitGTK 的 DMA-BUF 渲染器可能导致白屏/黑屏
    // 参考: https://github.com/tauri-apps/tauri/issues/9394
    #[cfg(target_os = "linux")]
    {
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        // 禁用 WebKitGTK 合成模式，规避 resize 时 webview 崩溃以及部分 Wayland
        // 合成器下的 surface 协商问题（整窗 UI 点击无响应、必须最大化-还原才能恢复）。
        // 参考: https://github.com/tauri-apps/tauri/issues/9394
        if std::env::var("WEBKIT_DISABLE_COMPOSITING_MODE").is_err() {
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }

    // 在 macOS 和 Linux 上，确保系统的 PATH 包含常用的开发者二进制目录。
    // macOS GUI 应用程序默认不会继承终端的 shell 配置（如 .zshrc），导致 which node/git 等检测失败。
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let current_path = std::env::var("PATH").unwrap_or_default();
        let home = std::env::var("HOME").unwrap_or_default();
        
        let mut paths: Vec<String> = Vec::new();
        
        // 优先添加常用的 Homebrew / 系统全局 / 工具链路径
        paths.push("/opt/homebrew/bin".to_string());
        paths.push("/usr/local/bin".to_string());
        paths.push("/opt/homebrew/sbin".to_string());
        paths.push("/usr/local/sbin".to_string());
        
        if !home.is_empty() {
            paths.push(format!("{}/.local/bin", home));
            paths.push(format!("{}/.cargo/bin", home));
            paths.push(format!("{}/.volta/bin", home));
            paths.push(format!("{}/.npm-global/bin", home));
        }
        
        // 保留原有的 PATH 项目以防遗漏
        for p in current_path.split(':') {
            if !p.is_empty() && !paths.iter().any(|existing| existing == p) {
                paths.push(p.to_string());
            }
        }
        
        std::env::set_var("PATH", paths.join(":"));
    }

    cc_switch_eco_lib::run();
}
