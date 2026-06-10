// Codex 目标模式与顶级字段工具函数

import { normalizeTomlText } from "@/utils/textNormalization";
import { parse as parseToml } from "smol-toml";
import {
  TOML_GOALS_FEATURE_PATTERN,
  TOML_GOALS_FEATURE_REPLACE_PATTERN,
  finalizeTomlText,
  getTomlSectionRange,
  getTopLevelEndIndex,
  getTomlSectionInsertIndex,
  findTomlLineInRange,
  hasTomlSectionBodyContent,
} from "@/utils/tomlConfigUtils";

export const isCodexGoalModeEnabled = (
  configText: string | undefined | null,
): boolean => {
  try {
    const raw = typeof configText === "string" ? configText : "";
    const text = normalizeTomlText(raw);
    if (!text) return false;

    try {
      const parsed = parseToml(text) as Record<string, any>;
      return parsed.features?.goals === true;
    } catch {
      // Fall back to line scanning while the user is editing invalid TOML.
    }

    const lines = text.split("\n");
    const featureRange = getTomlSectionRange(lines, "features");
    if (!featureRange) return false;

    const index = findTomlLineInRange(
      lines,
      TOML_GOALS_FEATURE_PATTERN,
      featureRange.bodyStartIndex,
      featureRange.bodyEndIndex,
    );
    if (index === -1) return false;

    return lines[index].match(TOML_GOALS_FEATURE_PATTERN)?.[1] === "true";
  } catch {
    return false;
  }
};

export const setCodexGoalMode = (
  configText: string,
  enabled: boolean,
): string => {
  const normalizedText = normalizeTomlText(configText);
  const lines = normalizedText ? normalizedText.split("\n") : [];
  let featureRange = getTomlSectionRange(lines, "features");

  if (featureRange) {
    const goalLineIndex = findTomlLineInRange(
      lines,
      TOML_GOALS_FEATURE_REPLACE_PATTERN,
      featureRange.bodyStartIndex,
      featureRange.bodyEndIndex,
    );

    if (enabled) {
      if (goalLineIndex !== -1) {
        lines[goalLineIndex] = lines[goalLineIndex].replace(
          TOML_GOALS_FEATURE_REPLACE_PATTERN,
          "$1true$3",
        );
      } else {
        lines.splice(
          getTomlSectionInsertIndex(lines, featureRange),
          0,
          "goals = true",
        );
      }
      return finalizeTomlText(lines);
    }

    if (goalLineIndex !== -1) {
      lines.splice(goalLineIndex, 1);
      featureRange = getTomlSectionRange(lines, "features");
      if (featureRange && !hasTomlSectionBodyContent(lines, featureRange)) {
        lines.splice(
          featureRange.headerLineIndex,
          featureRange.bodyEndIndex - featureRange.headerLineIndex,
        );
      }
    }
    return finalizeTomlText(lines);
  }

  if (!enabled) return normalizedText;

  const topLevelEndIndex = getTopLevelEndIndex(lines);
  const sectionLines: string[] = [];
  if (topLevelEndIndex > 0 && lines[topLevelEndIndex - 1].trim() !== "") {
    sectionLines.push("");
  }
  sectionLines.push("[features]", "goals = true");
  if (
    topLevelEndIndex < lines.length &&
    lines[topLevelEndIndex]?.trim() !== ""
  ) {
    sectionLines.push("");
  }

  lines.splice(topLevelEndIndex, 0, ...sectionLines);
  return finalizeTomlText(lines);
};

// ========== Codex top-level integer field utils ==========

const tomlTopLevelIntPattern = (field: string) =>
  new RegExp(`^\\s*${field}\\s*=\\s*(\\d+)\\s*(?:#.*)?$`);

const findTopLevelIntMatch = (
  lines: string[],
  fieldName: string,
  topLevelEndIndex: number,
): { index: number; value: number } | undefined => {
  const pattern = tomlTopLevelIntPattern(fieldName);
  for (let i = 0; i < topLevelEndIndex; i += 1) {
    const m = lines[i].match(pattern);
    if (m) {
      return { index: i, value: Number(m[1]) };
    }
  }
  return undefined;
};

// 从 Codex TOML 配置中提取顶级整数字段
export const extractCodexTopLevelInt = (
  configText: string | undefined | null,
  fieldName: string,
): number | undefined => {
  try {
    const raw = typeof configText === "string" ? configText : "";
    const text = normalizeTomlText(raw);
    if (!text) return undefined;
    const lines = text.split("\n");
    return findTopLevelIntMatch(lines, fieldName, getTopLevelEndIndex(lines))
      ?.value;
  } catch {
    return undefined;
  }
};

// 在 Codex TOML 配置中设置或更新顶级整数字段
export const setCodexTopLevelInt = (
  configText: string,
  fieldName: string,
  value: number,
): string => {
  const normalizedText = normalizeTomlText(configText);
  const lines = normalizedText ? normalizedText.split("\n") : [];
  const topLevelEndIndex = getTopLevelEndIndex(lines);
  const existing = findTopLevelIntMatch(lines, fieldName, topLevelEndIndex);
  const replacementLine = `${fieldName} = ${value}`;

  if (existing) {
    lines[existing.index] = replacementLine;
    return finalizeTomlText(lines);
  }

  // 插入位置：顶级区域末尾（section header 之前）
  if (lines.length === 0) {
    return `${replacementLine}\n`;
  }

  lines.splice(topLevelEndIndex, 0, replacementLine);
  return finalizeTomlText(lines);
};

// 从 Codex TOML 配置中移除顶级字段行
export const removeCodexTopLevelField = (
  configText: string,
  fieldName: string,
): string => {
  const normalizedText = normalizeTomlText(configText);
  if (!normalizedText) return normalizedText;
  const lines = normalizedText.split("\n");
  const topLevelEndIndex = getTopLevelEndIndex(lines);
  const existing = findTopLevelIntMatch(lines, fieldName, topLevelEndIndex);
  if (existing) {
    lines.splice(existing.index, 1);
  }
  return finalizeTomlText(lines);
};
