import { useMemo } from "react";
import type { ProviderCategory } from "@/types";
import type { PresetEntry } from "../strategies/types";

interface UseApiKeyLinkCoreProps {
  category?: ProviderCategory;
  selectedPresetId: string | null;
  presetEntries: PresetEntry[];
  formWebsiteUrl: string;
}

export interface ApiKeyLinkInfo {
  shouldShowApiKeyLink: boolean;
  websiteUrl: string;
  isPartner: boolean;
  partnerPromotionKey?: string;
}

/**
 * API Key 链接核心逻辑（不依赖 appId）
 *
 * 从 useApiKeyLink 提取，appId 仅影响 shouldShowApiKeyLink 的开关，
 * 其余逻辑（websiteUrl / isPartner / partnerPromotionKey）与 appId 无关。
 */
export function useApiKeyLinkCore({
  category,
  selectedPresetId,
  presetEntries,
  formWebsiteUrl,
}: UseApiKeyLinkCoreProps): Omit<ApiKeyLinkInfo, "shouldShowApiKeyLink"> & {
  /** category 满足条件时 shouldShowApiKeyLink 为 true，由调用方按 appId 决定是否启用 */
  categoryAllowsApiKeyLink: boolean;
} {
  const categoryAllowsApiKeyLink = useMemo(() => {
    return (
      category !== "official" &&
      (category === "cn_official" ||
        category === "aggregator" ||
        category === "third_party")
    );
  }, [category]);

  const currentPresetEntry = useMemo(() => {
    if (selectedPresetId && selectedPresetId !== "custom") {
      return presetEntries.find((item) => item.id === selectedPresetId);
    }
    return undefined;
  }, [selectedPresetId, presetEntries]);

  const websiteUrl = useMemo(() => {
    if (currentPresetEntry) {
      const preset = currentPresetEntry.preset;
      if (
        preset.category === "cn_official" ||
        preset.category === "aggregator" ||
        preset.category === "third_party"
      ) {
        return preset.apiKeyUrl || preset.websiteUrl || "";
      }
      return preset.websiteUrl || "";
    }
    return formWebsiteUrl || "";
  }, [currentPresetEntry, formWebsiteUrl]);

  const isPartner = useMemo(() => {
    return currentPresetEntry?.preset.isPartner ?? false;
  }, [currentPresetEntry]);

  const partnerPromotionKey = useMemo(() => {
    return currentPresetEntry?.preset.partnerPromotionKey;
  }, [currentPresetEntry]);

  return { categoryAllowsApiKeyLink, websiteUrl, isPartner, partnerPromotionKey };
}
