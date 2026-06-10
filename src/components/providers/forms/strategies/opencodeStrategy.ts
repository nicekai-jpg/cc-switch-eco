/**
 * OpenCode 策略
 */
import { opencodeProviderPresets } from "@/config/opencodeProviderPresets";
import { OPENCODE_DEFAULT_CONFIG } from "@/components/providers/forms/helpers/opencodeFormUtils";
import type { AppStrategy } from "./types";

export const opencodeStrategy: AppStrategy = {
  presetEntries: opencodeProviderPresets.map((preset, index) => ({
    id: `opencode-${index}`,
    preset,
  })),
  defaultConfig: OPENCODE_DEFAULT_CONFIG,
  supportsFullUrl: false,
  hasProviderKey: true,
};
