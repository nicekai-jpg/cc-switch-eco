/**
 * Codex 预设供应商配置模板
 */
import type {
  BaseProviderPreset,
} from "./baseProviderPreset";
import type {
  CodexApiFormat,
  CodexCatalogModel,
  CodexChatReasoning,
} from "../types";
import { PROVIDER_METADATA } from "./providerMetadata";

export interface CodexProviderPreset extends BaseProviderPreset {
  auth: Record<string, any>; // 将写入 ~/.codex/auth.json
  config: string; // 将写入 ~/.codex/config.toml（TOML 字符串）
  isCustomTemplate?: boolean; // 标识是否为自定义模板
  // 请求地址候选列表（用于地址管理/测速）
  endpointCandidates?: string[];
  // Codex API 格式
  apiFormat?: CodexApiFormat;
  // Codex Chat 本地路由模式下的模型目录
  modelCatalog?: CodexCatalogModel[];
  // Codex Responses -> Chat Completions reasoning capability defaults
  codexChatReasoning?: CodexChatReasoning;
}

/**
 * 生成第三方供应商的 auth.json
 */
export function generateThirdPartyAuth(apiKey: string): Record<string, any> {
  return {
    OPENAI_API_KEY: apiKey || "",
  };
}

/**
 * 生成第三方供应商的 config.toml
 */
export function generateThirdPartyConfig(
  providerName: string,
  baseUrl: string,
  modelName = "gpt-5.4",
): string {
  const tomlString = (value: string) => JSON.stringify(value);

  return `model_provider = "custom"
model = ${tomlString(modelName)}
model_reasoning_effort = "high"
disable_response_storage = true

[model_providers.custom]
name = ${tomlString(providerName)}
base_url = ${tomlString(baseUrl)}
wire_api = "responses"
requires_openai_auth = true`;
}

function modelCatalog(
  models: Array<
    string | { model: string; displayName?: string; contextWindow?: number }
  >,
): CodexCatalogModel[] {
  return models.map((entry) =>
    typeof entry === "string"
      ? { model: entry }
      : {
          model: entry.model,
          displayName: entry.displayName,
          contextWindow: entry.contextWindow,
        },
  );
}

