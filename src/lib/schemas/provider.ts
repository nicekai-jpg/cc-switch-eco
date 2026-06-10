import { z } from "zod";
import { parseJsonError } from "@/utils/jsonErrorUtils";

export const providerSchema = z.object({
  name: z.string(), // 必填校验移至 handleSubmit 中用 toast 提示
  websiteUrl: z.string().url("请输入有效的网址").optional().or(z.literal("")),
  notes: z.string().optional(),
  settingsConfig: z
    .string()
    .min(1, "请填写配置内容")
    .superRefine((value, ctx) => {
      try {
        JSON.parse(value);
      } catch (error) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: parseJsonError(error, "配置 JSON 格式错误"),
        });
      }
    }),
  // 图标配置
  icon: z.string().optional(),
  iconColor: z.string().optional(),
});

export type ProviderFormData = z.infer<typeof providerSchema>;
