/**
 * Gemini 策略
 */
import { geminiProviderPresets } from "@/config/geminiProviderPresets";
import { GEMINI_DEFAULT_CONFIG } from "@/components/providers/forms/helpers/opencodeFormUtils";
import type { AppStrategy } from "./types";

export const geminiStrategy: AppStrategy = {
  presetEntries: geminiProviderPresets.map((preset, index) => ({
    id: `gemini-${index}`,
    preset,
  })),
  defaultConfig: GEMINI_DEFAULT_CONFIG,
  supportsFullUrl: false,
  hasProviderKey: false,
};
