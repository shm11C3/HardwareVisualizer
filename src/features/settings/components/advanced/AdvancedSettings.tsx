import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { EXTERNAL_COMPONENT_DOCS_BASE_URL } from "@/components/shared/externalComponentGuidance";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { useTauriStore } from "@/hooks/useTauriStore";
import { openURL } from "@/lib/openUrl";

export const AdvancedSettings = () => {
  const { t } = useTranslation();
  const [showGpuUsageSource, setShowGpuUsageSource, isPending] = useTauriStore(
    "showGpuUsageSource",
    false,
  );

  return (
    <div className="p-4">
      <h3 className="py-3 font-bold text-2xl">
        {t("pages.settings.advanced.name")}
      </h3>
      <div className="px-4">
        <div className="flex w-full items-center justify-between space-x-4 py-6 xl:w-1/3">
          <div className="space-y-0.5">
            <Label htmlFor="showGpuUsageSource" className="text-lg">
              {t("pages.settings.advanced.showGpuUsageSource")}
            </Label>
            <p className="text-muted-foreground text-sm">
              {t("pages.settings.advanced.showGpuUsageSourceDescription")}
            </p>
          </div>

          {!isPending ? (
            <Switch
              id="showGpuUsageSource"
              checked={showGpuUsageSource ?? false}
              onCheckedChange={setShowGpuUsageSource}
            />
          ) : (
            <Skeleton className="h-6 w-11 rounded-full" />
          )}
        </div>
        <div className="flex w-full flex-col items-start gap-4 py-6 sm:flex-row sm:items-center sm:justify-between xl:w-1/2">
          <div className="space-y-0.5">
            <Label className="text-lg">
              {t("pages.settings.advanced.externalComponents")}
            </Label>
            <p className="text-muted-foreground text-sm">
              {t("pages.settings.advanced.externalComponentsDescription")}
            </p>
          </div>

          <Button
            onClick={() => openURL(EXTERNAL_COMPONENT_DOCS_BASE_URL)}
            type="button"
            variant="secondary"
          >
            {t("pages.settings.advanced.openExternalComponentsDocs")}
            <ArrowSquareOutIcon size={16} />
          </Button>
        </div>
      </div>
    </div>
  );
};
