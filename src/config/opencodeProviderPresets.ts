import type { OpenCodeProviderConfig } from "../types";
import type {
  BaseProviderPreset,
  TemplateValueConfig,
} from "./baseProviderPreset";
import { PROVIDER_METADATA } from "./providerMetadata";

export interface OpenCodeProviderPreset extends BaseProviderPreset {
  settingsConfig: OpenCodeProviderConfig;
  templateValues?: Record<string, TemplateValueConfig>;
  isCustomTemplate?: boolean;
}

export const opencodeNpmPackages = [
  { value: "@ai-sdk/openai", label: "OpenAI Responses" },
  { value: "@ai-sdk/openai-compatible", label: "OpenAI Compatible" },
  { value: "@ai-sdk/anthropic", label: "Anthropic" },
  { value: "@ai-sdk/amazon-bedrock", label: "Amazon Bedrock" },
  { value: "@ai-sdk/google", label: "Google (Gemini)" },
] as const;

export interface PresetModelVariant {
  id: string;
  name?: string;
  contextLimit?: number;
  outputLimit?: number;
  modalities?: { input: string[]; output: string[] };
  options?: Record<string, unknown>;
  variants?: Record<string, Record<string, unknown>>;
}

export const OPENCODE_PRESET_MODEL_VARIANTS: Record<
  string,
  PresetModelVariant[]
