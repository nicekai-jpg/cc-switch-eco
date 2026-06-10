/**
 * Hermes 策略
 */
import { hermesProviderPresets } from "@/config/hermesProviderPresets";
import { HERMES_DEFAULT_CONFIG } from "@/components/providers/forms/hooks/useHermesFormState";
import type { PresetEntry, PresetListStrategy, ProviderFormStrategy } from "./types";

export const hermesPresetListStrategy: PresetListStrategy = {
  getPresetEntries(): PresetEntry[] {
    return hermesProviderPresets.map<PresetEntry>((preset, index) => ({
      id: `hermes-${index}`,
      preset,
    }));
  },
};

export const hermesFormStrategy: ProviderFormStrategy = {
  getDefaultConfig(): string {
    return HERMES_DEFAULT_CONFIG;
  },
  supportsFullUrl(): boolean {
    return false;
  },
  hasProviderKey(): boolean {
    return true;
  },
};