export const codexProviderPresets: CodexProviderPreset[] = [
  {
    ...PROVIDER_METADATA.openaiOfficial,
    isOfficial: true,
    auth: {},
    config: ``,
    theme: {
      icon: "codex",
      backgroundColor: "#1F2937", // gray-800
      textColor: "#FFFFFF",
    },
  },
  {
    ...PROVIDER_METADATA.shengsuanyun,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "shengsuanyun",
      "https://router.shengsuanyun.com/api/v1",
      "gpt-5.4",
    ),
  },
  {
    ...PROVIDER_METADATA.patewayAi,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "patewayai",
      "https://api.pateway.ai/v1",
      "gpt-5.5",
    ),
    endpointCandidates: ["https://api.pateway.ai/v1"],
  },
  {
    ...PROVIDER_METADATA.volcengineAgentplan,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "ark_agentplan",
      "https://ark.cn-beijing.volces.com/api/coding/v3",
      "ark-code-latest",
    ),
    endpointCandidates: ["https://ark.cn-beijing.volces.com/api/coding/v3"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "ark-code-latest",
        displayName: "Ark Code Latest",
        contextWindow: 256000,
      },
    ]),
  },
  {
    ...PROVIDER_METADATA.bytePlus,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "byteplus",
      "https://ark.ap-southeast.bytepluses.com/api/coding/v3",
      "ark-code-latest",
    ),
    endpointCandidates: [
      "https://ark.ap-southeast.bytepluses.com/api/coding/v3",
    ],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "ark-code-latest",
        displayName: "Ark Code Latest",
        contextWindow: 256000,
      },
    ]),
  },
  {
    ...PROVIDER_METADATA.douBaoSeed,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "doubaoseed",
      "https://ark.cn-beijing.volces.com/api/v3",
      "doubao-seed-2-0-code-preview-latest",
    ),
    endpointCandidates: ["https://ark.cn-beijing.volces.com/api/v3"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "doubao-seed-2-0-code-preview-latest",
        displayName: "Doubao Seed Code Preview",
        contextWindow: 256000,
      },
    ]),
  },
  {
    ...PROVIDER_METADATA.azureOpenai,
    isOfficial: true,
    auth: generateThirdPartyAuth(""),
    config: `model_provider = "custom"
model = "gpt-5.4"
model_reasoning_effort = "high"
disable_response_storage = true

[model_providers.custom]
name = "Azure OpenAI"
base_url = "https://YOUR_RESOURCE_NAME.openai.azure.com/openai"
env_key = "OPENAI_API_KEY"
query_params = { "api-version" = "2025-04-01-preview" }
wire_api = "responses"
requires_openai_auth = true`,
    endpointCandidates: ["https://YOUR_RESOURCE_NAME.openai.azure.com/openai"],
    theme: {
      icon: "codex",
      backgroundColor: "#0078D4",
      textColor: "#FFFFFF",
    },
  },
  {
    ...PROVIDER_METADATA.deepSeek,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "deepseek",
      "https://api.deepseek.com",
      "deepseek-v4-flash",
    ),
    endpointCandidates: ["https://api.deepseek.com"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "deepseek-v4-flash",
        displayName: "DeepSeek V4 Flash",
        contextWindow: 1000000,
      },
      {
        model: "deepseek-v4-pro",
        displayName: "DeepSeek V4 Pro",
        contextWindow: 1000000,
      },
    ]),
    codexChatReasoning: {
      supportsThinking: true,
      supportsEffort: true,
      thinkingParam: "thinking",
      effortParam: "reasoning_effort",
      effortValueMode: "deepseek",
      outputFormat: "reasoning_content",
    },
  },
  {
    ...PROVIDER_METADATA.zhipuGlm,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "zhipu_glm",
      "https://open.bigmodel.cn/api/paas/v4",
      "glm-5",
    ),
    endpointCandidates: ["https://open.bigmodel.cn/api/paas/v4"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      { model: "glm-5", displayName: "GLM-5", contextWindow: 200000 },
    ]),
    codexChatReasoning: {
      supportsThinking: true,
      supportsEffort: false,
      thinkingParam: "thinking",
      effortParam: "none",
      outputFormat: "reasoning_content",
    },
  },
  {
    ...PROVIDER_METADATA.zhipuGlmEn,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "zhipu_glm_en",
      "https://api.z.ai/api/paas/v4",
      "glm-5",
    ),
    endpointCandidates: ["https://api.z.ai/api/paas/v4"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      { model: "glm-5", displayName: "GLM-5", contextWindow: 200000 },
    ]),
    codexChatReasoning: {
      supportsThinking: true,
      supportsEffort: false,
      thinkingParam: "thinking",
      effortParam: "none",
      outputFormat: "reasoning_content",
    },
  },
  {
    ...PROVIDER_METADATA.baiduQianfan,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "qianfan_coding",
      "https://qianfan.baidubce.com/v2/coding",
      "qianfan-code-latest",
    ),
    endpointCandidates: ["https://qianfan.baidubce.com/v2/coding"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "qianfan-code-latest",
        displayName: "Qianfan Code Latest",
        contextWindow: 131072,
      },
    ]),
  },
  {
    ...PROVIDER_METADATA.bailian,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "bailian",
      "https://dashscope.aliyuncs.com/compatible-mode/v1",
      "qwen3-coder-plus",
    ),
    endpointCandidates: ["https://dashscope.aliyuncs.com/compatible-mode/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "qwen3-coder-plus",
        displayName: "Qwen3 Coder Plus",
        contextWindow: 1000000,
      },
      { model: "qwen3-max", displayName: "Qwen3 Max", contextWindow: 262144 },
    ]),
    codexChatReasoning: {
      supportsThinking: true,
      supportsEffort: false,
      thinkingParam: "enable_thinking",
      effortParam: "none",
      outputFormat: "reasoning_content",
    },
  },
  {
    ...PROVIDER_METADATA.kimi,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "kimi",
      "https://api.moonshot.cn/v1",
      "kimi-k2.6",
    ),
    endpointCandidates: ["https://api.moonshot.cn/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      { model: "kimi-k2.6", displayName: "Kimi K2.6", contextWindow: 262144 },
    ]),
    codexChatReasoning: {
      supportsThinking: true,
      supportsEffort: false,
      thinkingParam: "thinking",
      effortParam: "none",
      outputFormat: "reasoning_content",
    },
  },
  {
    ...PROVIDER_METADATA.stepFun,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "stepfun",
      "https://api.stepfun.com/step_plan/v1",
      "step-3.5-flash-2603",
    ),
    endpointCandidates: ["https://api.stepfun.com/step_plan/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "step-3.5-flash-2603",
        displayName: "Step 3.5 Flash 2603",
        contextWindow: 262144,
      },
      {
        model: "step-3.5-flash",
        displayName: "Step 3.5 Flash",
        contextWindow: 262144,
      },
    ]),
  },
  {
    ...PROVIDER_METADATA.stepFunEn,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "stepfun_en",
      "https://api.stepfun.ai/step_plan/v1",
      "step-3.5-flash-2603",
    ),
    endpointCandidates: ["https://api.stepfun.ai/step_plan/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "step-3.5-flash-2603",
        displayName: "Step 3.5 Flash 2603",
        contextWindow: 262144,
      },
      {
        model: "step-3.5-flash",
        displayName: "Step 3.5 Flash",
        contextWindow: 262144,
      },
    ]),
  },
  {
    ...PROVIDER_METADATA.modelScope,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "modelscope",
      "https://api-inference.modelscope.cn/v1",
      "ZhipuAI/GLM-5",
    ),
    endpointCandidates: ["https://api-inference.modelscope.cn/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "ZhipuAI/GLM-5",
        displayName: "ZhipuAI / GLM-5",
        contextWindow: 200000,
      },
    ]),
    codexChatReasoning: {
      supportsThinking: true,
      supportsEffort: false,
      thinkingParam: "thinking",
      effortParam: "none",
      outputFormat: "reasoning_content",
    },
  },
  {
    ...PROVIDER_METADATA.longcat,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "longcat",
      "https://api.longcat.chat/openai/v1",
      "LongCat-Flash-Chat",
    ),
    endpointCandidates: ["https://api.longcat.chat/openai/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "LongCat-Flash-Chat",
        displayName: "LongCat Flash Chat",
        contextWindow: 262144,
      },
    ]),
  },
  {
    ...PROVIDER_METADATA.miniMax,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "minimax",
      "https://api.minimaxi.com/v1",
      "MiniMax-M2.7",
    ),
    endpointCandidates: ["https://api.minimaxi.com/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "MiniMax-M2.7",
        displayName: "MiniMax M2.7",
        contextWindow: 200000,
      },
    ]),
    codexChatReasoning: {
      supportsThinking: true,
      supportsEffort: false,
      thinkingParam: "reasoning_split",
      effortParam: "none",
      outputFormat: "reasoning_details",
    },
    theme: {
      backgroundColor: "#f64551",
      textColor: "#FFFFFF",
    },
  },
  {
    ...PROVIDER_METADATA.miniMaxEn,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "minimax_en",
      "https://api.minimax.io/v1",
      "MiniMax-M2.7",
    ),
    endpointCandidates: ["https://api.minimax.io/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "MiniMax-M2.7",
        displayName: "MiniMax M2.7",
        contextWindow: 200000,
      },
    ]),
    codexChatReasoning: {
      supportsThinking: true,
      supportsEffort: false,
      thinkingParam: "reasoning_split",
      effortParam: "none",
      outputFormat: "reasoning_details",
    },
    theme: {
      backgroundColor: "#f64551",
      textColor: "#FFFFFF",
    },
  },
  {
    ...PROVIDER_METADATA.baiLing,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "bailing",
      "https://api.tbox.cn/api/llm/v1",
      "Ling-2.5-1T",
    ),
    endpointCandidates: ["https://api.tbox.cn/api/llm/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "Ling-2.5-1T",
        displayName: "Ling-2.5-1T",
        contextWindow: 131072,
      },
    ]),
  },
  {
    ...PROVIDER_METADATA.xiaomiMiMo,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "xiaomi_mimo",
      "https://api.xiaomimimo.com/v1",
      "mimo-v2.5-pro",
    ),
    endpointCandidates: ["https://api.xiaomimimo.com/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "mimo-v2.5-pro",
        displayName: "MiMo V2.5 Pro",
        contextWindow: 1048576,
      },
    ]),
    codexChatReasoning: {
      supportsThinking: true,
      supportsEffort: false,
      thinkingParam: "thinking",
      effortParam: "none",
      outputFormat: "reasoning_content",
    },
  },
  {
    ...PROVIDER_METADATA.xiaomiMiMoTokenPlan,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "xiaomi_mimo_token_plan",
      "https://token-plan-cn.xiaomimimo.com/v1",
      "mimo-v2.5-pro",
    ),
    endpointCandidates: ["https://token-plan-cn.xiaomimimo.com/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "mimo-v2.5-pro",
        displayName: "MiMo V2.5 Pro",
        contextWindow: 1048576,
      },
    ]),
    codexChatReasoning: {
      supportsThinking: true,
      supportsEffort: false,
      thinkingParam: "thinking",
      effortParam: "none",
      outputFormat: "reasoning_content",
    },
  },
  {
    ...PROVIDER_METADATA.siliconFlow,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "siliconflow",
      "https://api.siliconflow.cn/v1",
      "Pro/MiniMaxAI/MiniMax-M2.7",
    ),
    endpointCandidates: ["https://api.siliconflow.cn/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "Pro/MiniMaxAI/MiniMax-M2.7",
        displayName: "Pro / MiniMax M2.7",
        contextWindow: 200000,
      },
    ]),
  },
  {
    ...PROVIDER_METADATA.siliconFlowEn,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "siliconflow_en",
      "https://api.siliconflow.com/v1",
      "MiniMaxAI/MiniMax-M2.7",
    ),
    endpointCandidates: ["https://api.siliconflow.com/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "MiniMaxAI/MiniMax-M2.7",
        displayName: "MiniMax M2.7",
        contextWindow: 200000,
      },
    ]),
  },
  {
    ...PROVIDER_METADATA.novitaAi,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "novita",
      "https://api.novita.ai/openai/v1",
      "zai-org/glm-5",
    ),
    endpointCandidates: ["https://api.novita.ai/openai/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      { model: "zai-org/glm-5", displayName: "GLM-5", contextWindow: 202800 },
    ]),
    codexChatReasoning: {
      supportsThinking: true,
      supportsEffort: false,
      thinkingParam: "thinking",
      effortParam: "none",
      outputFormat: "reasoning_content",
    },
  },
  {
    ...PROVIDER_METADATA.nvidia,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "nvidia",
      "https://integrate.api.nvidia.com/v1",
      "moonshotai/kimi-k2.5",
    ),
    endpointCandidates: ["https://integrate.api.nvidia.com/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "moonshotai/kimi-k2.5",
        displayName: "Kimi K2.5",
        contextWindow: 262144,
      },
    ]),
    codexChatReasoning: {
      supportsThinking: true,
      supportsEffort: false,
      thinkingParam: "thinking",
      effortParam: "none",
      outputFormat: "reasoning_content",
    },
  },
  {
    ...PROVIDER_METADATA.aiHubMix,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "aihubmix",
      "https://aihubmix.com/v1",
      "gpt-5.4",
    ),
    endpointCandidates: [
      "https://aihubmix.com/v1",
      "https://api.aihubmix.com/v1",
    ],
  },
  {
    ...PROVIDER_METADATA.dmxapi,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "dmxapi",
      "https://www.dmxapi.cn/v1",
      "gpt-5.4",
    ),
    endpointCandidates: ["https://www.dmxapi.cn/v1"],
  },
  {
    ...PROVIDER_METADATA.packyCode,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "packycode",
      "https://www.packyapi.com/v1",
      "gpt-5.4",
    ),
    endpointCandidates: [
      "https://www.packyapi.com/v1",
      "https://api-slb.packyapi.com/v1",
    ],
  },
  {
    ...PROVIDER_METADATA.claudecn,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "claudecn",
      "https://claudecn.top/v1",
      "gpt-5.5",
    ),
  },
  {
    ...PROVIDER_METADATA.runapi,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "runapi",
      "https://runapi.co/v1",
      "gpt-5.5",
    ),
  },
  {
    ...PROVIDER_METADATA.relaxyCode,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "relaxycode",
      "https://www.relaxycode.com/v1",
      "gpt-5.5",
    ),
  },
  {
    ...PROVIDER_METADATA.cubence,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "cubence",
      "https://api.cubence.com/v1",
      "gpt-5.4",
    ),
    endpointCandidates: [
      "https://api.cubence.com/v1",
      "https://api-cf.cubence.com/v1",
      "https://api-dmit.cubence.com/v1",
      "https://api-bwg.cubence.com/v1",
    ],
  },
  {
    ...PROVIDER_METADATA.aiGoCode,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "aigocode",
      "https://api.aigocode.com",
      "gpt-5.4",
    ),
    endpointCandidates: ["https://api.aigocode.com"],
  },
  {
    ...PROVIDER_METADATA.rightCode,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "rightcode",
      "https://right.codes/codex/v1",
      "gpt-5.4",
    ),
  },
  {
    ...PROVIDER_METADATA.aiCodeMirror,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "aicodemirror",
      "https://api.aicodemirror.com/api/codex/backend-api/codex",
      "gpt-5.4",
    ),
    endpointCandidates: [
      "https://api.aicodemirror.com/api/codex/backend-api/codex",
      "https://api.claudecode.net.cn/api/codex/backend-api/codex",
    ],
  },
  {
    ...PROVIDER_METADATA.crazyRouter,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "crazyrouter",
      "https://cn.crazyrouter.com/v1",
      "gpt-5.4",
    ),
    endpointCandidates: ["https://cn.crazyrouter.com/v1"],
  },
  {
    ...PROVIDER_METADATA.sssAiCode,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "sssaicode",
      "https://node-hk.sssaicode.com/api/v1",
      "gpt-5.4",
    ),
    endpointCandidates: [
      "https://node-hk.sssaicode.com/api/v1",
      "https://claude2.sssaicode.com/api/v1",
      "https://anti.sssaicode.com/api/v1",
    ],
  },
  {
    ...PROVIDER_METADATA.compshare,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "compshare",
      "https://api.modelverse.cn/v1",
      "gpt-5.4",
    ),
    endpointCandidates: ["https://api.modelverse.cn/v1"],
  },
  {
    ...PROVIDER_METADATA.compshareCodingPlan,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "compshare_coding",
      "https://cp.compshare.cn/v1",
      "gpt-5.4",
    ),
    endpointCandidates: ["https://cp.compshare.cn/v1"],
  },
  {
    ...PROVIDER_METADATA.micu,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "micu",
      "https://www.micuapi.ai/v1",
      "gpt-5.4",
    ),
    endpointCandidates: ["https://www.micuapi.ai/v1"],
  },
  {
    ...PROVIDER_METADATA.cTok,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "ctok",
      "https://api.ctok.ai/v1",
      "gpt-5.4",
    ),
    endpointCandidates: ["https://api.ctok.ai/v1"],
  },
  {
    ...PROVIDER_METADATA.eFlowCode,
    auth: {
      OPENAI_API_KEY: "",
    },
    config: `model_provider = "custom"
model = "gpt-5.4"
model_reasoning_effort = "high"
disable_response_storage = true
personality = "pragmatic"

[model_providers.custom]
name = "E-FlowCode"
base_url = "https://e-flowcode.cc/v1"
wire_api = "responses"
requires_openai_auth = true
model_context_window = 1000000
model_auto_compact_token_limit = 9000000`,
    endpointCandidates: ["https://e-flowcode.cc/v1"],
  },
  {
    ...PROVIDER_METADATA.lemonData,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "lemondata",
      "https://api.lemondata.cc/v1",
      "gpt-5.4",
    ),
    endpointCandidates: ["https://api.lemondata.cc/v1"],
  },
  {
    ...PROVIDER_METADATA.pipellm,
    auth: {
      OPENAI_API_KEY: "",
    },
    config: `model_provider = "custom"
model = "gpt-5.4"
model_reasoning_effort = "medium"
disable_response_storage = true

[model_providers.custom]
name = "PIPELLM"
wire_api = "responses"
requires_openai_auth = true
base_url = "https://cc-api.pipellm.ai/v1"`,
    endpointCandidates: ["https://cc-api.pipellm.ai/v1"],
  },
  {
    ...PROVIDER_METADATA.openRouter,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "openrouter",
      "https://openrouter.ai/api/v1",
      "gpt-5.4",
    ),
  },
  {
    ...PROVIDER_METADATA.theRouter,
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "therouter",
      "https://api.therouter.ai/v1",
      "openai/gpt-5.3-codex",
    ),
    endpointCandidates: ["https://api.therouter.ai/v1"],
  },
];
