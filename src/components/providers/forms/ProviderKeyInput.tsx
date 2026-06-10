import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";

export interface ProviderKeyInputProps {
  /** 输入框 id */
  inputId: string;
  /** 标签文本（已翻译） */
  label: string;
  /** 占位符文本（已翻译） */
  placeholder: string;
  /** 当前值 */
  value: string;
  /** 值变更回调 */
  onChange: (value: string) => void;
  /** 是否锁定（已存在于应用配置中） */
  isLocked: boolean;
  /** 锁定状态是否加载中 */
  isLockStateLoading: boolean;
  /** 是否与已有 key 重复 */
  isDuplicate: boolean;
  /** 重复提示文本 */
  duplicateMessage: string;
  /** 格式无效提示文本 */
  invalidMessage: string;
  /** 锁定状态提示文本 */
  lockedHint: string;
  /** 正常状态提示文本 */
  normalHint: string;
}

const KEY_PATTERN = /^[a-z0-9]+(-[a-z0-9]+)*$/;

/**
 * 通用 Provider Key 输入组件
 *
 * 统一 opencode / openclaw / hermes 的 providerKey 输入 UI，
 * 消除 ProviderForm.tsx 中三段几乎相同的 JSX。
 */
export function ProviderKeyInput({
  inputId,
  label,
  placeholder,
  value,
  onChange,
  isLocked,
  isLockStateLoading,
  isDuplicate,
  duplicateMessage,
  invalidMessage,
  lockedHint,
  normalHint,
}: ProviderKeyInputProps) {
  const isInvalid = value.trim() !== "" && !KEY_PATTERN.test(value);
  const showDuplicate = isDuplicate && !isLocked;
  const showInvalid = isInvalid;
  const showHint = !showDuplicate && (value.trim() === "" || KEY_PATTERN.test(value));

  return (
    <div className="space-y-2">
      <Label htmlFor={inputId}>
        {label}
        <span className="text-destructive ml-1">*</span>
      </Label>
      <Input
        id={inputId}
        value={value}
        onChange={(e) =>
          onChange(e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, ""))
        }
        placeholder={placeholder}
        disabled={isLocked || isLockStateLoading}
        className={
          showDuplicate || showInvalid ? "border-destructive" : ""
        }
      />
      {showDuplicate && (
        <p className="text-xs text-destructive">{duplicateMessage}</p>
      )}
      {showInvalid && (
        <p className="text-xs text-destructive">{invalidMessage}</p>
      )}
      {showHint && (
        <p className="text-xs text-muted-foreground">
          {isLocked ? lockedHint : normalHint}
        </p>
      )}
    </div>
  );
}
