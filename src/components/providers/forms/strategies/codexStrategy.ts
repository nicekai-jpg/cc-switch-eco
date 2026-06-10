/**
 * Codex 预设列表策略
 */
import { codexProviderPresets } from "@/config/codexProviderPresets";
import type { PresetEntry, PresetListStrategy } from "./types";

export const codexPresetListStrategy: PresetListStrategy = {
  getPresetEntries(): PresetEntry[] {
    return codexProviderPresets.map<PresetEntry>((preset, index) => ({
      id: `codex-${index}`,
      preset,
    }));
  },
};
