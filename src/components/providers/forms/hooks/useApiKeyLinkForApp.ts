import type { AppId } from "@/lib/api";
import type { ProviderCategory } from "@/types";
import type { PresetEntry } from "../strategies/types";
import { useApiKeyLinkCore, type ApiKeyLinkInfo } from "./useApiKeyLinkCore";

interface UseApiKeyLinkMapProps {
  appId: AppId;
  category?: ProviderCategory;
  selectedPresetId: string | null;
  presetEntries: PresetEntry[];
  formWebsiteUrl: string;
}

/** 支持 API Key 链接显示的 appId 集合 */
const API_KEY_LINK_APPS = new Set<AppId>(["claude", "codex", "gemini", "opencode"]);

/**
 * 获取当前 appId 的 API Key 链接信息
 *
 * 替代原 6 次 useApiKeyLink 独立调用。核心逻辑只执行 1 次，
 * shouldShowApiKeyLink 由 appId 集合决定。
 */
export function useApiKeyLinkForApp({
  appId,
  category,
  selectedPresetId,
  presetEntries,
  formWebsiteUrl,
}: UseApiKeyLinkMapProps): ApiKeyLinkInfo {
  const core = useApiKeyLinkCore({ category, selectedPresetId, presetEntries, formWebsiteUrl });

  return {
    shouldShowApiKeyLink: API_KEY_LINK_APPS.has(appId) && core.categoryAllowsApiKeyLink,
    websiteUrl: core.websiteUrl,
    isPartner: core.isPartner,
    partnerPromotionKey: core.partnerPromotionKey,
  };
}

export type { ApiKeyLinkInfo };
