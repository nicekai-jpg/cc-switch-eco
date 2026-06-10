/**
 * 统一应用策略 — 类型定义与注册表
 *
 * 合并原 PresetListStrategy + ProviderFormStrategy 为单一 AppStrategy。
 * 常量方法改为声明式字段，删除未使用的 FormFieldsStrategy。
 */
import type { AppId } from "@/lib/api";
import type { ProviderPreset } from "@/config/claudeProviderPresets";
import type { CodexProviderPreset } from "@/config/codexProviderPresets";
import type { GeminiProviderPreset } from "@/config/geminiProviderPresets";
import type { OpenCodeProviderPreset } from "@/config/opencodeProviderPresets";
import type { OpenClawProviderPreset } from "@/config/openclawProviderPresets";
import type { HermesProviderPreset } from "@/config/hermesProviderPresets";

/** 所有 app 预设类型的联合（替代 any） */
export type AnyProviderPreset =
  | ProviderPreset
  | CodexProviderPreset
  | GeminiProviderPreset
  | OpenCodeProviderPreset
  | OpenClawProviderPreset
  | HermesProviderPreset;

/** 预设条目（preset 类型安全） */
export interface PresetEntry {
  id: string;
  preset: AnyProviderPreset;
}

/**
 * 统一应用策略 — 声明式配置
 *
 * 合并原 PresetListStrategy + ProviderFormStrategy。
 * 原策略方法全是常量，改为声明式字段更简洁。
 */
export interface AppStrategy {
  /** 预设条目列表 */
  presetEntries: PresetEntry[];
  /** 新建自定义预设时的默认 settingsConfig 字符串 */
  defaultConfig: string;
  /** 是否支持 Full URL 模式 */
  supportsFullUrl: boolean;
  /** 是否有 providerKey（opencode/openclaw/hermes） */
  hasProviderKey: boolean;
}

/** 策略注册表 */
const strategies = new Map<AppId, AppStrategy>();

/** 注册策略 */
export function registerStrategy(appId: AppId, strategy: AppStrategy): void {
  strategies.set(appId, strategy);
}

/** 获取策略 */
export function getStrategy(appId: AppId): AppStrategy {
  const strategy = strategies.get(appId);
  if (!strategy) {
    throw new Error(`No AppStrategy registered for app: ${appId}`);
  }
  return strategy;
}
