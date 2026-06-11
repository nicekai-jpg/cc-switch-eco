import type { AppId } from "@/lib/api";
import { toast } from "sonner";
import type { TFunction } from "i18next";

const KEY_PATTERN = /^[a-z0-9]+(-[a-z0-9]+)*$/;

export interface ProviderKeyValidationParams {
  appId: AppId;
  providerKey: string;
  isProviderKeyLocked: boolean;
  isProviderKeyLockStateLoading: boolean;
  additiveExistingProviderKeys: string[];
  t: TFunction;
}

/** 各 app 的 providerKey 验证消息 i18n key */
const KEY_MESSAGES: Record<string, { required: string; invalid: string; duplicate: string }> = {
  opencode: {
    required: "opencode.providerKeyRequired",
    invalid: "opencode.providerKeyInvalid",
    duplicate: "opencode.providerKeyDuplicate",
  },
  openclaw: {
    required: "openclaw.providerKeyRequired",
    invalid: "openclaw.providerKeyInvalid",
    duplicate: "openclaw.providerKeyDuplicate",
  },
  hermes: {
    required: "hermes.form.providerKeyRequired",
    invalid: "hermes.form.providerKeyInvalid",
    duplicate: "hermes.form.providerKeyDuplicate",
  },
};

/**
 * 验证 providerKey 的通用逻辑
 *
 * opencode / openclaw / hermes 三者验证逻辑完全相同：
 * 空值 → 格式 → 加载中 → 重复
 *
 * @returns true 表示验证通过，false 表示验证失败（已 toast 提示）
 */
export function validateProviderKey({
  appId,
  providerKey,
  isProviderKeyLocked,
  isProviderKeyLockStateLoading,
  additiveExistingProviderKeys,
  t,
}: ProviderKeyValidationParams): boolean {
  const messages = KEY_MESSAGES[appId];
  if (!messages) return true; // 无需验证的 app

  if (!providerKey.trim()) {
    toast.error(t(messages.required));
    return false;
  }

  if (!KEY_PATTERN.test(providerKey)) {
    toast.error(t(messages.invalid));
    return false;
  }

  if (isProviderKeyLockStateLoading) {
    toast.error(
      t("providerForm.providerKeyStatusLoading", {
        defaultValue: "正在加载供应商标识状态，请稍后再试",
      }),
    );
    return false;
  }

  if (!isProviderKeyLocked && additiveExistingProviderKeys.includes(providerKey)) {
    toast.error(t(messages.duplicate));
    return false;
  }

  return true;
}
