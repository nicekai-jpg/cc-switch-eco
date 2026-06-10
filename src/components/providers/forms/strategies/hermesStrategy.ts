/**
 * Hermes 预设列表策略
 */
import { hermesProviderPresets } from "@/config/hermesProviderPresets";
import type { PresetEntry, PresetListStrategy } from "./types";

export const hermesPresetListStrategy: PresetListStrategy = {
  getPresetEntries(): PresetEntry[] {
    return hermesProviderPresets.map<PresetEntry>((preset, index) => ({
      id: `hermes-${index}`,
      preset,
    }));
  },
};
