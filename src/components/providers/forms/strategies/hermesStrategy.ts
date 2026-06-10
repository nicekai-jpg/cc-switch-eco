/**
 * Hermes 策略
 */
import { hermesProviderPresets } from "@/config/hermesProviderPresets";
import { HERMES_DEFAULT_CONFIG } from "@/components/providers/forms/hooks/useHermesFormState";
import type { AppStrategy } from "./types";

export const hermesStrategy: AppStrategy = {
  presetEntries: hermesProviderPresets.map((preset, index) => ({
    id: `hermes-${index}`,
    preset,
  })),
  defaultConfig: HERMES_DEFAULT_CONFIG,
  supportsFullUrl: false,
  hasProviderKey: true,
};
