/**
 * OpenClaw 策略
 */
import { openclawProviderPresets } from "@/config/openclawProviderPresets";
import { OPENCLAW_DEFAULT_CONFIG } from "@/components/providers/forms/helpers/opencodeFormUtils";
import type { AppStrategy } from "./types";

export const openclawStrategy: AppStrategy = {
  presetEntries: openclawProviderPresets.map((preset, index) => ({
    id: `openclaw-${index}`,
    preset,
  })),
  defaultConfig: OPENCLAW_DEFAULT_CONFIG,
  supportsFullUrl: false,
  hasProviderKey: true,
};
