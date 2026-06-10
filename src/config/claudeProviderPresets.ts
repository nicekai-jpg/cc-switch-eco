/**
 * 预设供应商配置模板
 */
import type {
  BaseProviderPreset,
  TemplateValueConfig,
} from "./baseProviderPreset";
import { PROVIDER_METADATA } from "./providerMetadata";

// Re-export 供外部导入保持向后兼容
export type { TemplateValueConfig, PresetTheme } from "./baseProviderPreset";

export interface ProviderPreset extends BaseProviderPreset {
  settingsConfig: object;
  // 指定该预设所使用的 API Key 字段名（默认 ANTHROPIC_AUTH_TOKEN）
  apiKeyField?: "ANTHROPIC_AUTH_TOKEN" | "ANTHROPIC_API_KEY";
  // 模板变量定义，用于动态替换配置中的值
  templateValues?: Record<string, TemplateValueConfig>; // editorValue 存储编辑器中的实时输入值
  // 请求地址候选列表（用于地址管理/测速）
  endpointCandidates?: string[];

  // Claude API 格式（仅 Claude 供应商使用）
  // - "anthropic" (默认): Anthropic Messages API 格式，直接透传
  // - "openai_chat": OpenAI Chat Completions 格式，需要格式转换
  // - "openai_responses": OpenAI Responses API 格式，需要格式转换
  // - "gemini_native": Gemini Native generateContent API 格式，需要格式转换
  apiFormat?:
    | "anthropic"
    | "openai_chat"
    | "openai_responses"
    | "gemini_native";

  // 供应商类型标识（用于特殊供应商检测）
  // - "github_copilot": GitHub Copilot 供应商（需要 OAuth 认证）
  // - "codex_oauth": OpenAI Codex via ChatGPT Plus/Pro 反代（需要 OAuth 认证）
  providerType?: "github_copilot" | "codex_oauth";

  // 是否需要 OAuth 认证（而非 API Key）
  requiresOAuth?: boolean;

  // 是否在 UI 中隐藏该预设（预设仍存在，仅不在列表中显示）
  hidden?: boolean;

  // 获取模型列表使用的完整 URL（覆写自动候选逻辑）
  // 缺省时后端基于 baseURL 自动尝试 /v1/models、/models 以及剥离已知兼容子路径后的变体。
  modelsUrl?: string;
}

