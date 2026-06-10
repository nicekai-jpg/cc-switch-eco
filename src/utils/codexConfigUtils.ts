// Codex 专用配置工具函数（wire_api, base_url, bearer_token, model）

import { normalizeTomlText } from "@/utils/textNormalization";
import { parse as parseToml } from "smol-toml";
import {
  TOML_BASE_URL_PATTERN,
  TOML_EXPERIMENTAL_BEARER_TOKEN_PATTERN,
  TOML_EXPERIMENTAL_BEARER_TOKEN_REPLACE_PATTERN,
  TOML_MODEL_PATTERN,
  TOML_WIRE_API_PATTERN,
  TOML_MODEL_PROVIDER_LINE_PATTERN,
  type TomlAssignmentMatch,
  finalizeTomlText,
  getTomlSectionRange,
  getTopLevelEndIndex,
  getTomlSectionInsertIndex,
  findTomlAssignmentInRange,
  findTomlLineInRange,
  findTomlAssignments,
  isMcpServerSection,
  isOtherProviderSection,
  getTopLevelModelProviderLineIndex,
  escapeTomlBasicString,
} from "@/utils/tomlConfigUtils";

// Codex 保留 provider ID 常量
const CODEX_RESERVED_MODEL_PROVIDER_IDS = new Set([
  "amazon-bedrock",
  "openai",
  "ollama",
  "lmstudio",
  "oss",
  "ollama-chat",
]);

const CODEX_CHAT_WIRE_API_VALUES = new Set([
  "chat",
  "chat_completions",
  "chat-completions",
  "openai_chat",
  "openai-chat",
  "openai_chat_completions",
]);

const getRecoverableCodexProviderAssignments = (
  assignments: TomlAssignmentMatch[],
  targetSectionName: string | undefined,
): TomlAssignmentMatch[] =>
  assignments.filter(
    ({ sectionName }) =>
      sectionName !== targetSectionName &&
      !isMcpServerSection(sectionName) &&
      !isOtherProviderSection(sectionName, targetSectionName),
  );

const getCodexModelProviderName = (configText: string): string | undefined => {
  const normalized = normalizeTomlText(configText);
  try {
    const parsed = parseToml(normalized) as Record<string, any>;
    const providerName =
      typeof parsed.model_provider === "string"
        ? parsed.model_provider.trim()
        : undefined;
    if (providerName) return providerName;
  } catch {
    // Fall back to a top-level line scan while the user is editing invalid TOML.
  }

  const lines = normalized.split("\n");
  const index = getTopLevelModelProviderLineIndex(lines);
  if (index === -1) return undefined;
  const match = lines[index].match(TOML_MODEL_PROVIDER_LINE_PATTERN);
  const providerName = match?.[2]?.trim();
  return providerName || undefined;
};

const getCodexProviderSectionName = (
  configText: string,
): string | undefined => {
  const providerName = getCodexModelProviderName(configText);
  return providerName ? `model_providers.${providerName}` : undefined;
};

const isCustomCodexModelProviderId = (providerName: string): boolean => {
  const id = providerName.trim().toLowerCase();
  return Boolean(id) && !CODEX_RESERVED_MODEL_PROVIDER_IDS.has(id);
};

const getCodexCustomProviderSectionName = (
  configText: string,
): string | undefined => {
  const providerName = getCodexModelProviderName(configText);
  return providerName && isCustomCodexModelProviderId(providerName)
    ? `model_providers.${providerName}`
    : undefined;
};

// 判断给定的 wire_api 字符串是否表示 Codex 的 Chat Completions 协议
export const isCodexChatWireApi = (
  wireApi: string | undefined | null,
): boolean =>
  CODEX_CHAT_WIRE_API_VALUES.has((wireApi ?? "").trim().toLowerCase());

