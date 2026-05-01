import { AlertTriangleIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useCloseToTrayPreference } from "@/hooks/useCloseToTrayPreference";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

export const CloseToTrayToggle = () => {
  const { t } = useTranslation();
  const { error } = useTauriDialog();
  const { closeToTray, isPending, setCloseToTray } = useCloseToTrayPreference();
  const [isAvailable, setIsAvailable] = useState(true);

  // biome-ignore lint/correctness/useExhaustiveDependencies: Availability is checked once when the settings row mounts.
  useEffect(() => {
    const checkAvailability = async () => {
      try {
        const result = await commands.isCloseToTrayAvailable();

        if (isError(result)) {
          setIsAvailable(false);
          console.error(
            "Failed to check close-to-tray availability:",
            result.error,
          );
          await error(t("closeToTray.errors.availability"));
          return;
        }

        setIsAvailable(result.data);
      } catch (err) {
        setIsAvailable(false);
        console.error("Failed to check close-to-tray availability:", err);
        await error(t("closeToTray.errors.availability"));
      }
    };

    checkAvailability();
  }, []);

  const updateCloseToTray = async (value: boolean) => {
    const saved = await setCloseToTray(value);

    if (!saved) {
      await error(t("closeToTray.errors.savePreference"));
    }
  };

  return (
    <div className="space-y-3 py-6 xl:w-1/2">
      <div className="flex w-full items-center justify-between gap-4">
        <div className="space-y-1">
          <Label htmlFor="closeToTray" className="text-lg">
            {t("pages.settings.general.closeToTray.name")}
          </Label>
          <p className="text-muted-foreground text-sm">
            {t("pages.settings.general.closeToTray.description")}
          </p>
        </div>

        <Switch
          id="closeToTray"
          checked={isAvailable && closeToTray}
          disabled={isPending || !isAvailable}
          onCheckedChange={updateCloseToTray}
        />
      </div>

      {!isAvailable && (
        <div className="flex items-start gap-2 rounded-md border border-yellow-500/40 bg-yellow-500/10 p-3 text-sm">
          <AlertTriangleIcon className="mt-0.5 size-4 shrink-0 text-yellow-600" />
          <p>{t("pages.settings.general.closeToTray.unavailable")}</p>
        </div>
      )}
    </div>
  );
};
