/**
 * 策略注册入口
 *
 * 在应用启动时调用 registerAllStrategies() 注册所有策略。
 * 新增 app 只需在此文件添加注册调用。
 */
import type { AppId } from "@/lib/api";
import {
  registerPresetListStrategy,
} from "./types";
import { claudePresetListStrategy } from "./claudeStrategy";
import { codexPresetListStrategy } from "./codexStrategy";
import { geminiPresetListStrategy } from "./geminiStrategy";
import { opencodePresetListStrategy } from "./opencodeStrategy";
import { openclawPresetListStrategy } from "./openclawStrategy";
import { hermesPresetListStrategy } from "./hermesStrategy";

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

  // 表单字段策略（Phase 3 后续步骤注册）
}

// 导出类型供外部使用
export type { PresetEntry, PresetListStrategy, FormFieldsStrategy } from "./types";
export {
  getPresetListStrategy,
  getFormFieldsStrategy,
} from "./types";
