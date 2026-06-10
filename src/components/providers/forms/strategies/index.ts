/**
 * 策略注册入口
 *
 * 在应用启动时调用 registerAllStrategies() 注册所有策略。
 * 新增 app 只需在此文件添加注册调用。
 */
import type { AppId } from "@/lib/api";
import {
  registerPresetListStrategy,
  registerProviderFormStrategy,
} from "./types";
import { claudePresetListStrategy, claudeFormStrategy } from "./claudeStrategy";
import { codexPresetListStrategy, codexFormStrategy } from "./codexStrategy";
import { geminiPresetListStrategy, geminiFormStrategy } from "./geminiStrategy";
import { opencodePresetListStrategy, opencodeFormStrategy } from "./opencodeStrategy";
import { openclawPresetListStrategy, openclawFormStrategy } from "./openclawStrategy";
import { hermesPresetListStrategy, hermesFormStrategy } from "./hermesStrategy";

let registered = false;

/** 注册所有策略（幂等，多次调用安全） */
export function registerAllStrategies(): void {
  if (registered) return;
  registered = true;

  // 预设列表策略
  registerPresetListStrategy("claude" as AppId, claudePresetListStrategy);
  registerPresetListStrategy("codex" as AppId, codexPresetListStrategy);
  registerPresetListStrategy("gemini" as AppId, geminiPresetListStrategy);
  registerPresetListStrategy("opencode" as AppId, opencodePresetListStrategy);
  registerPresetListStrategy("openclaw" as AppId, openclawPresetListStrategy);
  registerPresetListStrategy("hermes" as AppId, hermesPresetListStrategy);

  // ProviderForm 策略
  registerProviderFormStrategy("claude" as AppId, claudeFormStrategy);
  registerProviderFormStrategy("codex" as AppId, codexFormStrategy);
  registerProviderFormStrategy("gemini" as AppId, geminiFormStrategy);
  registerProviderFormStrategy("opencode" as AppId, opencodeFormStrategy);
  registerProviderFormStrategy("openclaw" as AppId, openclawFormStrategy);
  registerProviderFormStrategy("hermes" as AppId, hermesFormStrategy);

  // 表单字段策略（Phase 3 后续步骤注册）
}

// 导出类型供外部使用
export type { PresetEntry, PresetListStrategy, ProviderFormStrategy, FormFieldsStrategy } from "./types";
export {
  getPresetListStrategy,
  getProviderFormStrategy,
  getFormFieldsStrategy,
} from "./types";