// 从 Codex 的 TOML 配置文本中提取 wire_api（支持单/双引号）
export const extractCodexWireApi = (
  configText: string | undefined | null,
): string | undefined => {
  try {
    const raw = typeof configText === "string" ? configText : "";
    const text = normalizeTomlText(raw);
    if (!text) return undefined;

    const lines = text.split("\n");
    const targetSectionName = getCodexProviderSectionName(text);

    if (targetSectionName) {
      const sectionRange = getTomlSectionRange(lines, targetSectionName);
      if (sectionRange) {
        const match = findTomlAssignmentInRange(
          lines,
          TOML_WIRE_API_PATTERN,
          sectionRange.bodyStartIndex,
          sectionRange.bodyEndIndex,
          targetSectionName,
        );
        if (match?.value) {
          return match.value;
        }
      }
    }

    const topLevelMatch = findTomlAssignmentInRange(
      lines,
      TOML_WIRE_API_PATTERN,
      0,
      getTopLevelEndIndex(lines),
    );
    if (topLevelMatch?.value) {
      return topLevelMatch.value;
    }

    const fallbackAssignments = getRecoverableCodexProviderAssignments(
      findTomlAssignments(lines, TOML_WIRE_API_PATTERN),
      targetSectionName,
    );
    return fallbackAssignments.length === 1
      ? fallbackAssignments[0].value
      : undefined;
  } catch {
    return undefined;
  }
};

// 在 Codex 的 TOML 配置文本中写入或更新 wire_api 字段
export const setCodexWireApi = (
  configText: string,
  wireApi: "responses" | "chat",
): string => {
  const normalizedText = normalizeTomlText(configText);
  const lines = normalizedText ? normalizedText.split("\n") : [];
  const targetSectionName = getCodexProviderSectionName(normalizedText);
  const replacementLine = `wire_api = "${wireApi}"`;
  const allAssignments = findTomlAssignments(lines, TOML_WIRE_API_PATTERN);
  const recoverableAssignments = getRecoverableCodexProviderAssignments(
    allAssignments,
    targetSectionName,
  );

  if (targetSectionName) {
    let targetSectionRange = getTomlSectionRange(lines, targetSectionName);
    const targetMatch = targetSectionRange
      ? findTomlAssignmentInRange(
          lines,
          TOML_WIRE_API_PATTERN,
          targetSectionRange.bodyStartIndex,
          targetSectionRange.bodyEndIndex,
          targetSectionName,
        )
      : undefined;

    if (targetMatch) {
      lines[targetMatch.index] = replacementLine;
      return finalizeTomlText(lines);
    }

    if (recoverableAssignments.length === 1) {
      lines.splice(recoverableAssignments[0].index, 1);
      targetSectionRange = getTomlSectionRange(lines, targetSectionName);
    }

    if (targetSectionRange) {
      const insertIndex = getTomlSectionInsertIndex(lines, targetSectionRange);
      lines.splice(insertIndex, 0, replacementLine);
      return finalizeTomlText(lines);
    }

    if (lines.length > 0 && lines[lines.length - 1].trim() !== "") {
      lines.push("");
    }
    lines.push(`[${targetSectionName}]`, replacementLine);
    return finalizeTomlText(lines);
  }

  const topLevelEndIndex = getTopLevelEndIndex(lines);
  const topLevelMatch = findTomlAssignmentInRange(
    lines,
    TOML_WIRE_API_PATTERN,
    0,
    topLevelEndIndex,
  );
  if (topLevelMatch) {
    lines[topLevelMatch.index] = replacementLine;
    return finalizeTomlText(lines);
  }

  const modelProviderIndex = getTopLevelModelProviderLineIndex(lines);
  if (modelProviderIndex !== -1) {
    lines.splice(modelProviderIndex + 1, 0, replacementLine);
    return finalizeTomlText(lines);
  }

  if (lines.length === 0) {
    return `${replacementLine}\n`;
  }

  lines.splice(topLevelEndIndex, 0, replacementLine);
  return finalizeTomlText(lines);
};

