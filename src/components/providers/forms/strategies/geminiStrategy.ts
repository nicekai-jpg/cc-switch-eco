/**
 * Gemini 策略
 */
import { geminiProviderPresets } from "@/config/geminiProviderPresets";
import { GEMINI_DEFAULT_CONFIG } from "@/components/providers/forms/helpers/opencodeFormUtils";
import type { PresetEntry, PresetListStrategy, ProviderFormStrategy } from "./types";

export const geminiPresetListStrategy: PresetListStrategy = {
  getPresetEntries(): PresetEntry[] {
    return geminiProviderPresets.map<PresetEntry>((preset, index) => ({
      id: `gemini-${index}`,
      preset,
    }));
  },
};

export const geminiFormStrategy: ProviderFormStrategy = {
  getDefaultConfig(): string {
    return GEMINI_DEFAULT_CONFIG;
  },
  supportsFullUrl(): boolean {
    return false;
  },
  hasProviderKey(): boolean {
    return false;
  },
};
