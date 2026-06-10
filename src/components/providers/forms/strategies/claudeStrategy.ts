/**
 * Claude 策略
 */
import { providerPresets } from "@/config/claudeProviderPresets";
import { CLAUDE_DEFAULT_CONFIG } from "@/components/providers/forms/helpers/opencodeFormUtils";
import type { PresetEntry, PresetListStrategy, ProviderFormStrategy } from "./types";

export const claudePresetListStrategy: PresetListStrategy = {
  getPresetEntries(): PresetEntry[] {
    return providerPresets
      .filter((p) => !p.hidden)
      .map<PresetEntry>((preset, index) => ({
        id: `claude-${index}`,
        preset,
      }));
  },
};

export const claudeFormStrategy: ProviderFormStrategy = {
  getDefaultConfig(): string {
    return CLAUDE_DEFAULT_CONFIG;
  },
  supportsFullUrl(): boolean {
    return true;
  },
  hasProviderKey(): boolean {
    return false;
  },
};
