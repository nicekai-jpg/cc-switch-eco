/**
 * OpenClaw 策略
 */
import { openclawProviderPresets } from "@/config/openclawProviderPresets";
import { OPENCLAW_DEFAULT_CONFIG } from "@/components/providers/forms/helpers/opencodeFormUtils";
import type { PresetEntry, PresetListStrategy, ProviderFormStrategy } from "./types";

export const openclawPresetListStrategy: PresetListStrategy = {
  getPresetEntries(): PresetEntry[] {
    return openclawProviderPresets.map<PresetEntry>((preset, index) => ({
      id: `openclaw-${index}`,
      preset,
    }));
  },
};

export const openclawFormStrategy: ProviderFormStrategy = {
  getDefaultConfig(): string {
    return OPENCLAW_DEFAULT_CONFIG;
  },
  supportsFullUrl(): boolean {
    return false;
  },
  hasProviderKey(): boolean {
    return true;
  },
};
