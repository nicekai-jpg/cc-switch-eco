import { useTranslation } from "react-i18next";
import { Download, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { FrameworkRegistry } from "@/lib/api/ecosystem";

interface FrameworkPickerProps {
  allFrameworks: FrameworkRegistry[];
  installedFrameworks: string[];
  onInstall: (frameworkId: string) => void;
  installingFrameworkId: string | null;
}

export function FrameworkPicker({
  allFrameworks,
  installedFrameworks,
  onInstall,
  installingFrameworkId,
}: FrameworkPickerProps) {
  const { t } = useTranslation();

  const availableFrameworks = allFrameworks.filter(
    (fw) => !installedFrameworks.includes(fw.id),
  );

  return (
    <div className="space-y-2 pt-2">
      <div className="text-sm text-muted-foreground">
        {t("ecosystem.availableFrameworks")}
      </div>
      {availableFrameworks.map((fw) => {
        const isInstalling = installingFrameworkId === fw.id;
        return (
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
              onClick={() => onInstall(fw.id)}
              disabled={installingFrameworkId !== null}
            >
              {isInstalling ? (
                <Loader2 size={12} className="mr-1 animate-spin" />
              ) : (
                <Download size={12} className="mr-1" />
              )}
              {isInstalling
                ? t("ecosystem.installing")
                : t("ecosystem.install")}
            </Button>
          </div>
        );
      })}
      {availableFrameworks.length === 0 && (
        <div className="text-sm text-muted-foreground py-2">
          {t("ecosystem.allFrameworksInstalled")}
        </div>
      )}
    </div>
  );
}