// 从 Codex 的 TOML 配置文本中提取 base_url（支持单/双引号）
export const extractCodexBaseUrl = (
  configText: string | undefined | null,
): string | undefined => {
  try {
    const raw = typeof configText === "string" ? configText : "";
    const text = normalizeTomlText(raw);
    if (!text) return undefined;

    const lines = text.split("\n");
    const targetSectionName = getCodexProviderSectionName(text);

    if (targetSectionName) {
      const sectionRange = getTomlSectionRange(lines, targetSectionName);
      if (sectionRange) {
        const match = findTomlAssignmentInRange(
          lines,
          TOML_BASE_URL_PATTERN,
          sectionRange.bodyStartIndex,
          sectionRange.bodyEndIndex,
          targetSectionName,
        );
        if (match?.value) {
          return match.value;
        }
      }
    }

    const topLevelMatch = findTomlAssignmentInRange(
      lines,
      TOML_BASE_URL_PATTERN,
      0,
      getTopLevelEndIndex(lines),
    );
    if (topLevelMatch?.value) {
      return topLevelMatch.value;
    }

    const fallbackAssignments = getRecoverableCodexProviderAssignments(
      findTomlAssignments(lines, TOML_BASE_URL_PATTERN),
      targetSectionName,
    );
    return fallbackAssignments.length === 1
      ? fallbackAssignments[0].value
      : undefined;
  } catch {
    return undefined;
  }
};

// 从 Codex 的 TOML 配置文本中提取 experimental_bearer_token（兼容 Mobile 模式）
export const extractCodexExperimentalBearerToken = (
  configText: string | undefined | null,
): string | undefined => {
  try {
    const raw = typeof configText === "string" ? configText : "";
    const text = normalizeTomlText(raw);
    if (!text) return undefined;

    try {
      const parsed = parseToml(text) as Record<string, any>;
      const providerName =
        typeof parsed.model_provider === "string"
          ? parsed.model_provider.trim()
          : undefined;
      const providerToken =
        providerName &&
        isCustomCodexModelProviderId(providerName) &&
        parsed.model_providers &&
        typeof parsed.model_providers === "object" &&
        typeof parsed.model_providers[providerName]
          ?.experimental_bearer_token === "string"
          ? parsed.model_providers[
              providerName
            ].experimental_bearer_token.trim()
          : undefined;
      if (providerToken) return providerToken;
      const topLevelToken =
        typeof parsed.experimental_bearer_token === "string"
          ? parsed.experimental_bearer_token.trim()
          : undefined;
      if (topLevelToken) return topLevelToken;
    } catch {
      // Fall back to the line scanner for partially edited TOML.
    }

    const lines = text.split("\n");
    const targetSectionName = getCodexCustomProviderSectionName(text);

    if (targetSectionName) {
      const sectionRange = getTomlSectionRange(lines, targetSectionName);
      if (sectionRange) {
        const match = findTomlAssignmentInRange(
          lines,
          TOML_EXPERIMENTAL_BEARER_TOKEN_PATTERN,
          sectionRange.bodyStartIndex,
          sectionRange.bodyEndIndex,
          targetSectionName,
        );
        if (match?.value) {
          return match.value;
        }
      }
    }

    const topLevelMatch = findTomlAssignmentInRange(
      lines,
      TOML_EXPERIMENTAL_BEARER_TOKEN_PATTERN,
      0,
      getTopLevelEndIndex(lines),
    );
    return topLevelMatch?.value;
  } catch {
    return undefined;
  }
};

