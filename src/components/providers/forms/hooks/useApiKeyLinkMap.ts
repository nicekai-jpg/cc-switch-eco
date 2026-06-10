import { useMemo } from "react";
import type { AppId } from "@/lib/api";
import type { ProviderCategory } from "@/types";
import type { PresetEntry } from "../strategies/types";
import { useApiKeyLink } from "./useApiKeyLink";

interface UseApiKeyLinkMapProps {
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

const EMPTY_LINK: ApiKeyLinkInfo = {
  shouldShowApiKeyLink: false,
  websiteUrl: "",
  isPartner: false,
  partnerPromotionKey: undefined,
};

/**
 * 聚合所有 app 的 useApiKeyLink 结果为映射表
 *
 * 替代 ProviderForm.tsx 中 6 次独立调用 + 三元链传参。
 */
export function useApiKeyLinkMap({
  category,
  selectedPresetId,
  presetEntries,
  formWebsiteUrl,
}: UseApiKeyLinkMapProps): Record<string, ApiKeyLinkInfo> {
  const claudeLink = useApiKeyLink({ appId: "claude", category, selectedPresetId, presetEntries, formWebsiteUrl });
  const codexLink = useApiKeyLink({ appId: "codex", category, selectedPresetId, presetEntries, formWebsiteUrl });
  const geminiLink = useApiKeyLink({ appId: "gemini", category, selectedPresetId, presetEntries, formWebsiteUrl });
  const opencodeLink = useApiKeyLink({ appId: "opencode", category, selectedPresetId, presetEntries, formWebsiteUrl });
  const openclawLink = useApiKeyLink({ appId: "openclaw", category, selectedPresetId, presetEntries, formWebsiteUrl });
  const hermesLink = useApiKeyLink({ appId: "hermes", category, selectedPresetId, presetEntries, formWebsiteUrl });

  return useMemo(
    () => ({
      claude: claudeLink,
      codex: codexLink,
      gemini: geminiLink,
      opencode: opencodeLink,
      openclaw: openclawLink,
      hermes: hermesLink,
    }),
    [claudeLink, codexLink, geminiLink, opencodeLink, openclawLink, hermesLink],
  );
}

/**
 * 从映射表中获取当前 appId 的 ApiKeyLinkInfo
 */
export function getApiKeyLinkForApp(
  map: Record<string, ApiKeyLinkInfo>,
  appId: AppId,
): ApiKeyLinkInfo {
  return map[appId] ?? EMPTY_LINK;
}
