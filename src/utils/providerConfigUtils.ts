// 供应商配置处理工具函数 — re-export 兼容层
// 所有函数已拆分至职责单一的子模块，此文件仅做 re-export 以保持向后兼容

export {
  isPlainObject,
  deepMerge,
  deepRemove,
  isSubset,
  validateJsonConfig,
  updateCommonConfigSnippet,
  hasCommonConfigSnippet,
} from "@/utils/jsonConfigUtils";

export type { UpdateCommonConfigResult } from "@/utils/jsonConfigUtils";

export {
  updateTomlCommonConfigSnippet,
  hasTomlCommonConfigSnippet,
} from "@/utils/tomlConfigUtils";

export type { UpdateTomlCommonConfigResult } from "@/utils/tomlConfigUtils";

export {
  getApiKeyFromConfig,
  hasApiKeyField,
  setApiKeyInConfig,
} from "@/utils/apiKeyUtils";

export { applyTemplateValues } from "@/utils/templateUtils";

export {
  isCodexChatWireApi,
  extractCodexWireApi,
  setCodexWireApi,
  extractCodexBaseUrl,
  extractCodexExperimentalBearerToken,
  updateCodexExperimentalBearerToken,
  getCodexBaseUrl,
  setCodexBaseUrl,
  extractCodexModelName,
  setCodexModelName,
} from "@/utils/codexConfigUtils";

export {
  isCodexGoalModeEnabled,
  setCodexGoalMode,
  extractCodexTopLevelInt,
  setCodexTopLevelInt,
  removeCodexTopLevelField,
} from "@/utils/codexFieldUtils";
