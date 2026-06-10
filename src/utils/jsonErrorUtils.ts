/**
 * JSON 语法错误解析工具
 *
 * 统一 common.ts 和 provider.ts 中重复的 parseJsonError 函数。
 * 合并两者的逻辑：提取位置信息 + 清理错误消息。
 */

/** 解析 JSON 语法错误，提取位置信息并返回友好提示 */
export function parseJsonError(
  error: unknown,
  prefix = "JSON 格式错误",
): string {
  if (!(error instanceof SyntaxError)) {
    return prefix;
  }

  const message = error.message || "JSON 解析失败";

  // Chrome/V8: "Unexpected token ... in JSON at position 123"
  const positionMatch = message.match(/at position (\d+)/i);
  if (positionMatch) {
    const position = parseInt(positionMatch[1], 10);
    const detail = message.split(" in JSON")[0];
    return `${prefix}：${detail}（位置：${position}）`;
  }

  // Firefox: "JSON.parse: unexpected character at line 1 column 23"
  const lineColumnMatch = message.match(/line (\d+) column (\d+)/i);
  if (lineColumnMatch) {
    const line = lineColumnMatch[1];
    const column = lineColumnMatch[2];
    return `${prefix}：第 ${line} 行，第 ${column} 列`;
  }

  // 通用情况：清理错误消息
  const cleanMessage = message
    .replace(/^JSON\.parse:\s*/i, "")
    .replace(/^Unexpected\s+/i, "意外的 ")
    .replace(/token/gi, "符号")
    .replace(/Expected/gi, "预期");

  return `${prefix}：${cleanMessage}`;
}
