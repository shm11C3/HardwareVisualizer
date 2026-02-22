import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { useTauriStore } from "@/hooks/useTauriStore";

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
      </div>
    </div>
  );
};
