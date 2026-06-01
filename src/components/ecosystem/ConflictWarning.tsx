import { useTranslation } from "react-i18next";

interface ConflictWarningProps {
  conflicts: { file: string; frameworks: string[] }[];
}

export function ConflictWarning({ conflicts }: ConflictWarningProps) {
  const { t } = useTranslation();

  if (conflicts.length === 0) return null;

  return (
    <div className="rounded-lg border border-yellow-500/30 bg-yellow-500/5 p-3 text-sm">
      <div className="font-medium text-yellow-500 mb-1">
        {t("ecosystem.conflictWarning")}
      </div>
      {conflicts.map(({ file, frameworks }) => (
        <div key={file} className="text-muted-foreground">
          <span className="font-mono text-xs">{file}</span>:{" "}
          {frameworks.join(" + ")}
        </div>
      ))}
      <div className="text-xs text-muted-foreground mt-1">
        {t("ecosystem.conflictMergeHint")}
      </div>
      <div className="text-xs text-muted-foreground">
        {t("ecosystem.conflictScalarHint")}
      </div>
    </div>
  );
}
