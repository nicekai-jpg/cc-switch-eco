/**
 * OpenCode 策略
 */
import { opencodeProviderPresets } from "@/config/opencodeProviderPresets";
import { OPENCODE_DEFAULT_CONFIG } from "@/components/providers/forms/helpers/opencodeFormUtils";
import type { PresetEntry, PresetListStrategy, ProviderFormStrategy } from "./types";

export const opencodePresetListStrategy: PresetListStrategy = {
  getPresetEntries(): PresetEntry[] {
    return opencodeProviderPresets.map<PresetEntry>((preset, index) => ({
      id: `opencode-${index}`,
      preset,
    }));
  },
};

export const opencodeFormStrategy: ProviderFormStrategy = {
  getDefaultConfig(): string {
    return OPENCODE_DEFAULT_CONFIG;
  },
  supportsFullUrl(): boolean {
    return false;
  },
  hasProviderKey(): boolean {
    return true;
  },
};
