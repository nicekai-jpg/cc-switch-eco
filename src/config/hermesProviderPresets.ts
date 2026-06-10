/**
 * Hermes Agent provider presets configuration
 * Hermes uses custom_providers array in config.yaml
 */
import type {
  BaseProviderPreset,
  TemplateValueConfig,
} from "./baseProviderPreset";
import { PROVIDER_METADATA } from "./providerMetadata";

/**
 * Marker field and source values that `hermes_config.rs::get_providers`
 * injects onto each settings payload. Kept in sync with the Rust constants
 * `PROVIDER_SOURCE_FIELD` / `PROVIDER_SOURCE_CUSTOM_LIST` / `PROVIDER_SOURCE_DICT`.
 */
export const HERMES_PROVIDER_SOURCE_FIELD = "_cc_source";
export const HERMES_PROVIDER_SOURCE_CUSTOM_LIST = "custom_providers";
export const HERMES_PROVIDER_SOURCE_DICT = "providers_dict";

/**
 * True when the provider was sourced from Hermes' v12+ `providers:` dict —
 * CC Switch Eco renders those read-only and routes edits to Hermes Web UI.
 */
export function isHermesReadOnlyProvider(settingsConfig: unknown): boolean {
  if (!settingsConfig || typeof settingsConfig !== "object") {
    return false;
  }
  const marker = (settingsConfig as Record<string, unknown>)[
    HERMES_PROVIDER_SOURCE_FIELD
  ];
  return marker === HERMES_PROVIDER_SOURCE_DICT;
}

/**
 * A model entry under a Hermes custom_provider.
 *
 * Serialized to YAML as a dict keyed by `id`:
 *
 * ```yaml
 * models:
 *   anthropic/claude-opus-4-7:
 *     context_length: 200000
 * ```
 *
 * Hermes' `_VALID_CUSTOM_PROVIDER_FIELDS` (hermes_cli/config.py) does not include
 * `max_tokens` at the per-model level — writing it produces an "unknown field"
 * warning on Hermes startup. Max tokens is a per-request parameter, not a
 * provider-level config.
 */
export interface HermesModel {
  /** Model ID — becomes the YAML key and the value written to top-level model.default. */
  id: string;
  /** Optional display label (UI only, not serialized to YAML). */
  name?: string;
  /** Override the auto-detected context window. */
  context_length?: number;
}

/**
 * Top-level `model:` defaults suggested by a preset.
 *
 * Written to the YAML `model:` section when the user switches to this provider.
 * Per-model `context_length` lives on the individual `HermesModel` entries and
 * flows through `custom_providers[].models`, not this object.
 */
export interface HermesSuggestedDefaults {
  model: {
    /** Model ID for `model.default`. Typically equals `models[0].id`. */
    default: string;
    /** Value for `model.provider`. Omit to use the custom_provider name. */
    provider?: string;
  };
}

/** Hermes custom_provider protocol mode. Always written explicitly. */
export type HermesApiMode =
  | "chat_completions"
  | "anthropic_messages"
  | "codex_responses"
  | "bedrock_converse";

/** Default mode used when a provider has no stored value yet. */
export const HERMES_DEFAULT_API_MODE: HermesApiMode = "chat_completions";

/** Dropdown options for the API Mode selector. `labelKey` is looked up in i18n. */
export const hermesApiModes: Array<{
  value: HermesApiMode;
  labelKey: string;
}> = [
  { value: "chat_completions", labelKey: "hermes.form.apiModeChatCompletions" },
  {
    value: "anthropic_messages",
    labelKey: "hermes.form.apiModeAnthropicMessages",
  },
  { value: "codex_responses", labelKey: "hermes.form.apiModeCodexResponses" },
  {
    value: "bedrock_converse",
    labelKey: "hermes.form.apiModeBedrockConverse",
  },
];

export interface HermesProviderPreset extends BaseProviderPreset {
  settingsConfig: HermesProviderSettingsConfig;
  templateValues?: Record<string, TemplateValueConfig>;
  isCustomTemplate?: boolean;
  /** Optional top-level `model:` defaults written on switch. */
  suggestedDefaults?: HermesSuggestedDefaults;
}

export interface HermesProviderSettingsConfig {
  name: string;
  base_url?: string;
  api_key?: string;
  api_mode?: HermesApiMode;
  /** UI-side ordered list; serialized to YAML as a dict keyed by id. */
  models?: HermesModel[];
  /** Delay in seconds between consecutive requests to this provider. */
  rate_limit_delay?: number;
  [key: string]: unknown;
}

