/**
 * Codex 策略
 */
import { codexProviderPresets } from "@/config/codexProviderPresets";
import { CODEX_DEFAULT_CONFIG } from "@/components/providers/forms/helpers/opencodeFormUtils";
import type { AppStrategy } from "./types";

export const codexStrategy: AppStrategy = {
  presetEntries: codexProviderPresets.map((preset, index) => ({
    id: `codex-${index}`,
    preset,
  })),
  defaultConfig: CODEX_DEFAULT_CONFIG,
  supportsFullUrl: true,
  hasProviderKey: false,
};
