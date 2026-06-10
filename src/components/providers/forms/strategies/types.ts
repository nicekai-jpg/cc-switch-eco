/**
 * ProviderForm 策略模式 — 类型定义
 *
 * 每个 app 实现独立的策略，通过注册表获取。
 * 新增 app 只需新增策略文件 + 注册，无需修改 ProviderForm.tsx。
 */
import type { ComponentType } from "react";
import type { AppId } from "@/lib/api";

/** 预设条目（供 presetEntries 使用） */
export interface PresetEntry {
  id: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  preset: any; // 各 app 的 preset 类型不同，用 any 统一
}

/**
 * 预设列表策略
 *
 * 负责根据 appId 返回对应的预设列表。
 * 这是策略模式的第一步提取，后续可逐步扩展 validate/buildPayload 等方法。
 */
export interface PresetListStrategy {
  /** 获取该 app 的预设条目列表 */
  getPresetEntries(): PresetEntry[];
}

/**
 * 表单字段子组件策略
 *
 * 负责根据 appId 渲染对应的表单字段组件。
 * 替代 AppSpecificFormFields 中的 switch(appId) 分发。
 */
export interface FormFieldsStrategy {
  /** 该 app 的表单字段子组件 */
  FormFields: ComponentType<FormFieldsProps>;
  /** 该 app 的配置编辑器子组件 */
  ConfigEditor: ComponentType<ConfigEditorProps>;
}

/** 表单字段子组件的通用 props */
export interface FormFieldsProps {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any; // 各 app 的 props 不同，用通用签名
}

/** 配置编辑器子组件的通用 props */
export interface ConfigEditorProps {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

/** 策略注册表 */
const presetListStrategies = new Map<AppId, PresetListStrategy>();
const formFieldsStrategies = new Map<AppId, FormFieldsStrategy>();

/** 注册预设列表策略 */
export function registerPresetListStrategy(
  appId: AppId,
  strategy: PresetListStrategy,
): void {
  presetListStrategies.set(appId, strategy);
}

/** 获取预设列表策略 */
export function getPresetListStrategy(appId: AppId): PresetListStrategy {
  const strategy = presetListStrategies.get(appId);
  if (!strategy) {
    throw new Error(`No PresetListStrategy registered for app: ${appId}`);
  }
  return strategy;
}

/** 注册表单字段策略 */
export function registerFormFieldsStrategy(
  appId: AppId,
  strategy: FormFieldsStrategy,
): void {
  formFieldsStrategies.set(appId, strategy);
}

/** 获取表单字段策略 */
export function getFormFieldsStrategy(appId: AppId): FormFieldsStrategy {
  const strategy = formFieldsStrategies.get(appId);
  if (!strategy) {
    throw new Error(`No FormFieldsStrategy registered for app: ${appId}`);
  }
  return strategy;
}
