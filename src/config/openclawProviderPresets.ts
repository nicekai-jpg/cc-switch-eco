/**
 * OpenClaw provider presets configuration
 * OpenClaw uses models.providers structure with custom provider configs
 */
import type {
  OpenClawProviderConfig,
  OpenClawDefaultModel,
} from "../types";
import type {
  BaseProviderPreset,
  TemplateValueConfig,
} from "./baseProviderPreset";
import { PROVIDER_METADATA } from "./providerMetadata";

/** Suggested default model configuration for a preset */
export interface OpenClawSuggestedDefaults {
  /** Default model config to apply (agents.defaults.model) */
  model?: OpenClawDefaultModel;
  /** Model catalog entries to add (agents.defaults.models) */
  modelCatalog?: Record<string, { alias?: string }>;
}

export interface OpenClawProviderPreset extends BaseProviderPreset {
  /** OpenClaw settings_config structure */
  settingsConfig: OpenClawProviderConfig;
  /** Template variable definitions */
  templateValues?: Record<string, TemplateValueConfig>;
  /** Mark as custom template (for UI distinction) */
  isCustomTemplate?: boolean;
  /** Suggested default model configuration */
  suggestedDefaults?: OpenClawSuggestedDefaults;
}

function rebaseOpenClawModelRef(modelRef: string, providerKey: string): string {
  const slashIndex = modelRef.indexOf("/");
  return slashIndex === -1
    ? `${providerKey}/${modelRef}`
    : `${providerKey}${modelRef.slice(slashIndex)}`;
}

/**
 * OpenClaw default model refs are stored as "<provider-key>/<model-id>".
 * Presets carry stable built-in keys for display/tests, but the real key is
 * chosen in the add-provider form, so rewrite refs right before submission.
 */
export function rebaseOpenClawSuggestedDefaults(
  defaults: OpenClawSuggestedDefaults,
  providerKey: string,
): OpenClawSuggestedDefaults {
  const key = providerKey.trim();
  if (!key) return defaults;

  return {
    model: defaults.model
      ? {
          ...defaults.model,
          primary: rebaseOpenClawModelRef(defaults.model.primary, key),
          fallbacks: defaults.model.fallbacks?.map((modelRef) =>
            rebaseOpenClawModelRef(modelRef, key),
          ),
        }
      : undefined,
    modelCatalog: defaults.modelCatalog
      ? Object.fromEntries(
          Object.entries(defaults.modelCatalog).map(([modelRef, entry]) => [
            rebaseOpenClawModelRef(modelRef, key),
            entry,
          ]),
        )
      : undefined,
  };
}

/**
 * OpenClaw API protocol options
 * @see https://github.com/openclaw/openclaw/blob/main/docs/gateway/configuration.md
 */
export const openclawApiProtocols = [
  { value: "openai-completions", label: "OpenAI Completions" },
  { value: "openai-responses", label: "OpenAI Responses" },
  { value: "anthropic-messages", label: "Anthropic Messages" },
  { value: "google-generative-ai", label: "Google Generative AI" },
  { value: "bedrock-converse-stream", label: "AWS Bedrock" },
] as const;

/**
 * OpenClaw provider presets list
 */
