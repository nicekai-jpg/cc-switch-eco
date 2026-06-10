/**
 * OpenCode 预设列表策略
 */
import { opencodeProviderPresets } from "@/config/opencodeProviderPresets";
import type { PresetEntry, PresetListStrategy } from "./types";

export const opencodePresetListStrategy: PresetListStrategy = {
  getPresetEntries(): PresetEntry[] {
    return opencodeProviderPresets.map<PresetEntry>((preset, index) => ({
      id: `opencode-${index}`,
      preset,
    }));
  },
};
