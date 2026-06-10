/**
 * Claude Desktop 预设供应商配置模板
 *
 * 形态与 Claude Code 预设不同：
 * - baseUrl 是顶级字段，而不是 settingsConfig.env.ANTHROPIC_BASE_URL
 * - 模型信息以"Desktop 可见模型 ID → 上游模型"表达，
 *   对应后端 ClaudeDesktopModelRoute 的 routeId / model
 *
 * 翻译来源：src/config/claudeProviderPresets.ts（排除 OAuth 与不兼容预设）
 */
import type { BaseProviderPreset } from "./baseProviderPreset";
import { PROVIDER_METADATA } from "./providerMetadata";

export type ClaudeDesktopApiFormat =
  | "anthropic"
  | "openai_chat"
  | "openai_responses"
  | "gemini_native";

export interface ClaudeDesktopRoutePreset {
  routeId: string;
  upstreamModel: string;
  labelOverride?: string;
  supports1m: boolean;
}

/**
 * Claude Desktop 3P fail-all 校验只接受 `claude-(sonnet|opus|haiku)-*` 形式的
 * routeId（1.6259.1+，实测 2026-05-13）。所有预设工厂、表单角色下拉、
 * 后端 `next_catalog_safe_route_id` 都从此映射派生 routeId，避免散落硬编码。
 */
export const CLAUDE_DESKTOP_ROLE_ROUTE_IDS = {
  sonnet: "claude-sonnet-4-6",
  opus: "claude-opus-4-7",
  haiku: "claude-haiku-4-5",
} as const;

export type ClaudeDesktopRoleId = keyof typeof CLAUDE_DESKTOP_ROLE_ROUTE_IDS;

export interface ClaudeDesktopProviderPreset extends BaseProviderPreset {
  baseUrl: string;
  apiKeyField?: "ANTHROPIC_AUTH_TOKEN" | "ANTHROPIC_API_KEY";

  mode: "direct" | "proxy";
  apiFormat?: ClaudeDesktopApiFormat;
  modelRoutes?: ClaudeDesktopRoutePreset[];
  providerType?: "github_copilot" | "codex_oauth";
  requiresOAuth?: boolean;

  endpointCandidates?: string[];
}

const passthroughRoutes = (supports1m = false): ClaudeDesktopRoutePreset[] => [
  {
    routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.sonnet,
    upstreamModel: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.sonnet,
    supports1m,
  },
  {
    routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.opus,
    upstreamModel: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.opus,
    supports1m,
  },
  {
    routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.haiku,
    upstreamModel: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.haiku,
    supports1m,
  },
];

const mappedRoutes = (
  sonnet: string,
  opus: string,
  haiku: string,
  supports1m = false,
): ClaudeDesktopRoutePreset[] => [
  {
    routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.sonnet,
    upstreamModel: sonnet,
    supports1m,
  },
  {
    routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.opus,
    upstreamModel: opus,
    supports1m,
  },
  {
    routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.haiku,
    upstreamModel: haiku,
    supports1m,
  },
];

/**
 * 非 Claude 上游模型用此工厂：route ID 使用 Claude Desktop 能通过校验的
 * Sonnet/Opus/Haiku 路由，真实品牌名只写入 labelOverride 和 upstreamModel。
 */
const brandedRoutes = (
  sonnet: string,
  opus: string,
  haiku: string,
  supports1m = false,
): ClaudeDesktopRoutePreset[] => {
  const seenUpstream = new Set<string>();
  return [
    { routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.sonnet, upstreamModel: sonnet },
    { routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.opus, upstreamModel: opus },
    { routeId: CLAUDE_DESKTOP_ROLE_ROUTE_IDS.haiku, upstreamModel: haiku },
  ]
    .map(({ routeId, upstreamModel }) => ({
      routeId,
      upstreamModel,
      labelOverride: upstreamModel,
      supports1m,
    }))
    .filter((route) => {
      if (seenUpstream.has(route.upstreamModel)) {
        return false;
      }
      seenUpstream.add(route.upstreamModel);
      return true;
    });
};

