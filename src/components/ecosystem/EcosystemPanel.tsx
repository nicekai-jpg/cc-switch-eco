import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  Trees,
  Plus,
  Trash2,
  ArrowRightLeft,
  Check,
  Download,
  RefreshCw,
  X,
  Package,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import {
  useAllEcosystems,
  useCreateEcosystem,
  useSwitchEcosystem,
  useDeleteEcosystem,
  useAllFrameworks,
  useEcosystemFrameworks,
  useInstallFramework,
  useUninstallFramework,
  useUpdateFramework,
} from "@/hooks/useEcosystem";

interface EcosystemPanelProps {
  onOpenChange?: (open: boolean) => void;
}

export function EcosystemPanel({ onOpenChange: _onOpenChange }: EcosystemPanelProps) {
  const { t } = useTranslation();
  const { data: ecosystems, isLoading } = useAllEcosystems();
  const createMutation = useCreateEcosystem();
  const switchMutation = useSwitchEcosystem();
  const deleteMutation = useDeleteEcosystem();
  const { data: allFrameworks = [] } = useAllFrameworks();

  const [isCreating, setIsCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [newDescription, setNewDescription] = useState("");
  const [selectedFrameworks, setSelectedFrameworks] = useState<string[]>([]);
  const [confirmDelete, setConfirmDelete] = useState<{
    id: string;
    name: string;
  } | null>(null);
  const [expandedEco, setExpandedEco] = useState<string | null>(null);

  const handleCreate = async () => {
    if (!newName.trim()) return;
    try {
      await createMutation.mutateAsync({
        name: newName.trim(),
        description: newDescription.trim(),
        frameworks: selectedFrameworks,
      });
      toast.success(t("ecosystem.created", { name: newName.trim() }));
      setNewName("");
      setNewDescription("");
      setSelectedFrameworks([]);
      setIsCreating(false);
    } catch (e: any) {
      toast.error(e?.toString() || t("ecosystem.createFailed"));
    }
  };

  const handleSwitch = async (id: string) => {
    try {
      await switchMutation.mutateAsync(id);
      toast.success(t("ecosystem.switched"));
    } catch (e: any) {
      toast.error(e?.toString() || t("ecosystem.switchFailed"));
    }
  };

  const handleDelete = async () => {
    if (!confirmDelete) return;
    try {
      await deleteMutation.mutateAsync(confirmDelete.id);
      toast.success(t("ecosystem.deleted", { name: confirmDelete.name }));
      setConfirmDelete(null);
    } catch (e: any) {
      toast.error(e?.toString() || t("ecosystem.deleteFailed"));
    }
  };

  const toggleFramework = (fwId: string) => {
    setSelectedFrameworks((prev) =>
      prev.includes(fwId) ? prev.filter((id) => id !== fwId) : [...prev, fwId],
    );
  };

  return (
    <div className="flex flex-col flex-1 min-h-0 px-6">
      <div className="flex-shrink-0 py-4 glass rounded-xl border border-white/10 mb-4 px-6">
        <div className="flex items-center justify-between">
          <div className="text-sm text-muted-foreground">
            {isLoading
              ? t("ecosystem.loading")
              : t("ecosystem.count", { count: ecosystems?.length ?? 0 })}
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setIsCreating(true)}
            disabled={isCreating}
          >
            <Plus size={14} className="mr-1" />
            {t("ecosystem.create")}
          </Button>
        </div>
      </div>

      {isCreating && (
        <div className="flex-shrink-0 glass rounded-xl border border-white/10 mb-4 p-4 space-y-3">
          <Input
            placeholder={t("ecosystem.namePlaceholder")}
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleCreate()}
          />
          <Input
            placeholder={t("ecosystem.descriptionPlaceholder")}
            value={newDescription}
            onChange={(e) => setNewDescription(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleCreate()}
          />

          {/* 框架选择 */}
          {allFrameworks.length > 0 && (
            <div className="space-y-2">
              <div className="text-sm font-medium text-muted-foreground">
                {t("ecosystem.selectFrameworks")}
              </div>
              <div className="grid grid-cols-1 gap-2">
                {allFrameworks.map((fw) => (
                  <label
                    key={fw.id}
                    className={`flex items-start gap-3 p-3 rounded-lg border cursor-pointer transition-colors ${
                      selectedFrameworks.includes(fw.id)
                        ? "border-primary/40 bg-primary/5"
                        : "border-white/10 hover:border-white/20"
                    }`}
                  >
                    <input
                      type="checkbox"
                      checked={selectedFrameworks.includes(fw.id)}
                      onChange={() => toggleFramework(fw.id)}
                      className="mt-0.5"
                    />
                    <div className="min-w-0">
                      <div className="font-medium text-sm">{fw.name}</div>
                      <div className="text-xs text-muted-foreground mt-0.5">
                        {fw.description}
                      </div>
                      <div className="flex gap-1 mt-1 flex-wrap">
                        {fw.providedDirs.map((dir) => (
                          <span
                            key={dir}
                            className="text-[10px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground"
                          >
                            {dir}
                          </span>
                        ))}
                      </div>
                    </div>
                  </label>
                ))}
              </div>
            </div>
          )}

          <div className="flex gap-2 justify-end">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setIsCreating(false);
                setNewName("");
                setNewDescription("");
                setSelectedFrameworks([]);
              }}
            >
              {t("ecosystem.cancel")}
            </Button>
            <Button
              size="sm"
              onClick={handleCreate}
              disabled={!newName.trim() || createMutation.isPending}
            >
              {createMutation.isPending
                ? t("ecosystem.installing")
                : t("ecosystem.create")}
            </Button>
          </div>
        </div>
      )}

      <div className="flex-1 overflow-y-auto pb-16">
        {isLoading ? (
          <div className="text-center py-12 text-muted-foreground">
            {t("ecosystem.loading")}
          </div>
        ) : !ecosystems || ecosystems.length === 0 ? (
          <div className="text-center py-12">
            <div className="w-16 h-16 mx-auto mb-4 bg-muted rounded-full flex items-center justify-center">
              <Trees size={24} className="text-muted-foreground" />
            </div>
            <h3 className="text-lg font-medium text-foreground mb-2">
              {t("ecosystem.empty")}
            </h3>
            <p className="text-muted-foreground text-sm">
              {t("ecosystem.emptyDescription")}
            </p>
          </div>
        ) : (
          <div className="space-y-3">
            {ecosystems.map((eco) => (
              <EcoCard
                key={eco.id}
                eco={eco}
                expanded={expandedEco === eco.id}
                onToggleExpand={() =>
                  setExpandedEco(expandedEco === eco.id ? null : eco.id)
                }
                onSwitch={handleSwitch}
                onDelete={(id, name) =>
                  setConfirmDelete({ id, name })
                }
                switchPending={switchMutation.isPending}
              />
            ))}
          </div>
        )}
      </div>

      {confirmDelete && (
        <ConfirmDialog
          isOpen
          title={t("ecosystem.confirmDeleteTitle")}
          message={t("ecosystem.confirmDeleteMessage", {
            name: confirmDelete.name,
          })}
          onConfirm={handleDelete}
          onCancel={() => setConfirmDelete(null)}
        />
      )}
    </div>
  );
}

