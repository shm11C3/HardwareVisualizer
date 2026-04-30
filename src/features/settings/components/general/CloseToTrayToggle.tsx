import { AlertTriangleIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useCloseToTrayPreference } from "@/hooks/useCloseToTrayPreference";
import { commands } from "@/rspc/bindings";

export const CloseToTrayToggle = () => {
  const { t } = useTranslation();
  const { closeToTray, isPending, setCloseToTray } = useCloseToTrayPreference();
  const [isAvailable, setIsAvailable] = useState(true);

  useEffect(() => {
    commands
      .isCloseToTrayAvailable()
      .then(setIsAvailable)
      .catch(() => setIsAvailable(true));
  }, []);

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
          onCheckedChange={setCloseToTray}
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