export const claudeDesktopProviderPresets: ClaudeDesktopProviderPreset[] = [
  {
    ...PROVIDER_METADATA.claudeDesktopOfficial,
    baseUrl: "",
    mode: "direct",
    apiFormat: "anthropic",
    theme: {
      icon: "claude",
      backgroundColor: "#D97757",
      textColor: "#FFFFFF",
    },
  },
  {
    ...PROVIDER_METADATA.shengsuanyun,
    baseUrl: "https://router.shengsuanyun.com/api",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
  },
  {
    ...PROVIDER_METADATA.patewayAi,
    baseUrl: "https://api.pateway.ai",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
  },
  {
    ...PROVIDER_METADATA.volcengineAgentplan,
    baseUrl: "https://ark.cn-beijing.volces.com/api/coding",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "ark-code-latest",
      "ark-code-latest",
      "ark-code-latest",
    ),
  },
  {
    ...PROVIDER_METADATA.bytePlus,
    baseUrl: "https://ark.ap-southeast.bytepluses.com/api/coding",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "ark-code-latest",
      "ark-code-latest",
      "ark-code-latest",
    ),
  },
  {
    ...PROVIDER_METADATA.douBaoSeed,
    baseUrl: "https://ark.cn-beijing.volces.com/api/compatible",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "doubao-seed-2-0-code-preview-latest",
      "doubao-seed-2-0-code-preview-latest",
      "doubao-seed-2-0-code-preview-latest",
    ),
  },
  {
    ...PROVIDER_METADATA.geminiNative,
    baseUrl: "https://generativelanguage.googleapis.com",
    apiKeyField: "ANTHROPIC_API_KEY",
    mode: "proxy",
    apiFormat: "gemini_native",
    modelRoutes: brandedRoutes(
      "gemini-3.1-pro",
      "gemini-3.1-pro",
      "gemini-3-flash",
    ),
    endpointCandidates: ["https://generativelanguage.googleapis.com"],
  },
  {
    ...PROVIDER_METADATA.githubCopilot,
    baseUrl: "https://api.githubcopilot.com",
    mode: "proxy",
    apiFormat: "openai_chat",
    providerType: "github_copilot",
    requiresOAuth: true,
    modelRoutes: brandedRoutes(
      "claude-sonnet-4.6",
      "claude-sonnet-4.6",
      "claude-haiku-4.5",
    ),
  },
  {
    ...PROVIDER_METADATA.codex,
    baseUrl: "https://chatgpt.com/backend-api/codex",
    mode: "proxy",
    apiFormat: "openai_responses",
    providerType: "codex_oauth",
    requiresOAuth: true,
    modelRoutes: brandedRoutes("gpt-5.4", "gpt-5.4", "gpt-5.4-mini"),
  },
  {
    ...PROVIDER_METADATA.deepSeek,
    baseUrl: "https://api.deepseek.com/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "deepseek-v4-pro",
      "deepseek-v4-pro",
      "deepseek-v4-flash",
    ),
  },
  {
    ...PROVIDER_METADATA.openCodeGoDeepSeek,
    baseUrl: "https://opencode.ai/zen/go",
    mode: "proxy",
    apiFormat: "openai_chat",
    modelRoutes: brandedRoutes(
      "deepseek-v4-flash",
      "deepseek-v4-flash",
      "deepseek-v4-flash",
    ),
    endpointCandidates: ["https://opencode.ai/zen/go"],
  },
  {
    ...PROVIDER_METADATA.zhipuGlm,
    baseUrl: "https://open.bigmodel.cn/api/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("glm-5", "glm-5", "glm-5"),
  },
  {
    ...PROVIDER_METADATA.zhipuGlmEn,
    baseUrl: "https://api.z.ai/api/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("glm-5", "glm-5", "glm-5"),
  },
  {
    ...PROVIDER_METADATA.baiduQianfan,
    baseUrl: "https://qianfan.baidubce.com/anthropic/coding",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "qianfan-code-latest",
      "qianfan-code-latest",
      "qianfan-code-latest",
    ),
    endpointCandidates: ["https://qianfan.baidubce.com/anthropic/coding"],
  },
  {
    ...PROVIDER_METADATA.bailian,
    baseUrl: "https://dashscope.aliyuncs.com/apps/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
  },
  {
    ...PROVIDER_METADATA.bailianForCoding,
    baseUrl: "https://coding.dashscope.aliyuncs.com/apps/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
  },
  {
    ...PROVIDER_METADATA.kimi,
    baseUrl: "https://api.moonshot.cn/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("kimi-k2.6", "kimi-k2.6", "kimi-k2.6"),
  },
  {
    ...PROVIDER_METADATA.kimiForCoding,
    baseUrl: "https://api.kimi.com/coding/",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
  },
  {
    ...PROVIDER_METADATA.stepFun,
    baseUrl: "https://api.stepfun.com/step_plan",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "step-3.5-flash-2603",
      "step-3.5-flash-2603",
      "step-3.5-flash-2603",
    ),
    endpointCandidates: ["https://api.stepfun.com/step_plan"],
  },
  {
    ...PROVIDER_METADATA.stepFunEn,
    baseUrl: "https://api.stepfun.ai/step_plan",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "step-3.5-flash-2603",
      "step-3.5-flash-2603",
      "step-3.5-flash-2603",
    ),
    endpointCandidates: ["https://api.stepfun.ai/step_plan"],
  },
  {
    ...PROVIDER_METADATA.modelScope,
    baseUrl: "https://api-inference.modelscope.cn",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "ZhipuAI/GLM-5",
      "ZhipuAI/GLM-5",
      "ZhipuAI/GLM-5",
    ),
  },
  {
    ...PROVIDER_METADATA.longcat,
    baseUrl: "https://api.longcat.chat/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "LongCat-Flash-Chat",
      "LongCat-Flash-Chat",
      "LongCat-Flash-Chat",
    ),
  },
  {
    ...PROVIDER_METADATA.miniMax,
    baseUrl: "https://api.minimaxi.com/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("MiniMax-M2.7", "MiniMax-M2.7", "MiniMax-M2.7"),
    theme: {
      backgroundColor: "#f64551",
      textColor: "#FFFFFF",
    },
  },
  {
    ...PROVIDER_METADATA.miniMaxEn,
    baseUrl: "https://api.minimax.io/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("MiniMax-M2.7", "MiniMax-M2.7", "MiniMax-M2.7"),
    theme: {
      backgroundColor: "#f64551",
      textColor: "#FFFFFF",
    },
  },
  {
    ...PROVIDER_METADATA.baiLing,
    baseUrl: "https://api.tbox.cn/api/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes("Ling-2.5-1T", "Ling-2.5-1T", "Ling-2.5-1T"),
  },
  {
    ...PROVIDER_METADATA.aiHubMix,
    baseUrl: "https://aihubmix.com",
    apiKeyField: "ANTHROPIC_API_KEY",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: ["https://aihubmix.com", "https://api.aihubmix.com"],
  },
  {
    ...PROVIDER_METADATA.siliconFlow,
    baseUrl: "https://api.siliconflow.cn",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "Pro/MiniMaxAI/MiniMax-M2.7",
      "Pro/MiniMaxAI/MiniMax-M2.7",
      "Pro/MiniMaxAI/MiniMax-M2.7",
    ),
  },
  {
    ...PROVIDER_METADATA.siliconFlowEn,
    baseUrl: "https://api.siliconflow.com",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "MiniMaxAI/MiniMax-M2.7",
      "MiniMaxAI/MiniMax-M2.7",
      "MiniMaxAI/MiniMax-M2.7",
    ),
  },
  {
    ...PROVIDER_METADATA.dmxapi,
    baseUrl: "https://www.dmxapi.cn",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: ["https://www.dmxapi.cn", "https://api.dmxapi.cn"],
  },
  {
    ...PROVIDER_METADATA.packyCode,
    baseUrl: "https://www.packyapi.com",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: [
      "https://www.packyapi.com",
      "https://api-slb.packyapi.com",
    ],
  },
  {
    ...PROVIDER_METADATA.claudeapi,
    baseUrl: "https://gw.claudeapi.com",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
  },
  {
    ...PROVIDER_METADATA.claudecn,
    baseUrl: "https://claudecn.top",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
  },
  {
    ...PROVIDER_METADATA.runapi,
    baseUrl: "https://runapi.co",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
  },
  {
    ...PROVIDER_METADATA.relaxyCode,
    baseUrl: "https://www.relaxycode.com",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
  },
  {
    ...PROVIDER_METADATA.cubence,
    baseUrl: "https://api.cubence.com",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: [
      "https://api.cubence.com",
      "https://api-cf.cubence.com",
      "https://api-dmit.cubence.com",
      "https://api-bwg.cubence.com",
    ],
  },
  {
    ...PROVIDER_METADATA.aiGoCode,
    baseUrl: "https://api.aigocode.com",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: ["https://api.aigocode.com"],
  },
  {
    ...PROVIDER_METADATA.rightCode,
    baseUrl: "https://www.right.codes/claude",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
  },
  {
    ...PROVIDER_METADATA.aiCodeMirror,
    baseUrl: "https://api.aicodemirror.com/api/claudecode",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: [
      "https://api.aicodemirror.com/api/claudecode",
      "https://api.claudecode.net.cn/api/claudecode",
    ],
  },
  {
    ...PROVIDER_METADATA.crazyRouter,
    baseUrl: "https://cn.crazyrouter.com",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: ["https://cn.crazyrouter.com"],
  },
  {
    ...PROVIDER_METADATA.sssAiCode,
    baseUrl: "https://node-hk.ssssicode.com/api",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: [
      "https://node-hk.ssssicode.com/api",
      "https://claude2.ssssicode.com/api",
      "https://anti.ssssicode.com/api",
    ],
  },
  {
    ...PROVIDER_METADATA.compshare,
    baseUrl: "https://api.modelverse.cn",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: ["https://api.modelverse.cn"],
  },
  {
    ...PROVIDER_METADATA.compshareCodingPlan,
    baseUrl: "https://cp.compshare.cn",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: ["https://cp.compshare.cn"],
  },
  {
    ...PROVIDER_METADATA.micu,
    baseUrl: "https://www.micuapi.ai",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: ["https://www.micuapi.ai"],
  },
  {
    ...PROVIDER_METADATA.cTok,
    baseUrl: "https://api.ctok.ai",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
  },
  {
    ...PROVIDER_METADATA.eFlowCode,
    baseUrl: "https://e-flowcode.cc",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
    endpointCandidates: ["https://e-flowcode.cc"],
  },
  {
    ...PROVIDER_METADATA.openRouter,
    baseUrl: "https://openrouter.ai/api",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: mappedRoutes(
      "anthropic/claude-sonnet-4.6",
      "anthropic/claude-opus-4.7",
      "anthropic/claude-haiku-4.5",
      true,
    ),
  },
  {
    ...PROVIDER_METADATA.theRouter,
    baseUrl: "https://api.therouter.ai",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: mappedRoutes(
      "anthropic/claude-sonnet-4.6",
      "anthropic/claude-opus-4.7",
      "anthropic/claude-haiku-4.5",
      true,
    ),
    endpointCandidates: ["https://api.therouter.ai"],
  },
  {
    ...PROVIDER_METADATA.novitaAi,
    baseUrl: "https://api.novita.ai/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "zai-org/glm-5",
      "zai-org/glm-5",
      "zai-org/glm-5",
    ),
    endpointCandidates: ["https://api.novita.ai/anthropic"],
  },
  {
    ...PROVIDER_METADATA.lemonData,
    baseUrl: "https://api.lemondata.cc",
    apiKeyField: "ANTHROPIC_API_KEY",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
  },
  {
    ...PROVIDER_METADATA.nvidia,
    baseUrl: "https://integrate.api.nvidia.com",
    mode: "proxy",
    apiFormat: "openai_chat",
    modelRoutes: brandedRoutes(
      "moonshotai/kimi-k2.5",
      "moonshotai/kimi-k2.5",
      "moonshotai/kimi-k2.5",
    ),
  },
  {
    ...PROVIDER_METADATA.pipellm,
    baseUrl: "https://cc-api.pipellm.ai",
    mode: "direct",
    apiFormat: "anthropic",
    modelRoutes: passthroughRoutes(),
  },
  {
    ...PROVIDER_METADATA.xiaomiMiMo,
    baseUrl: "https://api.xiaomimimo.com/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "mimo-v2.5-pro",
      "mimo-v2.5-pro",
      "mimo-v2.5-pro",
    ),
  },
  {
    ...PROVIDER_METADATA.xiaomiMiMoTokenPlan,
    baseUrl: "https://token-plan-cn.xiaomimimo.com/anthropic",
    mode: "proxy",
    apiFormat: "anthropic",
    modelRoutes: brandedRoutes(
      "mimo-v2.5-pro",
      "mimo-v2.5-pro",
      "mimo-v2.5-pro",
    ),
  },
];