export const providerPresets: ProviderPreset[] = [
  {
    ...PROVIDER_METADATA.claudeOfficial,
    settingsConfig: {
      env: {},
    },
    isOfficial: true, // 明确标识为官方预设
    theme: {
      icon: "claude",
      backgroundColor: "#D97757",
      textColor: "#FFFFFF",
    },
  },
  {
    ...PROVIDER_METADATA.shengsuanyun,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://router.shengsuanyun.com/api",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.patewayAi,
    apiKeyField: "ANTHROPIC_API_KEY",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.pateway.ai",
        ANTHROPIC_API_KEY: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.volcengineAgentplan,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://ark.cn-beijing.volces.com/api/coding",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "ark-code-latest",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "ark-code-latest",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "ark-code-latest",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "ark-code-latest",
      },
    },
  },
  {
    ...PROVIDER_METADATA.bytePlus,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://ark.ap-southeast.bytepluses.com/api/coding",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "ark-code-latest",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "ark-code-latest",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "ark-code-latest",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "ark-code-latest",
      },
    },
  },
  {
    ...PROVIDER_METADATA.douBaoSeed,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://ark.cn-beijing.volces.com/api/compatible",
        ANTHROPIC_AUTH_TOKEN: "",
        API_TIMEOUT_MS: "3000000",
        ANTHROPIC_MODEL: "doubao-seed-2-0-code-preview-latest",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "doubao-seed-2-0-code-preview-latest",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "doubao-seed-2-0-code-preview-latest",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "doubao-seed-2-0-code-preview-latest",
      },
    },
  },
  {
    ...PROVIDER_METADATA.geminiNative,
    apiKeyField: "ANTHROPIC_API_KEY",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://generativelanguage.googleapis.com",
        ANTHROPIC_API_KEY: "",
        ANTHROPIC_MODEL: "gemini-3.1-pro",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "gemini-3-flash",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "gemini-3.1-pro",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "gemini-3.1-pro",
      },
    },
    apiFormat: "gemini_native",
    endpointCandidates: ["https://generativelanguage.googleapis.com"],
  },
  {
    ...PROVIDER_METADATA.deepSeek,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.deepseek.com/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "deepseek-v4-pro",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "deepseek-v4-flash",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "deepseek-v4-pro",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "deepseek-v4-pro",
      },
    },
    // Anthropic 兼容层挂在 /anthropic 子路径；/models 是根上独立端点
    modelsUrl: "https://api.deepseek.com/models",
  },
  {
    ...PROVIDER_METADATA.openCodeGoDeepSeek,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://opencode.ai/zen/go",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "deepseek-v4-flash",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "deepseek-v4-flash",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "deepseek-v4-flash",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "deepseek-v4-flash",
      },
    },
    apiFormat: "openai_chat",
    endpointCandidates: ["https://opencode.ai/zen/go"],
  },
  {
    ...PROVIDER_METADATA.zhipuGlm,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://open.bigmodel.cn/api/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "glm-5",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "glm-5",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "glm-5",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "glm-5",
      },
    },
  },
  {
    ...PROVIDER_METADATA.zhipuGlmEn,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.z.ai/api/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "glm-5",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "glm-5",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "glm-5",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "glm-5",
      },
    },
  },
  {
    ...PROVIDER_METADATA.baiduQianfan,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://qianfan.baidubce.com/anthropic/coding",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "qianfan-code-latest",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "qianfan-code-latest",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "qianfan-code-latest",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "qianfan-code-latest",
      },
    },
    endpointCandidates: ["https://qianfan.baidubce.com/anthropic/coding"],
  },
  {
    ...PROVIDER_METADATA.bailian,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://dashscope.aliyuncs.com/apps/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.bailianForCoding,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://coding.dashscope.aliyuncs.com/apps/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.kimi,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.moonshot.cn/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "kimi-k2.6",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "kimi-k2.6",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "kimi-k2.6",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "kimi-k2.6",
      },
    },
  },
  {
    ...PROVIDER_METADATA.kimiForCoding,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.kimi.com/coding/",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.stepFun,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.stepfun.com/step_plan",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "step-3.5-flash-2603",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "step-3.5-flash-2603",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "step-3.5-flash-2603",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "step-3.5-flash-2603",
      },
    },
    endpointCandidates: ["https://api.stepfun.com/step_plan"],
  },
  {
    ...PROVIDER_METADATA.stepFunEn,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.stepfun.ai/step_plan",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "step-3.5-flash-2603",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "step-3.5-flash-2603",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "step-3.5-flash-2603",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "step-3.5-flash-2603",
      },
    },
    endpointCandidates: ["https://api.stepfun.ai/step_plan"],
  },
  {
    ...PROVIDER_METADATA.modelScope,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api-inference.modelscope.cn",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "ZhipuAI/GLM-5",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "ZhipuAI/GLM-5",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "ZhipuAI/GLM-5",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "ZhipuAI/GLM-5",
      },
    },
  },
  {
    ...PROVIDER_METADATA.katCoder,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/claude-code-proxy",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "KAT-Coder-Pro V1",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "KAT-Coder-Air V1",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "KAT-Coder-Pro V1",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "KAT-Coder-Pro V1",
      },
    },
    templateValues: {
      ENDPOINT_ID: {
        label: "Vanchin Endpoint ID",
        placeholder: "ep-xxx-xxx",
        defaultValue: "",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.longcat,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.longcat.chat/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "LongCat-Flash-Chat",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "LongCat-Flash-Chat",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "LongCat-Flash-Chat",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "LongCat-Flash-Chat",
        CLAUDE_CODE_MAX_OUTPUT_TOKENS: "6000",
        CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: 1,
      },
    },
  },
  {
    ...PROVIDER_METADATA.miniMax,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.minimaxi.com/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        API_TIMEOUT_MS: "3000000",
        CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: 1,
        ANTHROPIC_MODEL: "MiniMax-M2.7",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "MiniMax-M2.7",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "MiniMax-M2.7",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "MiniMax-M2.7",
      },
    },
    theme: {
      backgroundColor: "#f64551",
      textColor: "#FFFFFF",
    },
  },
  {
    ...PROVIDER_METADATA.miniMaxEn,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.minimax.io/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        API_TIMEOUT_MS: "3000000",
        CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: 1,
        ANTHROPIC_MODEL: "MiniMax-M2.7",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "MiniMax-M2.7",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "MiniMax-M2.7",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "MiniMax-M2.7",
      },
    },
    theme: {
      backgroundColor: "#f64551",
      textColor: "#FFFFFF",
    },
  },
  {
    ...PROVIDER_METADATA.baiLing,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.tbox.cn/api/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "Ling-2.5-1T",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "Ling-2.5-1T",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "Ling-2.5-1T",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "Ling-2.5-1T",
      },
    },
  },
  {
    ...PROVIDER_METADATA.aiHubMix,
    // 说明：该供应商使用 ANTHROPIC_API_KEY（而非 ANTHROPIC_AUTH_TOKEN）
    apiKeyField: "ANTHROPIC_API_KEY",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://aihubmix.com",
        ANTHROPIC_API_KEY: "",
      },
    },
    // 请求地址候选（用于地址管理/测速），用户可自行选择/覆盖
    endpointCandidates: ["https://aihubmix.com", "https://api.aihubmix.com"],
  },
  {
    ...PROVIDER_METADATA.siliconFlow,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.siliconflow.cn",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "Pro/MiniMaxAI/MiniMax-M2.7",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "Pro/MiniMaxAI/MiniMax-M2.7",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "Pro/MiniMaxAI/MiniMax-M2.7",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "Pro/MiniMaxAI/MiniMax-M2.7",
      },
    },
  },
  {
    ...PROVIDER_METADATA.siliconFlowEn,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.siliconflow.com",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "MiniMaxAI/MiniMax-M2.7",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "MiniMaxAI/MiniMax-M2.7",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "MiniMaxAI/MiniMax-M2.7",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "MiniMaxAI/MiniMax-M2.7",
      },
    },
  },
  {
    ...PROVIDER_METADATA.dmxapi,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://www.dmxapi.cn",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
    // 请求地址候选（用于地址管理/测速），用户可自行选择/覆盖
    endpointCandidates: ["https://www.dmxapi.cn", "https://api.dmxapi.cn"],
  },
  {
    ...PROVIDER_METADATA.packyCode,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://www.packyapi.com",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
    // 请求地址候选（用于地址管理/测速）
    endpointCandidates: [
      "https://www.packyapi.com",
      "https://api-slb.packyapi.com",
    ],
  },
  {
    ...PROVIDER_METADATA.claudeapi,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://gw.claudeapi.com",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.claudecn,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://claudecn.top",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.runapi,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://runapi.co",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.relaxyCode,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://www.relaxycode.com",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.cubence,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.cubence.com",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
    endpointCandidates: [
      "https://api.cubence.com",
      "https://api-cf.cubence.com",
      "https://api-dmit.cubence.com",
      "https://api-bwg.cubence.com",
    ],
  },
  {
    ...PROVIDER_METADATA.aiGoCode,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.aigocode.com",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
    // 请求地址候选（用于地址管理/测速）
    endpointCandidates: ["https://api.aigocode.com"],
  },
  {
    ...PROVIDER_METADATA.rightCode,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://www.right.codes/claude",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.aiCodeMirror,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.aicodemirror.com/api/claudecode",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
    endpointCandidates: [
      "https://api.aicodemirror.com/api/claudecode",
      "https://api.claudecode.net.cn/api/claudecode",
    ],
  },
  {
    ...PROVIDER_METADATA.crazyRouter,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://cn.crazyrouter.com",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
    endpointCandidates: ["https://cn.crazyrouter.com"],
  },
  {
    ...PROVIDER_METADATA.sssAiCode,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://node-hk.sssaicode.com/api",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
    endpointCandidates: [
      "https://node-hk.sssaicode.com/api",
      "https://claude2.sssaicode.com/api",
      "https://anti.sssaicode.com/api",
    ],
  },
  {
    ...PROVIDER_METADATA.compshare,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.modelverse.cn",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
    endpointCandidates: ["https://api.modelverse.cn"],
  },
  {
    ...PROVIDER_METADATA.compshareCodingPlan,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://cp.compshare.cn",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
    endpointCandidates: ["https://cp.compshare.cn"],
  },
  {
    ...PROVIDER_METADATA.micu,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://www.micuapi.ai",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
    endpointCandidates: ["https://www.micuapi.ai"],
  },
  {
    ...PROVIDER_METADATA.cTok,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.ctok.ai",
        ANTHROPIC_AUTH_TOKEN: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.eFlowCode,
    settingsConfig: {
      effortLevel: "high",
      env: {
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_BASE_URL: "https://e-flowcode.cc",
      },
      enabledPlugins: {
        "superpowers-zh@superpowers-zh": true,
      },
      includeCoAuthoredBy: false,
      ENABLE_TOOL_SEARCH: true,
      skipWebFetchPreflight: true,
    },
    endpointCandidates: ["https://e-flowcode.cc"],
  },
  {
    ...PROVIDER_METADATA.openRouter,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://openrouter.ai/api",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "anthropic/claude-sonnet-4.6",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "anthropic/claude-haiku-4.5",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "anthropic/claude-sonnet-4.6",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "anthropic/claude-opus-4.7",
      },
    },
  },
  {
    ...PROVIDER_METADATA.theRouter,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.therouter.ai",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_API_KEY: "",
        ANTHROPIC_MODEL: "anthropic/claude-sonnet-4.6",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "anthropic/claude-haiku-4.5",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "anthropic/claude-sonnet-4.6",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "anthropic/claude-opus-4.7",
      },
    },
    endpointCandidates: ["https://api.therouter.ai"],
  },
  {
    ...PROVIDER_METADATA.novitaAi,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.novita.ai/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "zai-org/glm-5",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "zai-org/glm-5",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "zai-org/glm-5",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "zai-org/glm-5",
      },
    },
    endpointCandidates: ["https://api.novita.ai/anthropic"],
  },
  {
    ...PROVIDER_METADATA.githubCopilot,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.githubcopilot.com",
        ANTHROPIC_MODEL: "claude-sonnet-4.6",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "claude-haiku-4.5",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "claude-sonnet-4.6",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "claude-sonnet-4.6",
      },
    },
    apiFormat: "openai_chat",
    providerType: "github_copilot",
    requiresOAuth: true,
  },
  {
    ...PROVIDER_METADATA.codex,
    settingsConfig: {
      env: {
        // base_url 由代理后端强制重写为 chatgpt.com/backend-api/codex
        // 用户无需配置
        ANTHROPIC_BASE_URL: "https://chatgpt.com/backend-api/codex",
        ANTHROPIC_MODEL: "gpt-5.4",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "gpt-5.4-mini",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "gpt-5.4",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "gpt-5.4",
      },
    },
    apiFormat: "openai_responses",
    providerType: "codex_oauth",
    requiresOAuth: true,
  },
  {
    ...PROVIDER_METADATA.lemonData,
    apiKeyField: "ANTHROPIC_API_KEY",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.lemondata.cc",
        ANTHROPIC_API_KEY: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.nvidia,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://integrate.api.nvidia.com",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "moonshotai/kimi-k2.5",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "moonshotai/kimi-k2.5",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "moonshotai/kimi-k2.5",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "moonshotai/kimi-k2.5",
      },
    },
    apiFormat: "openai_chat",
  },
  {
    ...PROVIDER_METADATA.pipellm,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://cc-api.pipellm.ai",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "claude-opus-4-7",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "claude-haiku-4-5-20251001",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "claude-sonnet-4-6",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "claude-opus-4-7",
      },
      includeCoAuthoredBy: false,
    },
  },
  {
    ...PROVIDER_METADATA.xiaomiMiMo,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.xiaomimimo.com/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "mimo-v2.5-pro",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "mimo-v2.5-pro",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "mimo-v2.5-pro",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "mimo-v2.5-pro",
      },
    },
  },
  {
    ...PROVIDER_METADATA.xiaomiMiMoTokenPlan,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://token-plan-cn.xiaomimimo.com/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "mimo-v2.5-pro",
        ANTHROPIC_DEFAULT_HAIKU_MODEL: "mimo-v2.5-pro",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "mimo-v2.5-pro",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "mimo-v2.5-pro",
      },
    },
  },
  {
    ...PROVIDER_METADATA.awsBedrockAksk,
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://bedrock-runtime.${AWS_REGION}.amazonaws.com",
        AWS_ACCESS_KEY_ID: "${AWS_ACCESS_KEY_ID}",
        AWS_SECRET_ACCESS_KEY: "${AWS_SECRET_ACCESS_KEY}",
        AWS_REGION: "${AWS_REGION}",
        ANTHROPIC_MODEL: "global.anthropic.claude-opus-4-7",
        ANTHROPIC_DEFAULT_HAIKU_MODEL:
          "global.anthropic.claude-haiku-4-5-20251001-v1:0",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "global.anthropic.claude-sonnet-4-6",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "global.anthropic.claude-opus-4-7",
        CLAUDE_CODE_USE_BEDROCK: "1",
      },
    },
    templateValues: {
      AWS_REGION: {
        label: "AWS Region",
        placeholder: "us-west-2",
        editorValue: "us-west-2",
      },
      AWS_ACCESS_KEY_ID: {
        label: "Access Key ID",
        placeholder: "AKIA...",
        editorValue: "",
      },
      AWS_SECRET_ACCESS_KEY: {
        label: "Secret Access Key",
        placeholder: "your-secret-key",
        editorValue: "",
      },
    },
  },
  {
    ...PROVIDER_METADATA.awsBedrockApiKey,
    settingsConfig: {
      apiKey: "",
      env: {
        ANTHROPIC_BASE_URL:
          "https://bedrock-runtime.${AWS_REGION}.amazonaws.com",
        AWS_REGION: "${AWS_REGION}",
        ANTHROPIC_MODEL: "global.anthropic.claude-opus-4-7",
        ANTHROPIC_DEFAULT_HAIKU_MODEL:
          "global.anthropic.claude-haiku-4-5-20251001-v1:0",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "global.anthropic.claude-sonnet-4-6",
        ANTHROPIC_DEFAULT_OPUS_MODEL: "global.anthropic.claude-opus-4-7",
        CLAUDE_CODE_USE_BEDROCK: "1",
      },
    },
    templateValues: {
      AWS_REGION: {
        label: "AWS Region",
        placeholder: "us-west-2",
        editorValue: "us-west-2",
      },
    },
  },
];
