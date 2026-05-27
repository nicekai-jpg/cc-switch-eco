import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Trees, Plus, Trash2, ArrowRightLeft, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import {
  useAllEcosystems,
  useCreateEcosystem,
  useSwitchEcosystem,
  useDeleteEcosystem,
} from "@/hooks/useEcosystem";

interface EcosystemPanelProps {
  onOpenChange?: (open: boolean) => void;
}

export function EcosystemPanel({ onOpenChange }: EcosystemPanelProps) {
  const { t } = useTranslation();
  const { data: ecosystems, isLoading } = useAllEcosystems();
  const createMutation = useCreateEcosystem();
  const switchMutation = useSwitchEcosystem();
  const deleteMutation = useDeleteEcosystem();

  const [isCreating, setIsCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [newDescription, setNewDescription] = useState("");
  const [confirmDelete, setConfirmDelete] = useState<{
    id: string;
    name: string;
  } | null>(null);

  const handleCreate = async () => {
    if (!newName.trim()) return;
    try {
      await createMutation.mutateAsync({
        name: newName.trim(),
        description: newDescription.trim(),
      });
      toast.success(t("ecosystem.created", { name: newName.trim() }));
      setNewName("");
      setNewDescription("");
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
          <div className="flex gap-2 justify-end">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setIsCreating(false);
                setNewName("");
                setNewDescription("");
              }}
            >
              {t("ecosystem.cancel")}
            </Button>
            <Button
              size="sm"
              onClick={handleCreate}
              disabled={!newName.trim() || createMutation.isPending}
            >
              {t("ecosystem.create")}
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
              <div
                key={eco.id}
                className={`group glass rounded-xl border p-4 transition-colors ${
                  eco.isCurrent
                    ? "border-primary/40 bg-primary/5"
                    : "border-white/10 hover:border-white/20"
                }`}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3 min-w-0">
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
                        onClick={() => handleSwitch(eco.id)}
                        disabled={switchMutation.isPending}
                      >
                        <ArrowRightLeft size={14} className="mr-1" />
                        {t("ecosystem.switch")}
                      </Button>
                    )}
                    <Button
                      variant="ghost"
                      size="sm"
                      className="text-destructive hover:text-destructive"
                      onClick={() =>
                        setConfirmDelete({ id: eco.id, name: eco.name })
                      }
                      disabled={eco.isCurrent}
                    >
                      <Trash2 size={14} />
                    </Button>
                  </div>
                </div>
              </div>
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

export default EcosystemPanel;