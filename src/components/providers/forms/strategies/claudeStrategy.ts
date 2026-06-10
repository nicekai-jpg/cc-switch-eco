/**
 * Claude 预设列表策略
 */
import { providerPresets } from "@/config/claudeProviderPresets";
import type { PresetEntry, PresetListStrategy } from "./types";

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
