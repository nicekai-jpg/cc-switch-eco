/**
 * Gemini 预设列表策略
 */
import { geminiProviderPresets } from "@/config/geminiProviderPresets";
import type { PresetEntry, PresetListStrategy } from "./types";

export const geminiPresetListStrategy: PresetListStrategy = {
  getPresetEntries(): PresetEntry[] {
    return geminiProviderPresets.map<PresetEntry>((preset, index) => ({
      id: `gemini-${index}`,
      preset,
    }));
  },
};
