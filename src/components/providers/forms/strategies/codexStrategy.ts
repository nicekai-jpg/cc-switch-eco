/**
 * Codex 策略
 */
import { codexProviderPresets } from "@/config/codexProviderPresets";
import { CODEX_DEFAULT_CONFIG } from "@/components/providers/forms/helpers/opencodeFormUtils";
import type { PresetEntry, PresetListStrategy, ProviderFormStrategy } from "./types";

export const codexPresetListStrategy: PresetListStrategy = {
  getPresetEntries(): PresetEntry[] {
    return codexProviderPresets.map<PresetEntry>((preset, index) => ({
      id: `codex-${index}`,
      preset,
    }));
  },
};

export const codexFormStrategy: ProviderFormStrategy = {
  getDefaultConfig(): string {
    return CODEX_DEFAULT_CONFIG;
  },
  supportsFullUrl(): boolean {
    return true;
  },
  hasProviderKey(): boolean {
    return false;
  },
};
