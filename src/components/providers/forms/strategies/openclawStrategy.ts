/**
 * OpenClaw 预设列表策略
 */
import { openclawProviderPresets } from "@/config/openclawProviderPresets";
import type { PresetEntry, PresetListStrategy } from "./types";

export const openclawPresetListStrategy: PresetListStrategy = {
  getPresetEntries(): PresetEntry[] {
    return openclawProviderPresets.map<PresetEntry>((preset, index) => ({
      id: `openclaw-${index}`,
      preset,
    }));
  },
};
