/**
 * ProviderPreset 公共基接口与共享类型
 *
 * 从 claudeProviderPresets.ts 提取，供所有 *ProviderPresets.ts 复用。
 * 各应用预设接口应 extends BaseProviderPreset，仅声明自身特有字段。
 */
import type { ProviderCategory } from "../types";

/** 模板变量定义 */
export interface TemplateValueConfig {
  label: string;
  placeholder: string;
  defaultValue?: string;
  editorValue: string;
}

/** 预设供应商的视觉主题配置 */
export interface PresetTheme {
  /** 图标类型：'claude' | 'codex' | 'gemini' | 'generic' */
  icon?: "claude" | "codex" | "gemini" | "generic";
  /** 背景色（选中状态），支持 Tailwind 类名或 hex 颜色 */
  backgroundColor?: string;
  /** 文字色（选中状态），支持 Tailwind 类名或 hex 颜色 */
  textColor?: string;
}

/** 所有 ProviderPreset 的公共基接口 */
export interface BaseProviderPreset {
  name: string;
  /** i18n key for localized display name */
  nameKey?: string;
  websiteUrl: string;
  /** 第三方/聚合等可单独配置获取 API Key 的链接 */
  apiKeyUrl?: string;
  /** 标识是否为官方预设 */
  isOfficial?: boolean;
  /** 标识是否为商业合作伙伴 */
  isPartner?: boolean;
  /** 合作伙伴促销信息的 i18n key */
  partnerPromotionKey?: string;
  /** 供应商分类 */
  category?: ProviderCategory;
  /** 视觉主题配置 */
  theme?: PresetTheme;
  /** 图标名称 */
  icon?: string;
  /** 图标颜色 */
  iconColor?: string;
}