> = {
  "@ai-sdk/openai-compatible": [
    {
      id: "MiniMax-M2.7",
      name: "MiniMax M2.7",
      contextLimit: 204800,
      outputLimit: 131072,
      modalities: { input: ["text"], output: ["text"] },
    },
    {
      id: "glm-5",
      name: "GLM 5",
      contextLimit: 204800,
      outputLimit: 131072,
      modalities: { input: ["text"], output: ["text"] },
    },
    {
      id: "kimi-k2.6",
      name: "Kimi K2.6",
      contextLimit: 262144,
      outputLimit: 262144,
      modalities: { input: ["text", "image", "video"], output: ["text"] },
    },
    {
      id: "step-3.5-flash-2603",
      name: "Step 3.5 Flash 2603",
      contextLimit: 262144,
    },
    {
      id: "step-3.5-flash",
      name: "Step 3.5 Flash",
      contextLimit: 262144,
    },
  ],
  "@ai-sdk/google": [
    {
      id: "gemini-2.5-flash-lite",
      name: "Gemini 2.5 Flash Lite",
      contextLimit: 1048576,
      outputLimit: 65536,
      modalities: {
        input: ["text", "image", "pdf", "video", "audio"],
        output: ["text"],
      },
      variants: {
        auto: {
          thinkingConfig: { includeThoughts: true, thinkingBudget: -1 },
        },
        "no-thinking": { thinkingConfig: { thinkingBudget: 0 } },
      },
    },
    {
      id: "gemini-3-flash-preview",
      name: "Gemini 3 Flash Preview",
      contextLimit: 1048576,
      outputLimit: 65536,
      modalities: {
        input: ["text", "image", "pdf", "video", "audio"],
        output: ["text"],
      },
      variants: {
        minimal: {
          thinkingConfig: { includeThoughts: true, thinkingLevel: "minimal" },
        },
        low: {
          thinkingConfig: { includeThoughts: true, thinkingLevel: "low" },
        },
        medium: {
          thinkingConfig: { includeThoughts: true, thinkingLevel: "medium" },
        },
        high: {
          thinkingConfig: { includeThoughts: true, thinkingLevel: "high" },
        },
      },
    },
    {
      id: "gemini-3-pro-preview",
      name: "Gemini 3 Pro Preview",
      contextLimit: 1048576,
      outputLimit: 65536,
      modalities: {
        input: ["text", "image", "pdf", "video", "audio"],
        output: ["text"],
      },
      variants: {
        low: {
          thinkingConfig: { includeThoughts: true, thinkingLevel: "low" },
        },
        high: {
          thinkingConfig: { includeThoughts: true, thinkingLevel: "high" },
        },
      },
    },
  ],
  "@ai-sdk/openai": [
    {
      id: "gpt-5.4",
      name: "GPT-5.4",
      contextLimit: 400000,
      outputLimit: 128000,
      modalities: { input: ["text", "image"], output: ["text"] },
      variants: {
        low: {
          reasoningEffort: "low",
          reasoningSummary: "auto",
          textVerbosity: "medium",
        },
        medium: {
          reasoningEffort: "medium",
          reasoningSummary: "auto",
          textVerbosity: "medium",
        },
        high: {
          reasoningEffort: "high",
          reasoningSummary: "auto",
          textVerbosity: "medium",
        },
        xhigh: {
          reasoningEffort: "xhigh",
          reasoningSummary: "auto",
          textVerbosity: "medium",
        },
      },
    },
  ],
  "@ai-sdk/amazon-bedrock": [
    {
      id: "global.anthropic.claude-opus-4-7",
      name: "Claude Opus 4.7",
      contextLimit: 1000000,
      outputLimit: 128000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
    },
    {
      id: "global.anthropic.claude-sonnet-4-6",
      name: "Claude Sonnet 4.6",
      contextLimit: 1000000,
      outputLimit: 64000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
    },
    {
      id: "global.anthropic.claude-haiku-4-5-20251001-v1:0",
      name: "Claude Haiku 4.5",
      contextLimit: 200000,
      outputLimit: 64000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
    },
    {
      id: "us.amazon.nova-pro-v1:0",
      name: "Amazon Nova Pro",
      contextLimit: 300000,
      outputLimit: 5000,
      modalities: { input: ["text", "image"], output: ["text"] },
    },
    {
      id: "us.meta.llama4-maverick-17b-instruct-v1:0",
      name: "Meta Llama 4 Maverick",
      contextLimit: 131072,
      outputLimit: 131072,
      modalities: { input: ["text"], output: ["text"] },
    },
    {
      id: "us.deepseek.r1-v1:0",
      name: "DeepSeek R1",
      contextLimit: 131072,
      outputLimit: 131072,
      modalities: { input: ["text"], output: ["text"] },
    },
  ],
  "@ai-sdk/anthropic": [
    {
      id: "claude-sonnet-4-5-20250929",
      name: "Claude Sonnet 4.5",
      contextLimit: 200000,
      outputLimit: 64000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
      variants: {
        low: { effort: "low" },
        medium: { effort: "medium" },
        high: { effort: "high" },
      },
    },
    {
      id: "claude-opus-4-5-20251101",
      name: "Claude Opus 4.5",
      contextLimit: 200000,
      outputLimit: 64000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
      variants: {
        low: { thinking: { budgetTokens: 5000, type: "enabled" } },
        medium: { thinking: { budgetTokens: 13000, type: "enabled" } },
        high: { thinking: { budgetTokens: 18000, type: "enabled" } },
      },
    },
    {
      id: "claude-opus-4-7",
      name: "Claude Opus 4.7",
      contextLimit: 1000000,
      outputLimit: 128000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
      variants: {
        low: { effort: "low" },
        medium: { effort: "medium" },
        high: { effort: "high" },
        max: { effort: "max" },
      },
    },
    {
      id: "claude-haiku-4-5-20251001",
      name: "Claude Haiku 4.5",
      contextLimit: 200000,
      outputLimit: 64000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
    },
    {
      id: "gemini-claude-opus-4-5-thinking",
      name: "Antigravity - Claude Opus 4.5",
      contextLimit: 200000,
      outputLimit: 64000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
      variants: {
        low: { effort: "low" },
        medium: { effort: "medium" },
        high: { effort: "high" },
      },
    },
    {
      id: "gemini-claude-sonnet-4-5-thinking",
      name: "Antigravity - Claude Sonnet 4.5",
      contextLimit: 200000,
      outputLimit: 64000,
      modalities: { input: ["text", "image", "pdf"], output: ["text"] },
      variants: {
        low: { thinking: { budgetTokens: 5000, type: "enabled" } },
        medium: { thinking: { budgetTokens: 13000, type: "enabled" } },
        high: { thinking: { budgetTokens: 18000, type: "enabled" } },
      },
    },
  ],
};