function EcoCard({
  eco,
  expanded,
  onToggleExpand,
  onSwitch,
  onDelete,
  switchPending,
}: {
  eco: { id: string; name: string; description: string; isCurrent: boolean };
  expanded: boolean;
  onToggleExpand: () => void;
  onSwitch: (id: string) => void;
  onDelete: (id: string, name: string) => void;
  switchPending: boolean;
}) {
  const { t } = useTranslation();
  const { data: installedFrameworks = [] } = useEcosystemFrameworks(eco.id);
  const installMutation = useInstallFramework();
  const uninstallMutation = useUninstallFramework();
  const updateMutation = useUpdateFramework();
  const { data: allFrameworks = [] } = useAllFrameworks();
  const [showFrameworkPicker, setShowFrameworkPicker] = useState(false);

  const handleInstall = async (frameworkId: string) => {
    try {
      await installMutation.mutateAsync({ ecoId: eco.id, frameworkId });
      toast.success(
        t("ecosystem.frameworkInstalled", { name: frameworkId }),
      );
    } catch (e: any) {
      toast.error(e?.toString() || t("ecosystem.frameworkInstallFailed"));
    }
  };

  const handleUninstall = async (frameworkId: string) => {
    try {
      await uninstallMutation.mutateAsync({ ecoId: eco.id, frameworkId });
      toast.success(
        t("ecosystem.frameworkUninstalled", { name: frameworkId }),
      );
    } catch (e: any) {
      toast.error(e?.toString() || t("ecosystem.frameworkUninstallFailed"));
    }
  };

  const handleUpdate = async (frameworkId: string) => {
    try {
      await updateMutation.mutateAsync({ ecoId: eco.id, frameworkId });
      toast.success(t("ecosystem.frameworkUpdated", { name: frameworkId }));
    } catch (e: any) {
      toast.error(e?.toString() || t("ecosystem.frameworkUpdateFailed"));
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
              <ArrowRightLeft size={14} className="mr-1" />
              {t("ecosystem.switch")}
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

      {/* 已安装框架标签 */}
      {installedFrameworks.length > 0 && (
        <div className="flex gap-1 mt-2 flex-wrap">
          {installedFrameworks.map((fwId) => {
            const fw = allFrameworks.find((f) => f.id === fwId);
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

      {/* 展开的框架管理区域 */}
      {expanded && (
        <div className="mt-4 pt-3 border-t border-white/10 space-y-3">
          <div className="flex items-center justify-between">
            <div className="text-sm font-medium">
              {t("ecosystem.installedFrameworks")}
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowFrameworkPicker(!showFrameworkPicker)}
            >
              <Plus size={12} className="mr-1" />
              {t("ecosystem.addFramework")}
            </Button>
          </div>

          {/* 已安装框架列表 */}
          {installedFrameworks.length === 0 ? (
            <div className="text-sm text-muted-foreground py-2">
              {t("ecosystem.noFrameworks")}
            </div>
          ) : (
            <div className="space-y-2">
              {installedFrameworks.map((fwId) => {
                const fw = allFrameworks.find((f) => f.id === fwId);
                return (
                  <div
                    key={fwId}
                    className="flex items-center justify-between p-2 rounded-lg bg-muted/50"
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <Package size={14} className="text-muted-foreground flex-shrink-0" />
                      <div className="min-w-0">
                        <div className="text-sm font-medium truncate">
                          {fw?.name ?? fwId}
                        </div>
                        <div className="flex gap-1 mt-0.5">
                          {fw?.providedDirs.map((dir) => (
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
                        disabled={updateMutation.isPending}
                        title={t("ecosystem.updateFramework")}
                      >
                        <RefreshCw size={12} />
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="text-destructive hover:text-destructive"
                        onClick={() => handleUninstall(fwId)}
                        disabled={uninstallMutation.isPending}
                        title={t("ecosystem.uninstallFramework")}
                      >
                        <X size={12} />
                      </Button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          {/* 框架选择器 */}
          {showFrameworkPicker && (
            <div className="space-y-2 pt-2">
              <div className="text-sm text-muted-foreground">
                {t("ecosystem.availableFrameworks")}
              </div>
              {allFrameworks
                .filter((fw) => !installedFrameworks.includes(fw.id))
                .map((fw) => (
                  <div
                    key={fw.id}
                    className="flex items-center justify-between p-2 rounded-lg border border-white/10 hover:border-white/20"
                  >
                    <div className="min-w-0">
                      <div className="text-sm font-medium">{fw.name}</div>
                      <div className="text-xs text-muted-foreground">
                        {fw.description}
                      </div>
                    </div>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleInstall(fw.id)}
                      disabled={installMutation.isPending}
                    >
                      <Download size={12} className="mr-1" />
                      {t("ecosystem.install")}
                    </Button>
                  </div>
                ))}
              {allFrameworks.filter((fw) => !installedFrameworks.includes(fw.id))
                .length === 0 && (
                <div className="text-sm text-muted-foreground py-2">
                  {t("ecosystem.allFrameworksInstalled")}
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default EcosystemPanel;