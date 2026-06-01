import { useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Trees, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import {
  useAllEcosystems,
  useCreateEcosystem,
  useSwitchEcosystem,
  useDeleteEcosystem,
  useAllFrameworks,
} from "@/hooks/useEcosystem";
import { extractErrorMessage } from "@/utils/errorUtils";
import { EcoCard } from "./EcoCard";
import { ConflictWarning } from "./ConflictWarning";

interface EcosystemPanelProps {
  onOpenChange?: (open: boolean) => void;
}

export function EcosystemPanel(_props: EcosystemPanelProps) {
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
      resetCreateForm();
    } catch (e: unknown) {
      toast.error(extractErrorMessage(e) || t("ecosystem.createFailed"));
    }
  };

  const handleSwitch = async (id: string) => {
    try {
      await switchMutation.mutateAsync(id);
      toast.success(t("ecosystem.switched"));
    } catch (e: unknown) {
      toast.error(extractErrorMessage(e) || t("ecosystem.switchFailed"));
    }
  };

  const handleDelete = async () => {
    if (!confirmDelete) return;
    try {
      await deleteMutation.mutateAsync(confirmDelete.id);
      toast.success(t("ecosystem.deleted", { name: confirmDelete.name }));
      setConfirmDelete(null);
    } catch (e: unknown) {
      toast.error(extractErrorMessage(e) || t("ecosystem.deleteFailed"));
    }
  };

  const resetCreateForm = () => {
    setNewName("");
    setNewDescription("");
    setSelectedFrameworks([]);
    setIsCreating(false);
  };

  const toggleFramework = (fwId: string) => {
    setSelectedFrameworks((prev) =>
      prev.includes(fwId) ? prev.filter((id) => id !== fwId) : [...prev, fwId],
    );
  };

  const rootFileConflicts = useMemo(() => {
    if (selectedFrameworks.length < 2) return [];
    const selectedFws = allFrameworks.filter((fw) =>
      selectedFrameworks.includes(fw.id),
    );
    const fileToFrameworks = new Map<string, string[]>();
    for (const fw of selectedFws) {
      for (const file of fw.isolatedFiles ?? []) {
        const existing = fileToFrameworks.get(file) ?? [];
        existing.push(fw.name);
        fileToFrameworks.set(file, existing);
      }
    }
    return Array.from(fileToFrameworks.entries())
      .filter(([, fws]) => fws.length > 1)
      .map(([file, fws]) => ({ file, frameworks: fws }));
  }, [selectedFrameworks, allFrameworks]);

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
                        {(fw.providedDirs ?? []).map((dir) => (
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

          <ConflictWarning conflicts={rootFileConflicts} />

          <div className="flex gap-2 justify-end">
            <Button variant="ghost" size="sm" onClick={resetCreateForm}>
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
                onDelete={(id, name) => setConfirmDelete({ id, name })}
                switchPending={switchMutation.isPending}
                allFrameworks={allFrameworks}
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
