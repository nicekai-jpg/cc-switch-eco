/**
 * Claude 策略
 */
import { providerPresets } from "@/config/claudeProviderPresets";
import { CLAUDE_DEFAULT_CONFIG } from "@/components/providers/forms/helpers/opencodeFormUtils";
import type { AppStrategy } from "./types";

export const claudeStrategy: AppStrategy = {
  presetEntries: providerPresets
    .filter((p) => !p.hidden)
    .map((preset, index) => ({
      id: `claude-${index}`,
      preset,
    })),
  defaultConfig: CLAUDE_DEFAULT_CONFIG,
  supportsFullUrl: true,
  hasProviderKey: false,
};
