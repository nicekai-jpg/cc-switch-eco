import { useTranslation } from "react-i18next";
import { AlertTriangle, AlertCircle } from "lucide-react";

interface ConflictWarningProps {
  /** Pre-creation: isolatedFiles overlap between selected frameworks */
  conflicts?: { file: string; frameworks: string[] }[];
  /** Post-install: mergeConflicts from eco.json (fileName -> conflict descriptions) */
  mergeConflicts?: Record<string, string[]>;
  /** Partial install failures during eco creation */
  installErrors?: string[];
}

export function ConflictWarning({
  conflicts,
  mergeConflicts,
  installErrors,
}: ConflictWarningProps) {
  const { t } = useTranslation();

  const hasPreConflicts = conflicts && conflicts.length > 0;
  const hasMergeConflicts =
    mergeConflicts && Object.keys(mergeConflicts).length > 0;
  const hasInstallErrors = installErrors && installErrors.length > 0;

  if (!hasPreConflicts && !hasMergeConflicts && !hasInstallErrors) return null;

  return (
    <div className="space-y-2">
      {/* Pre-creation conflicts (isolatedFiles overlap) */}
      {hasPreConflicts && (
        <div className="rounded-lg border border-yellow-500/30 bg-yellow-500/5 p-3 text-sm">
          <div className="font-medium text-yellow-500 mb-1 flex items-center gap-1.5">
            <AlertTriangle size={14} />
            {t("ecosystem.conflictWarning")}
          </div>
          {conflicts!.map(({ file, frameworks }) => (
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
      )}

      {/* Post-install merge conflicts */}
      {hasMergeConflicts && (
        <div className="rounded-lg border border-orange-500/30 bg-orange-500/5 p-3 text-sm">
          <div className="font-medium text-orange-500 mb-1 flex items-center gap-1.5">
            <AlertTriangle size={14} />
            {t("ecosystem.mergeConflictWarning")}
          </div>
          {Object.entries(mergeConflicts!).map(([file, conflictList]) => (
            <div key={file} className="mb-1">
              <div className="font-mono text-xs text-foreground">{file}</div>
              {conflictList.map((conflict, i) => (
                <div
                  key={i}
                  className="text-xs text-muted-foreground ml-2 truncate"
                  title={conflict}
                >
                  {conflict}
                </div>
              ))}
            </div>
          ))}
          <div className="text-xs text-muted-foreground mt-1">
            {t("ecosystem.mergeConflictHint")}
          </div>
        </div>
      )}

      {/* Partial install failures */}
      {hasInstallErrors && (
        <div className="rounded-lg border border-red-500/30 bg-red-500/5 p-3 text-sm">
          <div className="font-medium text-red-500 mb-1 flex items-center gap-1.5">
            <AlertCircle size={14} />
            {t("ecosystem.installErrorWarning")}
          </div>
          {installErrors!.map((error, i) => (
            <div
              key={i}
              className="text-xs text-muted-foreground truncate"
              title={error}
            >
              {error}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
