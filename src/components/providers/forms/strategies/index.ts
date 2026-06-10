/**
 * 策略注册入口
 *
 * 模块加载时立即执行注册（副作用在顶层 import，不在组件渲染路径中）。
 * 新增 app 只需在此文件添加注册条目。
 */
import type { AppId } from "@/lib/api";
import { registerStrategy } from "./types";
import { claudeStrategy } from "./claudeStrategy";
import { codexStrategy } from "./codexStrategy";
import { geminiStrategy } from "./geminiStrategy";
import { opencodeStrategy } from "./opencodeStrategy";
import { openclawStrategy } from "./openclawStrategy";
import { hermesStrategy } from "./hermesStrategy";

/**
 * 声明式策略注册表
 *
 * key 类型为 AppId 字面量，TypeScript 自动收窄，无需 as AppId。
 */
const strategyRegistrations: Record<AppId, () => void> = {
  claude: () => registerStrategy("claude", claudeStrategy),
  "claude-desktop": () => {}, // claude-desktop 走独立表单，不注册策略
  codex: () => registerStrategy("codex", codexStrategy),
  gemini: () => registerStrategy("gemini", geminiStrategy),
  opencode: () => registerStrategy("opencode", opencodeStrategy),
  openclaw: () => registerStrategy("openclaw", openclawStrategy),
  hermes: () => registerStrategy("hermes", hermesStrategy),
};

// 模块初始化时注册所有策略
Object.values(strategyRegistrations).forEach((register) => register());

// 导出
export type { PresetEntry, AppStrategy, AnyProviderPreset } from "./types";
export { getStrategy } from "./types";