// 同步更新 Codex config.toml 中已有的 experimental_bearer_token
// 仅修改已存在的条目, 不主动新增——避免破坏未使用 Mobile 兼容模式的普通 third-party 配置
// token 为空时删除该行 (让用户能真正清空 API key, 而不是被 pickCodexApiKey 的 fallback 又填回去)
export const updateCodexExperimentalBearerToken = (
  configText: string,
  token: string,
): string => {
  const normalizedText = normalizeTomlText(configText);
  if (
    !normalizedText ||
    !normalizedText.includes("experimental_bearer_token")
  ) {
    return configText;
  }

  const lines = normalizedText.split("\n");
  const targetSectionName = getCodexCustomProviderSectionName(normalizedText);

  let tokenLineIndex = -1;
  if (targetSectionName) {
    const sectionRange = getTomlSectionRange(lines, targetSectionName);
    if (sectionRange) {
      const index = findTomlLineInRange(
        lines,
        TOML_EXPERIMENTAL_BEARER_TOKEN_REPLACE_PATTERN,
        sectionRange.bodyStartIndex,
        sectionRange.bodyEndIndex,
      );
      if (index !== -1) tokenLineIndex = index;
    }
  }
  if (tokenLineIndex === -1) {
    const topLevelIndex = findTomlLineInRange(
      lines,
      TOML_EXPERIMENTAL_BEARER_TOKEN_REPLACE_PATTERN,
      0,
      getTopLevelEndIndex(lines),
    );
    if (topLevelIndex !== -1) tokenLineIndex = topLevelIndex;
  }

  if (tokenLineIndex === -1) return configText;

  const trimmed = token.trim();
  if (!trimmed) {
    lines.splice(tokenLineIndex, 1);
  } else {
    const escaped = escapeTomlBasicString(trimmed);
    const existingLine = lines[tokenLineIndex];
    lines[tokenLineIndex] = TOML_EXPERIMENTAL_BEARER_TOKEN_REPLACE_PATTERN.test(
      existingLine,
    )
      ? existingLine.replace(
          TOML_EXPERIMENTAL_BEARER_TOKEN_REPLACE_PATTERN,
          `$1"${escaped}"$2`,
        )
      : `experimental_bearer_token = "${escaped}"`;
  }
  return finalizeTomlText(lines);
};

// 从 Provider 对象中提取 Codex base_url（当 settingsConfig.config 为 TOML 字符串时）
export const getCodexBaseUrl = (
  provider: { settingsConfig?: Record<string, any> } | undefined | null,
): string | undefined => {
  try {
    const text =
      typeof provider?.settingsConfig?.config === "string"
        ? (provider as any).settingsConfig.config
        : "";
    return extractCodexBaseUrl(text);
  } catch {
    return undefined;
  }
};

