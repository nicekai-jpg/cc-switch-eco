import { z } from "zod";
import { validateToml, tomlToMcpServer } from "@/utils/tomlUtils";
import { parseJsonError } from "@/utils/jsonErrorUtils";

/**
 * 通用的 JSON 配置文本校验：
 * - 非空
 * - 可解析且为对象（非数组）
 */
export const jsonConfigSchema = z
  .string()
  .min(1, "配置不能为空")
  .superRefine((value, ctx) => {
    try {
      const obj = JSON.parse(value);
      if (!obj || typeof obj !== "object" || Array.isArray(obj)) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: "需为单个对象配置",
        });
      }
    } catch (e) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: parseJsonError(e),
      });
    }
  });

/**
 * 通用的 TOML 配置文本校验：
 * - 允许为空（由上层业务决定是否必填）
 * - 语法与结构有效
 * - 针对 stdio/http/sse 的必填字段（command/url）进行提示
 */
export const tomlConfigSchema = z.string().superRefine((value, ctx) => {
  const err = validateToml(value);
  if (err) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: `TOML 无效：${err}`,
    });
    return;
  }

  if (!value.trim()) return;

  try {
    const server = tomlToMcpServer(value);
    if (server.type === "stdio" && !server.command?.trim()) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "stdio 类型需填写 command",
      });
    }
    if (
      (server.type === "http" || server.type === "sse") &&
      !server.url?.trim()
    ) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: `${server.type} 类型需填写 url`,
      });
    }
  } catch (e: any) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: e?.message || "TOML 解析失败",
    });
  }
});