export const openclawProviderPresets: OpenClawProviderPreset[] = [
  {
    ...PROVIDER_METADATA.shengsuanyun,
    settingsConfig: {
      baseUrl: "https://router.shengsuanyun.com/api",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
        {
          id: "claude-sonnet-4-6",
          name: "Claude Sonnet 4.6",
          contextWindow: 1000000,
          cost: { input: 3, output: 15 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "shengsuanyun/claude-opus-4-7",
        fallbacks: ["shengsuanyun/claude-sonnet-4-6"],
      },
      modelCatalog: {
        "shengsuanyun/claude-opus-4-7": { alias: "Opus" },
        "shengsuanyun/claude-sonnet-4-6": { alias: "Sonnet" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.volcengineAgentplan,
    settingsConfig: {
      baseUrl: "https://ark.cn-beijing.volces.com/api/coding/v3",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "ark-code-latest",
          name: "Ark Code Latest",
          contextWindow: 256000,
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "ark_agentplan/ark-code-latest" },
      modelCatalog: {
        "ark_agentplan/ark-code-latest": { alias: "Ark Code" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.bytePlus,
    settingsConfig: {
      baseUrl: "https://ark.ap-southeast.bytepluses.com/api/coding/v3",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "ark-code-latest",
          name: "Ark Code Latest",
          contextWindow: 256000,
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "byteplus/ark-code-latest" },
      modelCatalog: {
        "byteplus/ark-code-latest": { alias: "Ark Code" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.douBaoSeed,
    settingsConfig: {
      baseUrl: "https://ark.cn-beijing.volces.com/api/v3",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "doubao-seed-2-0-code-preview-latest",
          name: "DouBao Seed Code Preview",
          contextWindow: 128000,
          cost: { input: 0.002, output: 0.006 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "doubaoseed/doubao-seed-2-0-code-preview-latest" },
      modelCatalog: {
        "doubaoseed/doubao-seed-2-0-code-preview-latest": { alias: "DouBao" },
      },
    },
  },
  // ========== Chinese Officials ==========
  {
    ...PROVIDER_METADATA.deepSeek,
    settingsConfig: {
      baseUrl: "https://api.deepseek.com/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "deepseek-v4-pro",
          name: "DeepSeek V4 Pro",
          contextWindow: 1000000,
          cost: { input: 1.68, output: 3.36 },
        },
        {
          id: "deepseek-v4-flash",
          name: "DeepSeek V4 Flash",
          contextWindow: 1000000,
          cost: { input: 0.14, output: 0.28 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "deepseek/deepseek-v4-flash",
        fallbacks: ["deepseek/deepseek-v4-pro"],
      },
      modelCatalog: {
        "deepseek/deepseek-v4-flash": { alias: "Flash" },
        "deepseek/deepseek-v4-pro": { alias: "Pro" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.zhipuGlm,
    settingsConfig: {
      baseUrl: "https://open.bigmodel.cn/api/paas/v4",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "glm-5",
          name: "GLM-5",
          contextWindow: 128000,
          cost: { input: 0.001, output: 0.001 },
        },
      ],
    },
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://open.bigmodel.cn/api/paas/v4",
        defaultValue: "https://open.bigmodel.cn/api/paas/v4",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "zhipu/glm-5" },
      modelCatalog: { "zhipu/glm-5": { alias: "GLM" } },
    },
  },
  {
    ...PROVIDER_METADATA.zhipuGlmEn,
    settingsConfig: {
      baseUrl: "https://api.z.ai/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "glm-5",
          name: "GLM-5",
          contextWindow: 128000,
          cost: { input: 0.001, output: 0.001 },
        },
      ],
    },
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.z.ai/v1",
        defaultValue: "https://api.z.ai/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "zhipu-en/glm-5" },
      modelCatalog: { "zhipu-en/glm-5": { alias: "GLM" } },
    },
  },
  {
    ...PROVIDER_METADATA.qwenCoder,
    settingsConfig: {
      baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "qwen3.5-plus",
          name: "Qwen3.5 Plus",
          contextWindow: 32000,
          cost: { input: 0.002, output: 0.006 },
        },
      ],
    },
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        defaultValue: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "qwen/qwen3.5-plus" },
      modelCatalog: { "qwen/qwen3.5-plus": { alias: "Qwen" } },
    },
  },
  {
    ...PROVIDER_METADATA.kimiK26,
    settingsConfig: {
      baseUrl: "https://api.moonshot.cn/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "kimi-k2.6",
          name: "Kimi K2.6",
          contextWindow: 131072,
          cost: { input: 0.002, output: 0.006 },
        },
      ],
    },
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.moonshot.cn/v1",
        defaultValue: "https://api.moonshot.cn/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "kimi/kimi-k2.6" },
      modelCatalog: { "kimi/kimi-k2.6": { alias: "Kimi" } },
    },
  },
  {
    ...PROVIDER_METADATA.kimiForCoding,
    settingsConfig: {
      baseUrl: "https://api.kimi.com/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "kimi-for-coding",
          name: "Kimi For Coding",
          contextWindow: 131072,
          cost: { input: 0.002, output: 0.006 },
        },
      ],
    },
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.kimi.com/v1",
        defaultValue: "https://api.kimi.com/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "kimi-coding/kimi-for-coding" },
      modelCatalog: { "kimi-coding/kimi-for-coding": { alias: "Kimi" } },
    },
  },
  {
    ...PROVIDER_METADATA.stepFun,
    settingsConfig: {
      baseUrl: "https://api.stepfun.com/step_plan/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "step-3.5-flash-2603",
          name: "Step 3.5 Flash 2603",
          contextWindow: 262144,
        },
        {
          id: "step-3.5-flash",
          name: "Step 3.5 Flash",
          contextWindow: 262144,
        },
      ],
    },
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.stepfun.com/step_plan/v1",
        defaultValue: "https://api.stepfun.com/step_plan/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "step-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "stepfun/step-3.5-flash-2603" },
      modelCatalog: {
        "stepfun/step-3.5-flash-2603": { alias: "StepFun" },
        "stepfun/step-3.5-flash": { alias: "StepFun Flash" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.stepFunEn,
    settingsConfig: {
      baseUrl: "https://api.stepfun.ai/step_plan/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "step-3.5-flash-2603",
          name: "Step 3.5 Flash 2603",
          contextWindow: 262144,
        },
        {
          id: "step-3.5-flash",
          name: "Step 3.5 Flash",
          contextWindow: 262144,
        },
      ],
    },
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.stepfun.ai/step_plan/v1",
        defaultValue: "https://api.stepfun.ai/step_plan/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "step-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "stepfun-en/step-3.5-flash-2603" },
      modelCatalog: {
        "stepfun-en/step-3.5-flash-2603": { alias: "StepFun" },
        "stepfun-en/step-3.5-flash": { alias: "StepFun Flash" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.miniMax,
    settingsConfig: {
      baseUrl: "https://api.minimaxi.com/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "MiniMax-M2.7",
          name: "MiniMax M2.7",
          contextWindow: 200000,
          cost: { input: 0.001, output: 0.004 },
        },
      ],
    },
    theme: {
      backgroundColor: "#f64551",
      textColor: "#FFFFFF",
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "minimax/MiniMax-M2.7" },
      modelCatalog: { "minimax/MiniMax-M2.7": { alias: "MiniMax" } },
    },
  },
  {
    ...PROVIDER_METADATA.miniMaxEn,
    settingsConfig: {
      baseUrl: "https://api.minimax.io/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "MiniMax-M2.7",
          name: "MiniMax M2.7",
          contextWindow: 200000,
          cost: { input: 0.001, output: 0.004 },
        },
      ],
    },
    theme: {
      backgroundColor: "#f64551",
      textColor: "#FFFFFF",
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "minimax-en/MiniMax-M2.7" },
      modelCatalog: { "minimax-en/MiniMax-M2.7": { alias: "MiniMax" } },
    },
  },
  {
    ...PROVIDER_METADATA.katCoder,
    settingsConfig: {
      baseUrl:
        "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/openai",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "KAT-Coder-Pro",
          name: "KAT-Coder Pro",
          contextWindow: 128000,
          cost: { input: 0.002, output: 0.006 },
        },
      ],
    },
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder:
          "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/openai",
        defaultValue:
          "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/openai",
        editorValue: "",
      },
      ENDPOINT_ID: {
        label: "Endpoint ID",
        placeholder: "",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "katcoder/KAT-Coder-Pro" },
      modelCatalog: { "katcoder/KAT-Coder-Pro": { alias: "KAT-Coder" } },
    },
  },
  {
    ...PROVIDER_METADATA.longcat,
    settingsConfig: {
      baseUrl: "https://api.longcat.chat/v1",
      apiKey: "",
      api: "openai-completions",
      authHeader: true,
      models: [
        {
          id: "LongCat-Flash-Chat",
          name: "LongCat Flash Chat",
          contextWindow: 128000,
          cost: { input: 0.001, output: 0.004 },
        },
      ],
    },
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.longcat.chat/v1",
        defaultValue: "https://api.longcat.chat/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "longcat/LongCat-Flash-Chat" },
      modelCatalog: { "longcat/LongCat-Flash-Chat": { alias: "LongCat" } },
    },
  },
  {
    ...PROVIDER_METADATA.baiLing,
    settingsConfig: {
      baseUrl: "https://api.tbox.cn/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "Ling-2.5-1T",
          name: "Ling 2.5 1T",
          contextWindow: 128000,
          cost: { input: 0.001, output: 0.004 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "bailing/Ling-2.5-1T" },
      modelCatalog: { "bailing/Ling-2.5-1T": { alias: "BaiLing" } },
    },
  },
  {
    ...PROVIDER_METADATA.xiaomiMiMo,
    settingsConfig: {
      baseUrl: "https://api.xiaomimimo.com/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "mimo-v2.5-pro",
          name: "MiMo V2.5 Pro",
          reasoning: true,
          input: ["text"],
          contextWindow: 1048576,
          maxTokens: 131072,
          cost: { input: 1, output: 3, cacheRead: 0.2, cacheWrite: 0 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "xiaomimimo/mimo-v2.5-pro" },
      modelCatalog: { "xiaomimimo/mimo-v2.5-pro": { alias: "MiMo" } },
    },
  },
  {
    ...PROVIDER_METADATA.xiaomiMiMoTokenPlan,
    settingsConfig: {
      baseUrl: "https://token-plan-cn.xiaomimimo.com/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "mimo-v2.5-pro",
          name: "MiMo V2.5 Pro",
          reasoning: true,
          input: ["text"],
          contextWindow: 1048576,
          maxTokens: 131072,
        },
        {
          id: "mimo-v2.5",
          name: "MiMo V2.5",
          reasoning: true,
          input: ["text", "image"],
          contextWindow: 1048576,
          maxTokens: 131072,
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "Token Plan API Key",
        placeholder: "tp-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "xiaomi-mimo-token-plan/mimo-v2.5-pro" },
      modelCatalog: {
        "xiaomi-mimo-token-plan/mimo-v2.5-pro": {
          alias: "MiMo Token Plan (China)",
        },
        "xiaomi-mimo-token-plan/mimo-v2.5": {
          alias: "MiMo Token Plan (China) Multimodal",
        },
      },
    },
  },

  // ========== Aggregators ==========
  {
    ...PROVIDER_METADATA.aiHubMix,
    settingsConfig: {
      baseUrl: "https://aihubmix.com",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
        {
          id: "claude-sonnet-4-6",
          name: "Claude Sonnet 4.6",
          contextWindow: 1000000,
          cost: { input: 3, output: 15 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "aihubmix/claude-opus-4-7",
        fallbacks: ["aihubmix/claude-sonnet-4-6"],
      },
      modelCatalog: {
        "aihubmix/claude-opus-4-7": { alias: "Opus" },
        "aihubmix/claude-sonnet-4-6": { alias: "Sonnet" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.dmxapi,
    settingsConfig: {
      baseUrl: "https://www.dmxapi.cn",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
        {
          id: "claude-sonnet-4-6",
          name: "Claude Sonnet 4.6",
          contextWindow: 1000000,
          cost: { input: 3, output: 15 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "dmxapi/claude-opus-4-7",
        fallbacks: ["dmxapi/claude-sonnet-4-6"],
      },
      modelCatalog: {
        "dmxapi/claude-opus-4-7": { alias: "Opus" },
        "dmxapi/claude-sonnet-4-6": { alias: "Sonnet" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.claudecn,
    settingsConfig: {
      baseUrl: "https://claudecn.top",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
        },
        {
          id: "claude-sonnet-4-6",
          name: "Claude Sonnet 4.6",
          contextWindow: 1000000,
        },
        {
          id: "claude-haiku-4-5",
          name: "Claude Haiku 4.5",
          contextWindow: 200000,
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "claudecn/claude-sonnet-4-6",
      },
      modelCatalog: {
        "claudecn/claude-opus-4-7": { alias: "Opus" },
        "claudecn/claude-sonnet-4-6": { alias: "Sonnet" },
        "claudecn/claude-haiku-4-5": { alias: "Haiku" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.runapi,
    settingsConfig: {
      baseUrl: "https://runapi.co",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
        },
        {
          id: "claude-sonnet-4-6",
          name: "Claude Sonnet 4.6",
          contextWindow: 1000000,
        },
        {
          id: "claude-haiku-4-5",
          name: "Claude Haiku 4.5",
          contextWindow: 200000,
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "runapi/claude-sonnet-4-6",
      },
      modelCatalog: {
        "runapi/claude-opus-4-7": { alias: "Opus" },
        "runapi/claude-sonnet-4-6": { alias: "Sonnet" },
        "runapi/claude-haiku-4-5": { alias: "Haiku" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.openRouter,
    settingsConfig: {
      baseUrl: "https://openrouter.ai/api/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "anthropic/claude-opus-4.7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
        {
          id: "anthropic/claude-sonnet-4.6",
          name: "Claude Sonnet 4.6",
          contextWindow: 1000000,
          cost: { input: 3, output: 15 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-or-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "openrouter/anthropic/claude-opus-4.7",
        fallbacks: ["openrouter/anthropic/claude-sonnet-4.6"],
      },
      modelCatalog: {
        "openrouter/anthropic/claude-opus-4.7": { alias: "Opus" },
        "openrouter/anthropic/claude-sonnet-4.6": { alias: "Sonnet" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.theRouter,
    settingsConfig: {
      baseUrl: "https://api.therouter.ai/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "anthropic/claude-sonnet-4.6",
          name: "Claude Sonnet 4.6",
          contextWindow: 1000000,
          cost: { input: 3, output: 15, cacheRead: 0.3, cacheWrite: 3.75 },
        },
        {
          id: "openai/gpt-5.3-codex",
          name: "GPT-5.3 Codex",
          contextWindow: 400000,
          cost: { input: 5, output: 40, cacheRead: 0.5 },
        },
        {
          id: "openai/gpt-5.2",
          name: "GPT-5.2",
          contextWindow: 400000,
          cost: { input: 1.75, output: 14, cacheRead: 0.175 },
        },
        {
          id: "google/gemini-3-flash-preview",
          name: "Gemini 3 Flash Preview",
          contextWindow: 1000000,
          cost: { input: 0.5, output: 3, cacheRead: 0.05 },
        },
        {
          id: "qwen/qwen3-coder-480b",
          name: "Qwen3 Coder 480B",
          contextWindow: 262144,
          cost: { input: 0.6, output: 2.35 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "therouter/anthropic/claude-sonnet-4.6",
        fallbacks: [
          "therouter/openai/gpt-5.2",
          "therouter/google/gemini-3-flash-preview",
        ],
      },
      modelCatalog: {
        "therouter/anthropic/claude-sonnet-4.6": { alias: "Sonnet" },
        "therouter/openai/gpt-5.2": { alias: "GPT-5.2" },
        "therouter/google/gemini-3-flash-preview": { alias: "Gemini Flash" },
        "therouter/openai/gpt-5.3-codex": { alias: "Codex" },
        "therouter/qwen/qwen3-coder-480b": { alias: "Qwen Coder" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.modelScope,
    settingsConfig: {
      baseUrl: "https://api-inference.modelscope.cn/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "ZhipuAI/GLM-5",
          name: "GLM-5",
          contextWindow: 128000,
          cost: { input: 0.001, output: 0.001 },
        },
      ],
    },
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api-inference.modelscope.cn/v1",
        defaultValue: "https://api-inference.modelscope.cn/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "modelscope/ZhipuAI/GLM-5" },
      modelCatalog: { "modelscope/ZhipuAI/GLM-5": { alias: "GLM" } },
    },
  },
  {
    ...PROVIDER_METADATA.siliconFlow,
    settingsConfig: {
      baseUrl: "https://api.siliconflow.cn/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "Pro/MiniMaxAI/MiniMax-M2.7",
          name: "MiniMax M2.7",
          contextWindow: 200000,
          cost: { input: 0.001, output: 0.004 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "siliconflow/Pro/MiniMaxAI/MiniMax-M2.7" },
      modelCatalog: {
        "siliconflow/Pro/MiniMaxAI/MiniMax-M2.7": { alias: "MiniMax" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.siliconFlowEn,
    settingsConfig: {
      baseUrl: "https://api.siliconflow.com/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "MiniMaxAI/MiniMax-M2.7",
          name: "MiniMax M2.7",
          contextWindow: 200000,
          cost: { input: 0.001, output: 0.004 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "siliconflow-en/MiniMaxAI/MiniMax-M2.7" },
      modelCatalog: {
        "siliconflow-en/MiniMaxAI/MiniMax-M2.7": { alias: "MiniMax" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.novitaAi,
    settingsConfig: {
      baseUrl: "https://api.novita.ai/openai",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "zai-org/glm-5",
          name: "GLM-5",
          contextWindow: 202800,
          cost: { input: 1, output: 3.2, cacheRead: 0.2 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "novita/zai-org/glm-5" },
      modelCatalog: {
        "novita/zai-org/glm-5": { alias: "GLM-5" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.nvidia,
    settingsConfig: {
      baseUrl: "https://integrate.api.nvidia.com/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "moonshotai/kimi-k2.5",
          name: "Kimi K2.5",
          contextWindow: 131072,
          cost: { input: 0.002, output: 0.006 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "nvapi-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { primary: "nvidia/moonshotai/kimi-k2.5" },
      modelCatalog: { "nvidia/moonshotai/kimi-k2.5": { alias: "Kimi" } },
    },
  },
  {
    ...PROVIDER_METADATA.pipellm,
    settingsConfig: {
      baseUrl: "https://cc-api.pipellm.ai",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "claude-opus-4-7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
        {
          id: "claude-sonnet-4-6",
          name: "claude-sonnet-4-6",
          contextWindow: 1000000,
          cost: { input: 3, output: 15 },
        },
        {
          id: "claude-haiku-4-5-20251001",
          name: "claude-haiku-4-5-20251001",
          contextWindow: 200000,
          cost: { input: 0.8, output: 4 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "pipe-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "pipellm/claude-opus-4-7",
        fallbacks: ["pipellm/claude-sonnet-4-6"],
      },
      modelCatalog: {
        "pipellm/claude-opus-4-7": { alias: "Opus" },
        "pipellm/claude-sonnet-4-6": { alias: "Sonnet" },
        "pipellm/claude-haiku-4-5-20251001": { alias: "Haiku" },
      },
    },
  },

  // ========== Third Party Partners ==========
  {
    ...PROVIDER_METADATA.packyCode,
    settingsConfig: {
      baseUrl: "https://www.packyapi.com",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
        {
          id: "claude-sonnet-4-6",
          name: "Claude Sonnet 4.6",
          contextWindow: 1000000,
          cost: { input: 3, output: 15 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "packycode/claude-opus-4-7",
        fallbacks: ["packycode/claude-sonnet-4-6"],
      },
      modelCatalog: {
        "packycode/claude-opus-4-7": { alias: "Opus" },
        "packycode/claude-sonnet-4-6": { alias: "Sonnet" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.cubence,
    settingsConfig: {
      baseUrl: "https://api.cubence.com",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
        {
          id: "claude-sonnet-4-6",
          name: "Claude Sonnet 4.6",
          contextWindow: 1000000,
          cost: { input: 3, output: 15 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "cubence/claude-opus-4-7",
        fallbacks: ["cubence/claude-sonnet-4-6"],
      },
      modelCatalog: {
        "cubence/claude-opus-4-7": { alias: "Opus" },
        "cubence/claude-sonnet-4-6": { alias: "Sonnet" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.aiGoCode,
    settingsConfig: {
      baseUrl: "https://api.aigocode.com",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
        {
          id: "claude-sonnet-4-6",
          name: "Claude Sonnet 4.6",
          contextWindow: 1000000,
          cost: { input: 3, output: 15 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "aigocode/claude-opus-4-7",
        fallbacks: ["aigocode/claude-sonnet-4-6"],
      },
      modelCatalog: {
        "aigocode/claude-opus-4-7": { alias: "Opus" },
        "aigocode/claude-sonnet-4-6": { alias: "Sonnet" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.rightCode,
    settingsConfig: {
      baseUrl: "https://www.right.codes/claude",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
        {
          id: "claude-sonnet-4-6",
          name: "Claude Sonnet 4.6",
          contextWindow: 1000000,
          cost: { input: 3, output: 15 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "rightcode/claude-opus-4-7",
        fallbacks: ["rightcode/claude-sonnet-4-6"],
      },
      modelCatalog: {
        "rightcode/claude-opus-4-7": { alias: "Opus" },
        "rightcode/claude-sonnet-4-6": { alias: "Sonnet" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.aiCodeMirror,
    settingsConfig: {
      baseUrl: "https://api.aicodemirror.com/api/claudecode",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
        {
          id: "claude-sonnet-4-6",
          name: "Claude Sonnet 4.6",
          contextWindow: 1000000,
          cost: { input: 3, output: 15 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "aicodemirror/claude-opus-4-7",
        fallbacks: ["aicodemirror/claude-sonnet-4-6"],
      },
      modelCatalog: {
        "aicodemirror/claude-opus-4-7": { alias: "Opus" },
        "aicodemirror/claude-sonnet-4-6": { alias: "Sonnet" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.crazyRouter,
    settingsConfig: {
      baseUrl: "https://cn.crazyrouter.com/v1",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
        {
          id: "claude-sonnet-4-6",
          name: "Claude Sonnet 4.6",
          contextWindow: 1000000,
          cost: { input: 3, output: 15 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "crazyrouter/claude-opus-4-7",
        fallbacks: ["crazyrouter/claude-sonnet-4-6"],
      },
      modelCatalog: {
        "crazyrouter/claude-opus-4-7": { alias: "Opus" },
        "crazyrouter/claude-sonnet-4-6": { alias: "Sonnet" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.sssAiCode,
    settingsConfig: {
      baseUrl: "https://node-hk.sssaicode.com/api",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
        {
          id: "claude-sonnet-4-6",
          name: "Claude Sonnet 4.6",
          contextWindow: 1000000,
          cost: { input: 3, output: 15 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "sssaicode/claude-opus-4-7",
        fallbacks: ["sssaicode/claude-sonnet-4-6"],
      },
      modelCatalog: {
        "sssaicode/claude-opus-4-7": { alias: "Opus" },
        "sssaicode/claude-sonnet-4-6": { alias: "Sonnet" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.compshare,
    settingsConfig: {
      baseUrl: "https://api.modelverse.cn/v1",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "compshare/claude-opus-4-7",
      },
      modelCatalog: {
        "compshare/claude-opus-4-7": { alias: "Opus" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.compshareCodingPlan,
    settingsConfig: {
      baseUrl: "https://cp.compshare.cn/v1",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "compshare-coding/claude-opus-4-7",
      },
      modelCatalog: {
        "compshare-coding/claude-opus-4-7": { alias: "Opus" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.micu,
    settingsConfig: {
      baseUrl: "https://www.micuapi.ai",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "micu/claude-opus-4-7",
      },
      modelCatalog: {
        "micu/claude-opus-4-7": { alias: "Opus" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.cTok,
    settingsConfig: {
      baseUrl: "https://api.ctok.ai",
      apiKey: "",
      api: "anthropic-messages",
      models: [
        {
          id: "claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25 },
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "ctok/claude-opus-4-7",
      },
      modelCatalog: {
        "ctok/claude-opus-4-7": { alias: "Opus" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.eFlowCode,
    settingsConfig: {
      api: "openai-responses",
      apiKey: "",
      baseUrl: "https://e-flowcode.cc/v1",
      headers: {
        "User-Agent":
          "codex_cli_rs/0.77.0 (Windows 10.0.26100; x86_64) WindowsTerminal",
      },
      models: [
        {
          contextWindow: 200000,
          cost: {
            cacheRead: 0,
            cacheWrite: 0,
            input: 0,
            output: 0,
          },
          id: "gpt-5.3-codex",
          maxTokens: 32000,
          name: "gpt-5.3-codex",
        },
        {
          id: "gpt-5.4",
          name: "gpt-5.4",
        },
        {
          id: "gpt-5.2-codex",
          name: "gpt-5.2-codex",
        },
        {
          id: "gpt-5.2",
          name: "gpt-5.2",
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "eflowcode/gpt-5.3-codex",
        fallbacks: ["eflowcode/gpt-5.4", "eflowcode/gpt-5.2-codex"],
      },
      modelCatalog: {
        "eflowcode/gpt-5.3-codex": { alias: "gpt-5.3-codex" },
        "eflowcode/gpt-5.4": { alias: "gpt-5.4" },
        "eflowcode/gpt-5.2-codex": { alias: "gpt-5.2-codex" },
        "eflowcode/gpt-5.2": { alias: "gpt-5.2" },
      },
    },
  },
  {
    ...PROVIDER_METADATA.lemonData,
    settingsConfig: {
      baseUrl: "https://api.lemondata.cc/v1",
      apiKey: "",
      api: "openai-completions",
      models: [
        {
          id: "gpt-5.4",
          name: "GPT-5.4",
          contextWindow: 400000,
        },
      ],
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: {
        primary: "lemondata/gpt-5.4",
      },
      modelCatalog: {
        "lemondata/gpt-5.4": { alias: "GPT-5.4" },
      },
    },
  },
  // ========== Cloud Providers ==========
  {
    ...PROVIDER_METADATA.awsBedrock,
    settingsConfig: {
      // 请将 us-west-2 替换为你的 AWS Region
      baseUrl: "https://bedrock-runtime.us-west-2.amazonaws.com",
      apiKey: "",
      api: "bedrock-converse-stream",
      models: [
        {
          id: "anthropic.claude-opus-4-7",
          name: "Claude Opus 4.7",
          contextWindow: 1000000,
          cost: { input: 5, output: 25, cacheRead: 0.5, cacheWrite: 6.25 },
        },
        {
          id: "anthropic.claude-sonnet-4-6",
          name: "Claude Sonnet 4.6",
          contextWindow: 1000000,
          cost: { input: 3, output: 15, cacheRead: 0.3, cacheWrite: 3.75 },
        },
        {
          id: "anthropic.claude-haiku-4-5-20251022-v1:0",
          name: "Claude Haiku 4.5",
          contextWindow: 200000,
          cost: { input: 0.8, output: 4, cacheRead: 0.08, cacheWrite: 1 },
        },
      ],
    },
  },

  // ========== Custom Template ==========
  {
    ...PROVIDER_METADATA.openaiCompatible,
    settingsConfig: {
      baseUrl: "",
      apiKey: "",
      api: "openai-completions",
      models: [],
    },
    isCustomTemplate: true,
    templateValues: {
      baseUrl: {
        label: "Base URL",
        placeholder: "https://api.example.com/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
];
