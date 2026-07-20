import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  Trash2,
  ArrowRightLeft,
  Check,
  Plus,
  RefreshCw,
  X,
  Package,
  Save,
  Loader2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  useEcosystemFrameworks,
  useAllFrameworks,
  useInstallFramework,
  useUninstallFramework,
  useUpdateFramework,
  useSaveUserPreferences,
  useEcosystemStatus,
} from "@/hooks/useEcosystem";
import { extractErrorMessage } from "@/utils/errorUtils";
import { FrameworkPicker } from "./FrameworkPicker";
import { ConflictWarning } from "./ConflictWarning";
import type { Ecosystem } from "@/lib/api/ecosystem";

interface EcoCardProps {
  eco: Ecosystem;
  expanded: boolean;
  onToggleExpand: () => void;
  onSwitch: (id: string) => void;
  onDelete: (id: string, name: string) => void;
  switchPending: boolean;
  allFrameworks: ReturnType<typeof useAllFrameworks>["data"];
}

export function EcoCard({
  eco,
  expanded,
  onToggleExpand,
  onSwitch,
  onDelete,
  switchPending,
  allFrameworks = [],
}: EcoCardProps) {
  const { t } = useTranslation();
  const { data: installedFrameworks = [] } = useEcosystemFrameworks(eco.id);
  const { data: status } = useEcosystemStatus(expanded ? eco.id : undefined);
  const installMutation = useInstallFramework();
  const uninstallMutation = useUninstallFramework();
  const updateMutation = useUpdateFramework();
  const savePrefMutation = useSaveUserPreferences();
  const [operatingFrameworkId, setOperatingFrameworkId] = useState<string | null>(null);
  const [showFrameworkPicker, setShowFrameworkPicker] = useState(false);

  const handleInstall = async (frameworkId: string) => {
    setOperatingFrameworkId(frameworkId);
    try {
      await installMutation.mutateAsync({ ecoId: eco.id, frameworkId });
      toast.success(t("ecosystem.frameworkInstalled", { name: frameworkId }));
    } catch (e: unknown) {
      toast.error(
        extractErrorMessage(e) || t("ecosystem.frameworkInstallFailed"),
      );
    } finally {
      setOperatingFrameworkId(null);
    }
  };

  const handleUninstall = async (frameworkId: string) => {
    setOperatingFrameworkId(frameworkId);
    try {
      await uninstallMutation.mutateAsync({ ecoId: eco.id, frameworkId });
      toast.success(t("ecosystem.frameworkUninstalled", { name: frameworkId }));
    } catch (e: unknown) {
      toast.error(
        extractErrorMessage(e) || t("ecosystem.frameworkUninstallFailed"),
      );
    } finally {
      setOperatingFrameworkId(null);
    }
  };

  const handleUpdate = async (frameworkId: string) => {
    setOperatingFrameworkId(frameworkId);
    try {
      await updateMutation.mutateAsync({ ecoId: eco.id, frameworkId });
      toast.success(t("ecosystem.frameworkUpdated", { name: frameworkId }));
    } catch (e: unknown) {
      toast.error(
        extractErrorMessage(e) || t("ecosystem.frameworkUpdateFailed"),
      );
    } finally {
      setOperatingFrameworkId(null);
    }
  };

  const handleSavePref = async () => {
    try {
      await savePrefMutation.mutateAsync({
        ecoId: eco.id,
        fileName: "settings.json",
      });
      toast.success(t("ecosystem.userPrefSaved"));
    } catch (e: unknown) {
      toast.error(extractErrorMessage(e) || t("ecosystem.userPrefSaveFailed"));
    }
  };

  return (
    <div
      className={`group glass rounded-xl border p-4 transition-colors ${
        eco.isCurrent
          ? "border-primary/40 bg-primary/5"
          : "border-white/10 hover:border-white/20"
      }`}
    >
      <div className="flex items-center justify-between">
        <div
          className="flex items-center gap-3 min-w-0 cursor-pointer flex-1"
          onClick={onToggleExpand}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => e.key === "Enter" && onToggleExpand()}
        >
          {eco.isCurrent && (
            <Check size={16} className="text-primary flex-shrink-0" />
          )}
          <div className="min-w-0">
            <div className="font-medium truncate">{eco.name}</div>
            {eco.description && (
              <div className="text-sm text-muted-foreground truncate">
                {eco.description}
              </div>
            )}
          </div>
        </div>
        <div className="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
          {!eco.isCurrent && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onSwitch(eco.id)}
              disabled={switchPending}
            >
              {switchPending ? (
                <Loader2 size={14} className="mr-1 animate-spin" />
              ) : (
                <ArrowRightLeft size={14} className="mr-1" />
              )}
              {switchPending
                ? t("ecosystem.switching")
                : t("ecosystem.switch")}
            </Button>
          )}
          <Button
            variant="ghost"
            size="sm"
            className="text-destructive hover:text-destructive"
            onClick={() => onDelete(eco.id, eco.name)}
            disabled={eco.isCurrent}
          >
            <Trash2 size={14} />
          </Button>
        </div>
      </div>

      {installedFrameworks.length > 0 && (
        <div className="flex gap-1 mt-2 flex-wrap">
          {installedFrameworks.map((fwId) => {
            const fw = allFrameworks?.find((f) => f.id === fwId);
            return (
              <span
                key={fwId}
                className="text-[10px] px-1.5 py-0.5 rounded bg-primary/10 text-primary border border-primary/20"
              >
                {fw?.name ?? fwId}
              </span>
            );
          })}
        </div>
      )}

      {expanded && (
        <div className="mt-4 pt-3 border-t border-white/10 space-y-3">
          <div className="flex items-center justify-between">
            <div className="text-sm font-medium">
              {t("ecosystem.installedFrameworks")}
            </div>
            <div className="flex items-center gap-2">
              {eco.isCurrent && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleSavePref}
                  disabled={savePrefMutation.isPending}
                  title={t("ecosystem.saveUserPref")}
                >
                  <Save size={12} className="mr-1" />
                  {t("ecosystem.saveUserPref")}
                </Button>
              )}
              <Button
                variant="outline"
                size="sm"
                onClick={() => setShowFrameworkPicker(!showFrameworkPicker)}
              >
                <Plus size={12} className="mr-1" />
                {t("ecosystem.addFramework")}
              </Button>
            </div>
          </div>

          {installedFrameworks.length === 0 ? (
            <div className="text-sm text-muted-foreground py-2">
              {t("ecosystem.noFrameworks")}
            </div>
          ) : (
            <div className="space-y-2">
              {installedFrameworks.map((fwId) => {
                const fw = allFrameworks?.find((f) => f.id === fwId);
                return (
                  <div
                    key={fwId}
                    className="flex items-center justify-between p-2 rounded-lg bg-muted/50"
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <Package
                        size={14}
                        className="text-muted-foreground flex-shrink-0"
                      />
                      <div className="min-w-0">
                        <div className="text-sm font-medium truncate">
                          {fw?.name ?? fwId}
                        </div>
                        <div className="flex gap-1 mt-0.5">
                          {(fw?.providedDirs ?? []).map((dir) => (
                            <span
                              key={dir}
                              className="text-[10px] px-1 py-0.5 rounded bg-muted text-muted-foreground"
                            >
                              {dir}
                            </span>
                          ))}
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-1 flex-shrink-0">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleUpdate(fwId)}
                        disabled={operatingFrameworkId !== null}
                        title={t("ecosystem.updateFramework")}
                      >
                        {operatingFrameworkId === fwId ? (
                          <Loader2 size={12} className="animate-spin" />
                        ) : (
                          <RefreshCw size={12} />
                        )}
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="text-destructive hover:text-destructive"
                        onClick={() => handleUninstall(fwId)}
                        disabled={operatingFrameworkId !== null}
                        title={t("ecosystem.uninstallFramework")}
                      >
                        {operatingFrameworkId === fwId ? (
                          <Loader2 size={12} className="animate-spin" />
                        ) : (
                          <X size={12} />
                        )}
                      </Button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          {showFrameworkPicker && (
            <FrameworkPicker
              allFrameworks={allFrameworks ?? []}
              installedFrameworks={installedFrameworks}
              onInstall={handleInstall}
              installingFrameworkId={operatingFrameworkId}
            />
          )}

          <ConflictWarning
            mergeConflicts={status?.mergeConflicts}
            installErrors={status?.installErrors}
          />
        </div>
      )}
    </div>
  );
}