/**
 * Look up preset metadata for a model by npm package and model ID.
 * Returns enrichment fields (options, limit, modalities) that can be
 * merged into a model definition when the user's config doesn't already
 * provide them.
 */
export function getPresetModelDefaults(
  npm: string,
  modelId: string,
): PresetModelVariant | undefined {
  const models = OPENCODE_PRESET_MODEL_VARIANTS[npm];
  if (!models) return undefined;
  return models.find((m) => m.id === modelId);
}

export const opencodeProviderPresets: OpenCodeProviderPreset[] = [
  {
    ...PROVIDER_METADATA.shengsuanyun,
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "Shengsuanyun",
      options: {
        baseURL: "https://router.shengsuanyun.com/api/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "claude-opus-4-7": { name: "Claude Opus 4.7" },
        "claude-sonnet-4-6": { name: "Claude Sonnet 4.6" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.volcengineAgentplan,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "火山Agentplan",
      options: {
        baseURL: "https://ark.cn-beijing.volces.com/api/coding/v3",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "ark-code-latest": {
          name: "Ark Code Latest",
        },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.bytePlus,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "BytePlus",
      options: {
        baseURL: "https://ark.ap-southeast.bytepluses.com/api/coding/v3",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "ark-code-latest": {
          name: "Ark Code Latest",
        },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.douBaoSeed,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "DouBaoSeed",
      options: {
        baseURL: "https://ark.cn-beijing.volces.com/api/v3",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "doubao-seed-2-0-code-preview-latest": {
          name: "Doubao Seed Code Preview",
        },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.deepSeek,
    apiKeyUrl: "https://platform.deepseek.com/api_keys",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      options: {
        baseURL: "https://api.deepseek.com/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "deepseek-v4-pro": { name: "DeepSeek V4 Pro" },
        "deepseek-v4-flash": { name: "DeepSeek V4 Flash" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.zhipuGlm,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Zhipu GLM",
      options: {
        baseURL: "https://open.bigmodel.cn/api/paas/v4",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "glm-5": { name: "GLM-5" },
      },
    },
    templateValues: {
      baseURL: {
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
  },
  {
    ...PROVIDER_METADATA.zhipuGlmEn,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Zhipu GLM en",
      options: {
        baseURL: "https://api.z.ai/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "glm-5": { name: "GLM-5" },
      },
    },
    templateValues: {
      baseURL: {
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
  },
  {
    ...PROVIDER_METADATA.bailian,
    apiKeyUrl: "https://bailian.console.aliyun.com/#/api-key",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Bailian",
      options: {
        baseURL: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {},
    },
    templateValues: {
      baseURL: {
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
  },
  {
    ...PROVIDER_METADATA.kimiK26,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Kimi k2.6",
      options: {
        baseURL: "https://api.moonshot.cn/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "kimi-k2.6": { name: "Kimi K2.6" },
      },
    },
    templateValues: {
      baseURL: {
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
  },
  {
    ...PROVIDER_METADATA.kimiForCoding,
    apiKeyUrl: "https://platform.moonshot.cn/console/api-keys",
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "Kimi For Coding",
      options: {
        baseURL: "https://api.kimi.com/coding/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "kimi-for-coding": { name: "Kimi For Coding" },
      },
    },
    templateValues: {
      baseURL: {
        label: "Base URL",
        placeholder: "https://api.kimi.com/coding/v1",
        defaultValue: "https://api.kimi.com/coding/v1",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.stepFun,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "StepFun",
      options: {
        baseURL: "https://api.stepfun.com/step_plan/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "step-3.5-flash-2603": { name: "Step 3.5 Flash 2603" },
        "step-3.5-flash": { name: "Step 3.5 Flash" },
      },
    },
    templateValues: {
      baseURL: {
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
  },
  {
    ...PROVIDER_METADATA.stepFunEn,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "StepFun en",
      options: {
        baseURL: "https://api.stepfun.ai/step_plan/v1",
        apiKey: "",
      },
      models: {
        "step-3.5-flash-2603": { name: "Step 3.5 Flash 2603" },
        "step-3.5-flash": { name: "Step 3.5 Flash" },
      },
    },
    templateValues: {
      baseURL: {
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
  },
  {
    ...PROVIDER_METADATA.stepFunStepPlan,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "StepFun Step Plan",
      options: {
        baseURL: "https://api.stepfun.com/step_plan/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "step-3.5-flash": { name: "Step 3.5 Flash" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "step-...",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.modelScope,
    apiKeyUrl: "https://modelscope.cn/my/myaccesstoken",
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "ModelScope",
      options: {
        baseURL: "https://api-inference.modelscope.cn/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "ZhipuAI/GLM-5": { name: "GLM-5" },
      },
    },
    templateValues: {
      baseURL: {
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
  },
  {
    ...PROVIDER_METADATA.katCoder,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "KAT-Coder",
      options: {
        baseURL:
          "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/openai",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "KAT-Coder-Pro": { name: "KAT-Coder Pro" },
      },
    },
    templateValues: {
      baseURL: {
        label: "Base URL",
        placeholder:
          "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/openai",
        defaultValue:
          "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/openai",
        editorValue: "",
      },
      ENDPOINT_ID: {
        label: "Vanchin Endpoint ID",
        placeholder: "ep-xxx-xxx",
        defaultValue: "",
        editorValue: "",
      },
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.longcat,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Longcat",
      options: {
        baseURL: "https://api.longcat.chat/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "LongCat-Flash-Chat": { name: "LongCat Flash Chat" },
      },
    },
    templateValues: {
      baseURL: {
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
  },
  {
    ...PROVIDER_METADATA.miniMax,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "MiniMax",
      options: {
        baseURL: "https://api.minimaxi.com/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "MiniMax-M2.7": { name: "MiniMax M2.7" },
      },
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
  },
  {
    ...PROVIDER_METADATA.miniMaxEn,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "MiniMax en",
      options: {
        baseURL: "https://api.minimax.io/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "MiniMax-M2.7": { name: "MiniMax M2.7" },
      },
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
  },
  {
    ...PROVIDER_METADATA.baiLing,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "BaiLing",
      options: {
        baseURL: "https://api.tbox.cn/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "Ling-2.5-1T": { name: "Ling 2.5-1T" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.xiaomiMiMo,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Xiaomi MiMo",
      options: {
        baseURL: "https://api.xiaomimimo.com/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "mimo-v2.5-pro": {
          name: "MiMo V2.5 Pro",
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text"], output: ["text"] },
        },
        "mimo-v2.5": {
          name: "MiMo V2.5",
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.xiaomiMiMoTokenPlan,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Xiaomi MiMo Token Plan (China)",
      options: {
        baseURL: "https://token-plan-cn.xiaomimimo.com/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "mimo-v2.5-pro": {
          name: "MiMo V2.5 Pro",
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text"], output: ["text"] },
        },
        "mimo-v2.5": {
          name: "MiMo V2.5",
          limit: { context: 1048576, output: 131072 },
          modalities: { input: ["text", "image"], output: ["text"] },
        },
      },
    },
    templateValues: {
      apiKey: {
        label: "Token Plan API Key",
        placeholder: "tp-...",
        editorValue: "",
      },
    },
  },

  {
    ...PROVIDER_METADATA.aiHubMix,
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "AiHubMix",
      options: {
        baseURL: "https://aihubmix.com/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "claude-sonnet-4-6": { name: "Claude Sonnet 4.6" },
        "claude-opus-4-7": { name: "Claude Opus 4.7" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.dmxapi,
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "DMXAPI",
      options: {
        baseURL: "https://www.dmxapi.cn/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "claude-sonnet-4-6": { name: "Claude Sonnet 4.6" },
        "claude-opus-4-7": { name: "Claude Opus 4.7" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.openRouter,
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "OpenRouter",
      options: {
        baseURL: "https://openrouter.ai/api/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "anthropic/claude-sonnet-4.6": { name: "Claude Sonnet 4.6" },
        "anthropic/claude-opus-4.7": { name: "Claude Opus 4.7" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-or-...",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.theRouter,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "TheRouter",
      options: {
        baseURL: "https://api.therouter.ai/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "anthropic/claude-sonnet-4.6": { name: "Claude Sonnet 4.6" },
        "openai/gpt-5.3-codex": { name: "GPT-5.3 Codex" },
        "openai/gpt-5.2": { name: "GPT-5.2" },
        "google/gemini-3-flash-preview": {
          name: "Gemini 3 Flash Preview",
        },
        "qwen/qwen3-coder-480b": { name: "Qwen3 Coder 480B" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.novitaAi,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Novita AI",
      options: {
        baseURL: "https://api.novita.ai/openai",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "zai-org/glm-5": { name: "GLM-5" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.nvidia,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "Nvidia",
      options: {
        baseURL: "https://integrate.api.nvidia.com/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "moonshotai/kimi-k2.5": { name: "Kimi K2.5" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.pipellm,
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "PIPELLM",
      options: {
        baseURL: "https://cc-api.pipellm.ai",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "claude-opus-4-7": { name: "claude-opus-4-7" },
        "claude-sonnet-4-6": { name: "claude-sonnet-4-6" },
        "claude-haiku-4-5-20251001": { name: "claude-haiku-4-5-20251001" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "pipe-...",
        editorValue: "",
      },
    },
  },

  {
    ...PROVIDER_METADATA.packyCode,
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "PackyCode",
      options: {
        baseURL: "https://www.packyapi.com/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "claude-sonnet-4-6": { name: "Claude Sonnet 4.6" },
        "claude-opus-4-7": { name: "Claude Opus 4.7" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.cubence,
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "Cubence",
      options: {
        baseURL: "https://api.cubence.com/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "claude-sonnet-4-6": { name: "Claude Sonnet 4.6" },
        "claude-opus-4-7": { name: "Claude Opus 4.7" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.aiGoCode,
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "AIGoCode",
      options: {
        baseURL: "https://api.aigocode.com",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "claude-sonnet-4-6": { name: "Claude Sonnet 4.6" },
        "claude-opus-4-7": { name: "Claude Opus 4.7" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.rightCode,
    settingsConfig: {
      npm: "@ai-sdk/openai",
      name: "RightCode",
      options: {
        baseURL: "https://right.codes/codex/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "gpt-5.4": { name: "GPT-5.4" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.aiCodeMirror,
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "AICodeMirror",
      options: {
        baseURL: "https://api.aicodemirror.com/api/claudecode",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "claude-sonnet-4.6": { name: "Claude Sonnet 4.6" },
        "claude-opus-4.7": { name: "Claude Opus 4.7" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.claudecn,
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "ClaudeCN",
      options: {
        baseURL: "https://claudecn.top",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "claude-sonnet-4-6": { name: "Claude Sonnet 4.6" },
        "claude-opus-4-7": { name: "Claude Opus 4.7" },
        "claude-haiku-4-5": { name: "Claude Haiku 4.5" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.runapi,
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "RunAPI",
      options: {
        baseURL: "https://runapi.co",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "claude-sonnet-4-6": { name: "Claude Sonnet 4.6" },
        "claude-opus-4-7": { name: "Claude Opus 4.7" },
        "claude-haiku-4-5": { name: "Claude Haiku 4.5" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.crazyRouter,
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "CrazyRouter",
      options: {
        baseURL: "https://cn.crazyrouter.com",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "claude-sonnet-4-6": { name: "Claude Sonnet 4.6" },
        "claude-opus-4-7": { name: "Claude Opus 4.7" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.sssAiCode,
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "SSSAiCode",
      options: {
        baseURL: "https://node-hk.sssaicode.com/api/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "claude-sonnet-4-6": { name: "Claude Sonnet 4.6" },
        "claude-opus-4-7": { name: "Claude Opus 4.7" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.micu,
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "Micu",
      options: {
        baseURL: "https://www.micuapi.ai/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "claude-opus-4-7": { name: "Claude Opus 4.7" },
        "claude-sonnet-4-6": { name: "Claude Sonnet 4.6" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.cTok,
    settingsConfig: {
      npm: "@ai-sdk/anthropic",
      name: "CTok",
      options: {
        baseURL: "https://api.ctok.ai/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "claude-opus-4-7": { name: "Claude Opus 4.7" },
        "claude-sonnet-4-6": { name: "Claude Sonnet 4.6" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.eFlowCode,
    settingsConfig: {
      npm: "@ai-sdk/openai",
      options: {
        apiKey: "",
        baseURL: "https://e-flowcode.cc/v1",
      },
      models: {
        "gpt-5.2-codex": {
          name: "gpt-5.2-codex",
        },
        "gpt-5.3-codex": {
          name: "gpt-5.3-codex",
        },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "sk-...",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.lemonData,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      name: "LemonData",
      options: {
        baseURL: "https://api.lemondata.cc/v1",
        apiKey: "",
        setCacheKey: true,
      },
      models: {
        "gpt-5.4": { name: "GPT-5.4" },
      },
    },
    templateValues: {
      apiKey: {
        label: "API Key",
        placeholder: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.awsBedrock,
    settingsConfig: {
      npm: "@ai-sdk/amazon-bedrock",
      name: "AWS Bedrock",
      options: {
        region: "${region}",
        accessKeyId: "${accessKeyId}",
        secretAccessKey: "${secretAccessKey}",
        setCacheKey: true,
      },
      models: {
        "global.anthropic.claude-opus-4-7": { name: "Claude Opus 4.7" },
        "global.anthropic.claude-sonnet-4-6": {
          name: "Claude Sonnet 4.6",
        },
        "global.anthropic.claude-haiku-4-5-20251001-v1:0": {
          name: "Claude Haiku 4.5",
        },
        "us.amazon.nova-pro-v1:0": { name: "Amazon Nova Pro" },
        "us.meta.llama4-maverick-17b-instruct-v1:0": {
          name: "Meta Llama 4 Maverick",
        },
        "us.deepseek.r1-v1:0": { name: "DeepSeek R1" },
      },
    },
    templateValues: {
      region: {
        label: "AWS Region",
        placeholder: "us-west-2",
        defaultValue: "us-west-2",
        editorValue: "us-west-2",
      },
      accessKeyId: {
        label: "Access Key ID",
        placeholder: "AKIA...",
        editorValue: "",
      },
      secretAccessKey: {
        label: "Secret Access Key",
        placeholder: "your-secret-key",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.openaiCompatible,
    settingsConfig: {
      npm: "@ai-sdk/openai-compatible",
      options: {
        baseURL: "",
        apiKey: "",
        setCacheKey: true,
      },
      models: {},
    },
    isCustomTemplate: true,
    templateValues: {
      baseURL: {
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

  {
    ...PROVIDER_METADATA.ohMyOpenCode,
    settingsConfig: {
      npm: "",
      options: {},
      models: {},
    },
    isCustomTemplate: true,
  },
  {
    ...PROVIDER_METADATA.ohMyOpenCodeSlim,
    settingsConfig: {
      npm: "",
      options: {},
      models: {},
    },
    isCustomTemplate: true,
  },
];