export const hermesProviderPresets: HermesProviderPreset[] = [
  {
    ...PROVIDER_METADATA.shengsuanyun,
    settingsConfig: {
      name: "shengsuanyun",
      base_url: "https://router.shengsuanyun.com/api/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "openai/gpt-5.4", name: "GPT-5.4" }],
    },
    suggestedDefaults: {
      model: { default: "openai/gpt-5.4", provider: "shengsuanyun" },
    },
  },
  {
    ...PROVIDER_METADATA.volcengineAgentplan,
    settingsConfig: {
      name: "ark_agentplan",
      base_url: "https://ark.cn-beijing.volces.com/api/coding",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        {
          id: "ark-code-latest",
          name: "Ark Code Latest",
        },
      ],
    },
    suggestedDefaults: {
      model: {
        default: "ark-code-latest",
        provider: "ark_agentplan",
      },
    },
  },
  {
    ...PROVIDER_METADATA.bytePlus,
    settingsConfig: {
      name: "byteplus",
      base_url: "https://ark.ap-southeast.bytepluses.com/api/coding",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        {
          id: "ark-code-latest",
          name: "Ark Code Latest",
        },
      ],
    },
    suggestedDefaults: {
      model: {
        default: "ark-code-latest",
        provider: "byteplus",
      },
    },
  },
  {
    ...PROVIDER_METADATA.douBaoSeed,
    settingsConfig: {
      name: "doubao_seed",
      base_url: "https://ark.cn-beijing.volces.com/api/compatible",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        {
          id: "doubao-seed-2-0-code-preview-latest",
          name: "Doubao Seed 2.0 Code Preview",
        },
      ],
    },
    suggestedDefaults: {
      model: {
        default: "doubao-seed-2-0-code-preview-latest",
        provider: "doubao_seed",
      },
    },
  },
  {
    ...PROVIDER_METADATA.openRouter,
    settingsConfig: {
      name: "openrouter",
      base_url: "https://openrouter.ai/api/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        {
          id: "anthropic/claude-opus-4-7",
          name: "Claude Opus 4.7",
          context_length: 1000000,
        },
        {
          id: "anthropic/claude-sonnet-4-6",
          name: "Claude Sonnet 4.6",
          context_length: 1000000,
        },
        {
          id: "anthropic/claude-haiku-4-5",
          name: "Claude Haiku 4.5",
          context_length: 200000,
        },
        {
          id: "openai/gpt-5.4",
          name: "GPT-5.4",
          context_length: 400000,
        },
        {
          id: "google/gemini-3-pro",
          name: "Gemini 3 Pro",
          context_length: 1000000,
        },
      ],
    },
    suggestedDefaults: {
      model: { default: "anthropic/claude-opus-4-7", provider: "openrouter" },
    },
  },
  {
    ...PROVIDER_METADATA.deepSeek,
    settingsConfig: {
      name: "deepseek",
      base_url: "https://api.deepseek.com",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        {
          id: "deepseek-v4-pro",
          name: "DeepSeek V4 Pro",
          context_length: 1000000,
        },
        {
          id: "deepseek-v4-flash",
          name: "DeepSeek V4 Flash",
          context_length: 1000000,
        },
      ],
    },
    iconColor: "#4D6BFE",
    suggestedDefaults: {
      model: { default: "deepseek-v4-flash", provider: "deepseek" },
    },
  },
  {
    ...PROVIDER_METADATA.togetherAi,
    settingsConfig: {
      name: "together",
      base_url: "https://api.together.xyz/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        {
          id: "Qwen/Qwen3-Coder-480B-A35B-Instruct",
          name: "Qwen3 Coder 480B",
          context_length: 262144,
        },
        {
          id: "deepseek-ai/DeepSeek-V3.2",
          name: "DeepSeek V3.2",
          context_length: 64000,
        },
        {
          id: "meta-llama/Llama-4-Maverick-17B-128E-Instruct-FP8",
          name: "Llama 4 Maverick",
          context_length: 131072,
        },
      ],
    },
    suggestedDefaults: {
      model: {
        default: "Qwen/Qwen3-Coder-480B-A35B-Instruct",
        provider: "together",
      },
    },
  },
  {
    ...PROVIDER_METADATA.nousResearch,
    settingsConfig: {
      name: "nous",
      base_url: "https://inference-api.nousresearch.com/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        {
          id: "Hermes-4-405B",
          name: "Hermes 4 405B",
          context_length: 131072,
        },
        {
          id: "Hermes-4-70B",
          name: "Hermes 4 70B",
          context_length: 131072,
        },
      ],
    },
    isOfficial: true,
    suggestedDefaults: {
      model: { default: "Hermes-4-405B", provider: "nous" },
    },
  },

  // ===== 以下为从 Claude 应用预设同步而来的供应商 =====
  // 字段映射：env.ANTHROPIC_BASE_URL → base_url；env.ANTHROPIC_AUTH_TOKEN → api_key；
  // apiFormat "anthropic"(默认) → api_mode "anthropic_messages"；
  // apiFormat "openai_chat" → api_mode "chat_completions"；
  // ANTHROPIC_MODEL / DEFAULT_HAIKU / SONNET / OPUS_MODEL 去重后塞进 models[]。
  {
    ...PROVIDER_METADATA.zhipuGlm,
    settingsConfig: {
      name: "zhipu_glm",
      base_url: "https://open.bigmodel.cn/api/paas/v4",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "glm-5", name: "GLM-5" }],
    },
    suggestedDefaults: {
      model: { default: "glm-5", provider: "zhipu_glm" },
    },
  },
  {
    ...PROVIDER_METADATA.zhipuGlmEn,
    settingsConfig: {
      name: "zhipu_glm_en",
      base_url: "https://api.z.ai/api/paas/v4",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "glm-5", name: "GLM-5" }],
    },
    suggestedDefaults: {
      model: { default: "glm-5", provider: "zhipu_glm_en" },
    },
  },
  {
    ...PROVIDER_METADATA.bailian,
    settingsConfig: {
      name: "bailian",
      base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "qwen3-coder-plus", name: "Qwen3 Coder Plus" },
        { id: "qwen3-max", name: "Qwen3 Max" },
      ],
    },
    suggestedDefaults: {
      model: { default: "qwen3-coder-plus", provider: "bailian" },
    },
  },
  {
    ...PROVIDER_METADATA.bailianForCoding,
    settingsConfig: {
      name: "bailian_coding",
      base_url: "https://coding.dashscope.aliyuncs.com/apps/anthropic",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "qwen3-coder-plus", name: "Qwen3 Coder Plus" },
        { id: "qwen3-max", name: "Qwen3 Max" },
      ],
    },
    suggestedDefaults: {
      model: { default: "qwen3-coder-plus", provider: "bailian_coding" },
    },
  },
  {
    ...PROVIDER_METADATA.kimi,
    settingsConfig: {
      name: "kimi",
      base_url: "https://api.moonshot.cn/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "kimi-k2.6", name: "Kimi K2.6" }],
    },
    suggestedDefaults: {
      model: { default: "kimi-k2.6", provider: "kimi" },
    },
  },
  {
    ...PROVIDER_METADATA.kimiForCoding,
    settingsConfig: {
      name: "kimi_coding",
      base_url: "https://api.kimi.com/coding/",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [{ id: "kimi-for-coding", name: "Kimi For Coding" }],
    },
    suggestedDefaults: {
      model: { default: "kimi-for-coding", provider: "kimi_coding" },
    },
  },
  {
    ...PROVIDER_METADATA.stepFunEn,
    settingsConfig: {
      name: "stepfun",
      base_url: "https://api.stepfun.ai/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "step-3.5-flash", name: "Step 3.5 Flash" }],
    },
    iconColor: "#005AFF",
    suggestedDefaults: {
      model: { default: "step-3.5-flash", provider: "stepfun" },
    },
  },
  {
    ...PROVIDER_METADATA.modelScope,
    settingsConfig: {
      name: "modelscope",
      base_url: "https://api-inference.modelscope.cn/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "ZhipuAI/GLM-5", name: "ZhipuAI / GLM-5" }],
    },
    suggestedDefaults: {
      model: { default: "ZhipuAI/GLM-5", provider: "modelscope" },
    },
  },
  {
    ...PROVIDER_METADATA.katCoder,
    settingsConfig: {
      name: "kat_coder",
      base_url:
        "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/claude-code-proxy",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "KAT-Coder-Pro V1", name: "KAT-Coder Pro V1" },
        { id: "KAT-Coder-Air V1", name: "KAT-Coder Air V1" },
      ],
    },
    templateValues: {
      ENDPOINT_ID: {
        label: "Vanchin Endpoint ID",
        placeholder: "ep-xxx-xxx",
        defaultValue: "",
        editorValue: "",
      },
    },
    suggestedDefaults: {
      model: { default: "KAT-Coder-Pro V1", provider: "kat_coder" },
    },
  },
  {
    ...PROVIDER_METADATA.longcat,
    settingsConfig: {
      name: "longcat",
      base_url: "https://api.longcat.chat/openai/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "LongCat-Flash-Chat", name: "LongCat Flash Chat" }],
    },
    suggestedDefaults: {
      model: { default: "LongCat-Flash-Chat", provider: "longcat" },
    },
  },
  {
    ...PROVIDER_METADATA.miniMax,
    settingsConfig: {
      name: "minimax",
      base_url: "https://api.minimaxi.com/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "MiniMax-M2.7", name: "MiniMax M2.7" }],
    },
    theme: { backgroundColor: "#f64551", textColor: "#FFFFFF" },
    suggestedDefaults: {
      model: { default: "MiniMax-M2.7", provider: "minimax" },
    },
  },
  {
    ...PROVIDER_METADATA.miniMaxEn,
    settingsConfig: {
      name: "minimax_en",
      base_url: "https://api.minimax.io/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "MiniMax-M2.7", name: "MiniMax M2.7" }],
    },
    theme: { backgroundColor: "#f64551", textColor: "#FFFFFF" },
    suggestedDefaults: {
      model: { default: "MiniMax-M2.7", provider: "minimax_en" },
    },
  },
  {
    ...PROVIDER_METADATA.baiLing,
    settingsConfig: {
      name: "bailing",
      base_url: "https://api.tbox.cn/api/anthropic",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [{ id: "Ling-2.5-1T", name: "Ling 2.5 1T" }],
    },
    suggestedDefaults: {
      model: { default: "Ling-2.5-1T", provider: "bailing" },
    },
  },
  {
    ...PROVIDER_METADATA.aiHubMix,
    settingsConfig: {
      name: "aihubmix",
      base_url: "https://aihubmix.com/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "gpt-5.4", name: "GPT-5.4" }],
    },
    suggestedDefaults: {
      model: { default: "gpt-5.4", provider: "aihubmix" },
    },
  },
  {
    ...PROVIDER_METADATA.siliconFlow,
    settingsConfig: {
      name: "siliconflow",
      base_url: "https://api.siliconflow.cn/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        {
          id: "Pro/MiniMaxAI/MiniMax-M2.7",
          name: "Pro / MiniMax M2.7",
        },
      ],
    },
    suggestedDefaults: {
      model: {
        default: "Pro/MiniMaxAI/MiniMax-M2.7",
        provider: "siliconflow",
      },
    },
  },
  {
    ...PROVIDER_METADATA.siliconFlowEn,
    settingsConfig: {
      name: "siliconflow_en",
      base_url: "https://api.siliconflow.com/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "MiniMaxAI/MiniMax-M2.7", name: "MiniMax M2.7" }],
    },
    suggestedDefaults: {
      model: {
        default: "MiniMaxAI/MiniMax-M2.7",
        provider: "siliconflow_en",
      },
    },
  },
  {
    ...PROVIDER_METADATA.dmxapi,
    settingsConfig: {
      name: "dmxapi",
      base_url: "https://www.dmxapi.cn/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "gpt-5.4", name: "GPT-5.4" }],
    },
    suggestedDefaults: {
      model: { default: "gpt-5.4", provider: "dmxapi" },
    },
  },
  {
    ...PROVIDER_METADATA.packyCode,
    settingsConfig: {
      name: "packycode",
      base_url: "https://www.packyapi.com",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "claude-opus-4-7", name: "Claude Opus 4.7" },
        { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6" },
        { id: "claude-haiku-4-5-20251001", name: "Claude Haiku 4.5" },
      ],
    },
    suggestedDefaults: {
      model: { default: "claude-opus-4-7", provider: "packycode" },
    },
  },
  {
    ...PROVIDER_METADATA.cubence,
    settingsConfig: {
      name: "cubence",
      base_url: "https://api.cubence.com",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "claude-opus-4-7", name: "Claude Opus 4.7" },
        { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6" },
        { id: "claude-haiku-4-5-20251001", name: "Claude Haiku 4.5" },
      ],
    },
    suggestedDefaults: {
      model: { default: "claude-opus-4-7", provider: "cubence" },
    },
  },
  {
    ...PROVIDER_METADATA.claudecn,
    settingsConfig: {
      name: "claudecn",
      base_url: "https://claudecn.top",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "claude-opus-4-7", name: "Claude Opus 4.7" },
        { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6" },
        { id: "claude-haiku-4-5", name: "Claude Haiku 4.5" },
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
      model: { default: "claude-sonnet-4-6", provider: "claudecn" },
    },
  },
  {
    ...PROVIDER_METADATA.runapi,
    settingsConfig: {
      name: "runapi",
      base_url: "https://runapi.co",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "claude-opus-4-7", name: "Claude Opus 4.7" },
        { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6" },
        { id: "claude-haiku-4-5", name: "Claude Haiku 4.5" },
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
      model: { default: "claude-sonnet-4-6", provider: "runapi" },
    },
  },
  {
    ...PROVIDER_METADATA.aiGoCode,
    settingsConfig: {
      name: "aigocode",
      base_url: "https://api.aigocode.com",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "claude-opus-4-7", name: "Claude Opus 4.7" },
        { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6" },
        { id: "claude-haiku-4-5-20251001", name: "Claude Haiku 4.5" },
      ],
    },
    suggestedDefaults: {
      model: { default: "claude-opus-4-7", provider: "aigocode" },
    },
  },
  {
    ...PROVIDER_METADATA.rightCode,
    settingsConfig: {
      name: "rightcode",
      base_url: "https://www.right.codes/claude",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "claude-opus-4-7", name: "Claude Opus 4.7" },
        { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6" },
        { id: "claude-haiku-4-5-20251001", name: "Claude Haiku 4.5" },
      ],
    },
    suggestedDefaults: {
      model: { default: "claude-opus-4-7", provider: "rightcode" },
    },
  },
  {
    ...PROVIDER_METADATA.aiCodeMirror,
    settingsConfig: {
      name: "aicodemirror",
      base_url: "https://api.aicodemirror.com/api/claudecode",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "claude-opus-4-7", name: "Claude Opus 4.7" },
        { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6" },
        { id: "claude-haiku-4-5-20251001", name: "Claude Haiku 4.5" },
      ],
    },
    suggestedDefaults: {
      model: { default: "claude-opus-4-7", provider: "aicodemirror" },
    },
  },
  {
    ...PROVIDER_METADATA.crazyRouter,
    settingsConfig: {
      name: "crazyrouter",
      base_url: "https://cn.crazyrouter.com",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "claude-opus-4-7", name: "Claude Opus 4.7" },
        { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6" },
        { id: "claude-haiku-4-5-20251001", name: "Claude Haiku 4.5" },
      ],
    },
    suggestedDefaults: {
      model: { default: "claude-opus-4-7", provider: "crazyrouter" },
    },
  },
  {
    ...PROVIDER_METADATA.sssAiCode,
    settingsConfig: {
      name: "sssaicode",
      base_url: "https://node-hk.sssaicode.com/api",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "claude-opus-4-7", name: "Claude Opus 4.7" },
        { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6" },
        { id: "claude-haiku-4-5-20251001", name: "Claude Haiku 4.5" },
      ],
    },
    suggestedDefaults: {
      model: { default: "claude-opus-4-7", provider: "sssaicode" },
    },
  },
  {
    ...PROVIDER_METADATA.compshare,
    settingsConfig: {
      name: "compshare",
      base_url: "https://api.modelverse.cn/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "gpt-5.4", name: "GPT-5.4" }],
    },
    suggestedDefaults: {
      model: { default: "gpt-5.4", provider: "compshare" },
    },
  },
  {
    ...PROVIDER_METADATA.compshareCodingPlan,
    settingsConfig: {
      name: "compshare_coding",
      base_url: "https://cp.compshare.cn/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "gpt-5.4", name: "GPT-5.4" }],
    },
    suggestedDefaults: {
      model: { default: "gpt-5.4", provider: "compshare_coding" },
    },
  },
  {
    ...PROVIDER_METADATA.micu,
    settingsConfig: {
      name: "micu",
      base_url: "https://www.micuapi.ai",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "claude-opus-4-7", name: "Claude Opus 4.7" },
        { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6" },
        { id: "claude-haiku-4-5-20251001", name: "Claude Haiku 4.5" },
      ],
    },
    suggestedDefaults: {
      model: { default: "claude-opus-4-7", provider: "micu" },
    },
  },
  {
    ...PROVIDER_METADATA.cTok,
    settingsConfig: {
      name: "ctok",
      base_url: "https://api.ctok.ai",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "claude-opus-4-7", name: "Claude Opus 4.7" },
        { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6" },
        { id: "claude-haiku-4-5-20251001", name: "Claude Haiku 4.5" },
      ],
    },
    suggestedDefaults: {
      model: { default: "claude-opus-4-7", provider: "ctok" },
    },
  },
  {
    ...PROVIDER_METADATA.eFlowCode,
    settingsConfig: {
      name: "eflowcode",
      base_url: "https://e-flowcode.cc",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "claude-opus-4-7", name: "Claude Opus 4.7" },
        { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6" },
        { id: "claude-haiku-4-5-20251001", name: "Claude Haiku 4.5" },
      ],
    },
    suggestedDefaults: {
      model: { default: "claude-opus-4-7", provider: "eflowcode" },
    },
  },
  {
    ...PROVIDER_METADATA.lemonData,
    settingsConfig: {
      name: "lemondata",
      base_url: "https://api.lemondata.cc/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "gpt-5.4", name: "GPT-5.4" }],
    },
    suggestedDefaults: {
      model: { default: "gpt-5.4", provider: "lemondata" },
    },
  },
  {
    ...PROVIDER_METADATA.theRouter,
    settingsConfig: {
      name: "therouter",
      base_url: "https://api.therouter.ai/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "openai/gpt-5.4", name: "GPT-5.4" },
        { id: "openai/gpt-5.4-mini", name: "GPT-5.4 mini" },
        { id: "openai/gpt-5.4-nano", name: "GPT-5.4 nano" },
      ],
    },
    suggestedDefaults: {
      model: {
        default: "openai/gpt-5.4",
        provider: "therouter",
      },
    },
  },
  {
    ...PROVIDER_METADATA.novitaAi,
    settingsConfig: {
      name: "novita",
      base_url: "https://api.novita.ai/v3/openai",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "zai-org/glm-5", name: "Zai-Org / GLM-5" }],
    },
    suggestedDefaults: {
      model: { default: "zai-org/glm-5", provider: "novita" },
    },
  },
  {
    ...PROVIDER_METADATA.nvidia,
    settingsConfig: {
      name: "nvidia",
      base_url: "https://integrate.api.nvidia.com",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "moonshotai/kimi-k2.5", name: "Moonshot Kimi K2.5" }],
    },
    suggestedDefaults: {
      model: { default: "moonshotai/kimi-k2.5", provider: "nvidia" },
    },
  },
  {
    ...PROVIDER_METADATA.pipellm,
    settingsConfig: {
      name: "pipellm",
      base_url: "https://cc-api.pipellm.ai",
      api_key: "",
      api_mode: "anthropic_messages",
      models: [
        { id: "claude-opus-4-7", name: "Claude Opus 4.7" },
        { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6" },
        {
          id: "claude-haiku-4-5-20251001",
          name: "Claude Haiku 4.5",
        },
      ],
    },
    suggestedDefaults: {
      model: { default: "claude-opus-4-7", provider: "pipellm" },
    },
  },
  {
    ...PROVIDER_METADATA.xiaomiMiMo,
    settingsConfig: {
      name: "xiaomi_mimo",
      base_url: "https://api.xiaomimimo.com/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [{ id: "mimo-v2.5-pro", name: "MiMo v2.5 Pro" }],
    },
    suggestedDefaults: {
      model: { default: "mimo-v2.5-pro", provider: "xiaomi_mimo" },
    },
  },
  {
    ...PROVIDER_METADATA.xiaomiMiMoTokenPlan,
    settingsConfig: {
      name: "xiaomi_mimo_token_plan",
      base_url: "https://token-plan-cn.xiaomimimo.com/v1",
      api_key: "",
      api_mode: "chat_completions",
      models: [
        { id: "mimo-v2.5-pro", name: "MiMo v2.5 Pro" },
        { id: "mimo-v2.5", name: "MiMo v2.5" },
      ],
    },
    suggestedDefaults: {
      model: { default: "mimo-v2.5-pro", provider: "xiaomi_mimo_token_plan" },
    },
  },
];