// 在 Codex 的 TOML 配置文本中写入或更新 base_url 字段
export const setCodexBaseUrl = (
  configText: string,
  baseUrl: string,
): string => {
  const trimmed = baseUrl.trim();
  const normalizedText = normalizeTomlText(configText);
  const lines = normalizedText ? normalizedText.split("\n") : [];
  const targetSectionName = getCodexProviderSectionName(normalizedText);
  const allAssignments = findTomlAssignments(lines, TOML_BASE_URL_PATTERN);
  const recoverableAssignments = getRecoverableCodexProviderAssignments(
    allAssignments,
    targetSectionName,
  );

  if (!trimmed) {
    if (!normalizedText) return normalizedText;

    if (targetSectionName) {
      const sectionRange = getTomlSectionRange(lines, targetSectionName);
      const targetMatch = sectionRange
        ? findTomlAssignmentInRange(
            lines,
            TOML_BASE_URL_PATTERN,
            sectionRange.bodyStartIndex,
            sectionRange.bodyEndIndex,
            targetSectionName,
          )
        : undefined;

      if (targetMatch) {
        lines.splice(targetMatch.index, 1);
        return finalizeTomlText(lines);
      }
    }

    if (recoverableAssignments.length === 1) {
      lines.splice(recoverableAssignments[0].index, 1);
      return finalizeTomlText(lines);
    }

    return finalizeTomlText(lines);
  }

  const normalizedUrl = trimmed.replace(/\s+/g, "");
  const replacementLine = `base_url = "${normalizedUrl}"`;

  if (targetSectionName) {
    let targetSectionRange = getTomlSectionRange(lines, targetSectionName);
    const targetMatch = targetSectionRange
      ? findTomlAssignmentInRange(
          lines,
          TOML_BASE_URL_PATTERN,
          targetSectionRange.bodyStartIndex,
          targetSectionRange.bodyEndIndex,
          targetSectionName,
        )
      : undefined;

    if (targetMatch) {
      lines[targetMatch.index] = replacementLine;
      return finalizeTomlText(lines);
    }

    if (recoverableAssignments.length === 1) {
      lines.splice(recoverableAssignments[0].index, 1);
      targetSectionRange = getTomlSectionRange(lines, targetSectionName);
    }

    if (targetSectionRange) {
      const insertIndex = getTomlSectionInsertIndex(lines, targetSectionRange);
      lines.splice(insertIndex, 0, replacementLine);
      return finalizeTomlText(lines);
    }

    if (lines.length > 0 && lines[lines.length - 1].trim() !== "") {
      lines.push("");
    }
    lines.push(`[${targetSectionName}]`, replacementLine);
    return finalizeTomlText(lines);
  }

  const topLevelEndIndex = getTopLevelEndIndex(lines);
  const topLevelMatch = findTomlAssignmentInRange(
    lines,
    TOML_BASE_URL_PATTERN,
    0,
    topLevelEndIndex,
  );
  if (topLevelMatch) {
    lines[topLevelMatch.index] = replacementLine;
    return finalizeTomlText(lines);
  }

  const modelProviderIndex = getTopLevelModelProviderLineIndex(lines);
  if (modelProviderIndex !== -1) {
    lines.splice(modelProviderIndex + 1, 0, replacementLine);
    return finalizeTomlText(lines);
  }

  if (lines.length === 0) {
    return `${replacementLine}\n`;
  }

  const insertIndex = topLevelEndIndex;
  lines.splice(insertIndex, 0, replacementLine);
  return finalizeTomlText(lines);
};

// ========== Codex model name utils ==========

// 从 Codex 的 TOML 配置文本中提取 model 字段（支持单/双引号）
export const extractCodexModelName = (
  configText: string | undefined | null,
): string | undefined => {
  try {
    const raw = typeof configText === "string" ? configText : "";
    const text = normalizeTomlText(raw);
    if (!text) return undefined;
    const lines = text.split("\n");
    const topLevelMatch = findTomlAssignmentInRange(
      lines,
      TOML_MODEL_PATTERN,
      0,
      getTopLevelEndIndex(lines),
    );
    return topLevelMatch?.value;
  } catch {
    return undefined;
  }
};

// 在 Codex 的 TOML 配置文本中写入或更新 model 字段
export const setCodexModelName = (
  configText: string,
  modelName: string,
): string => {
  const trimmed = modelName.trim();
  const normalizedText = normalizeTomlText(configText);
  const lines = normalizedText ? normalizedText.split("\n") : [];
  const topLevelEndIndex = getTopLevelEndIndex(lines);
  const topLevelMatch = findTomlAssignmentInRange(
    lines,
    TOML_MODEL_PATTERN,
    0,
    topLevelEndIndex,
  );

  if (!trimmed) {
    if (!normalizedText) return normalizedText;
    if (topLevelMatch) {
      lines.splice(topLevelMatch.index, 1);
    }
    return finalizeTomlText(lines);
  }

  const replacementLine = `model = "${trimmed}"`;
  if (topLevelMatch) {
    lines[topLevelMatch.index] = replacementLine;
    return finalizeTomlText(lines);
  }

  const modelProviderIndex = getTopLevelModelProviderLineIndex(lines);
  if (modelProviderIndex !== -1) {
    lines.splice(modelProviderIndex + 1, 0, replacementLine);
    return finalizeTomlText(lines);
  }

  if (lines.length === 0) {
    return `${replacementLine}\n`;
  }

  lines.splice(topLevelEndIndex, 0, replacementLine);
  return finalizeTomlText(lines);
};
