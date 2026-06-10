import type { BaseProviderPreset } from "./baseProviderPreset";
import { PROVIDER_METADATA } from "./providerMetadata";

export interface GeminiProviderPreset extends BaseProviderPreset {
  settingsConfig: object;
  baseURL?: string;
  model?: string;
  description?: string;
  endpointCandidates?: string[];
}

export const geminiProviderPresets: GeminiProviderPreset[] = [
  {
    ...PROVIDER_METADATA.googleOfficial,
    settingsConfig: {
      env: {},
    },
    description: "Google 官方 Gemini API (OAuth)",
    partnerPromotionKey: "google-official",
    theme: {
      icon: "gemini",
      backgroundColor: "#4285F4",
      textColor: "#FFFFFF",
    },
  },
  {
    ...PROVIDER_METADATA.shengsuanyun,
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://router.shengsuanyun.com/api",
        GEMINI_MODEL: "gemini-3.1-pro",
      },
    },
    baseURL: "https://router.shengsuanyun.com/api",
    model: "gemini-3.1-pro",
    description: "Shengsuanyun",
  },
  {
    ...PROVIDER_METADATA.packyCode,
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://www.packyapi.com",
        GEMINI_MODEL: "gemini-3.1-pro",
      },
    },
    baseURL: "https://www.packyapi.com",
    model: "gemini-3.1-pro",
    description: "PackyCode",
    endpointCandidates: [
      "https://api-slb.packyapi.com",
      "https://www.packyapi.com",
    ],
  },
  {
    ...PROVIDER_METADATA.cubence,
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://api.cubence.com",
        GEMINI_MODEL: "gemini-3.1-pro",
      },
    },
    baseURL: "https://api.cubence.com",
    model: "gemini-3.1-pro",
    description: "Cubence",
    endpointCandidates: [
      "https://api.cubence.com/v1",
      "https://api-cf.cubence.com/v1",
      "https://api-dmit.cubence.com/v1",
      "https://api-bwg.cubence.com/v1",
    ],
  },
  {
    ...PROVIDER_METADATA.aiGoCode,
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://api.aigocode.com",
        GEMINI_MODEL: "gemini-3.1-pro",
      },
    },
    baseURL: "https://api.aigocode.com",
    model: "gemini-3.1-pro",
    description: "AIGoCode",
    endpointCandidates: ["https://api.aigocode.com"],
  },
  {
    ...PROVIDER_METADATA.aiCodeMirror,
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://api.aicodemirror.com/api/gemini",
        GEMINI_MODEL: "gemini-3.1-pro",
      },
    },
    baseURL: "https://api.aicodemirror.com/api/gemini",
    model: "gemini-3.1-pro",
    description: "AICodeMirror",
    endpointCandidates: [
      "https://api.aicodemirror.com/api/gemini",
      "https://api.claudecode.net.cn/api/gemini",
    ],
  },
  {
    ...PROVIDER_METADATA.crazyRouter,
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://cn.crazyrouter.com",
        GEMINI_MODEL: "gemini-3.1-pro",
      },
    },
    baseURL: "https://cn.crazyrouter.com",
    model: "gemini-3.1-pro",
    description: "CrazyRouter",
    endpointCandidates: ["https://cn.crazyrouter.com"],
  },
  {
    ...PROVIDER_METADATA.sssAiCode,
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://node-hk.sssaicode.com/api",
        GEMINI_MODEL: "gemini-3.1-pro",
      },
    },
    baseURL: "https://node-hk.sssaicode.com/api",
    model: "gemini-3.1-pro",
    description: "SSSAiCode",
    endpointCandidates: [
      "https://node-hk.sssaicode.com/api",
      "https://claude2.sssaicode.com/api",
      "https://anti.sssaicode.com/api",
    ],
  },
  {
    ...PROVIDER_METADATA.cTok,
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://api.ctok.ai/v1beta",
        GEMINI_MODEL: "gemini-3.1-pro",
      },
    },
    baseURL: "https://api.ctok.ai/v1beta",
    model: "gemini-3.1-pro",
    description: "CTok",
    endpointCandidates: ["https://api.ctok.ai/v1beta"],
  },
  {
    ...PROVIDER_METADATA.eFlowCode,
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://e-flowcode.cc",
        GEMINI_API_KEY: "",
        GEMINI_MODEL: "gemini-3.1-pro-preview",
      },
      config: {
        general: {
          previewFeatures: true,
          sessionRetention: {
            enabled: true,
            maxAge: "30d",
            warningAcknowledged: true,
          },
        },
        mcpServers: {},
        security: {
          auth: {
            selectedType: "gemini-api-key",
          },
        },
      },
    },
    baseURL: "https://e-flowcode.cc",
    model: "gemini-3.1-pro-preview",
    description: "E-FlowCode",
    endpointCandidates: ["https://e-flowcode.cc"],
  },
  {
    ...PROVIDER_METADATA.lemonData,
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://api.lemondata.cc",
        GEMINI_MODEL: "gemini-3.1-pro",
      },
    },
    baseURL: "https://api.lemondata.cc",
    model: "gemini-3.1-pro",
    description: "LemonData",
    endpointCandidates: ["https://api.lemondata.cc"],
  },
  {
    ...PROVIDER_METADATA.openRouter,
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://openrouter.ai/api",
        GEMINI_MODEL: "gemini-3.1-pro",
      },
    },
    baseURL: "https://openrouter.ai/api",
    model: "gemini-3.1-pro",
    description: "OpenRouter",
  },
  {
    ...PROVIDER_METADATA.theRouter,
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "https://api.therouter.ai",
        GEMINI_MODEL: "gemini-3.1-pro",
      },
    },
    baseURL: "https://api.therouter.ai",
    model: "gemini-3.1-pro",
    description: "TheRouter",
    endpointCandidates: ["https://api.therouter.ai"],
  },
  {
    ...PROVIDER_METADATA.customGemini,
    settingsConfig: {
      env: {
        GOOGLE_GEMINI_BASE_URL: "",
        GEMINI_MODEL: "gemini-3.1-pro",
      },
    },
    model: "gemini-3.1-pro",
    description: "自定义 Gemini API 端点",
  },
];

export function getGeminiPresetByName(
  name: string,
): GeminiProviderPreset | undefined {
  return geminiProviderPresets.find((preset) => preset.name === name);
}

export function getGeminiPresetByUrl(
  url: string,
): GeminiProviderPreset | undefined {
  if (!url) return undefined;
  return geminiProviderPresets.find(
    (preset) =>
      preset.baseURL &&
      url.toLowerCase().includes(preset.baseURL.toLowerCase()),
  );
}
