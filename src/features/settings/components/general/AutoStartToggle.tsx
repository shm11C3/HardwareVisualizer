import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { useTauriDialog } from "@/hooks/useTauriDialog";

export const AutoStartToggle = () => {
  const { t } = useTranslation();
  const [autoStartEnabled, setAutoStartEnabled] = useState<boolean | null>(
    null,
  );
  const { error } = useTauriDialog();

  const toggleAutoStart = async (value: boolean) => {
    setAutoStartEnabled(value);

    try {
      value ? await enable() : await disable();
    } catch {
      error("Failed to set autostart");
      setAutoStartEnabled(!value);
    }
  };

  useEffect(() => {
    const checkAutoStart = async () => {
      const enabled = await isEnabled();
      setAutoStartEnabled(enabled);
    };

    checkAutoStart();
  }, []);

  return (
    <div className="flex w-full items-center justify-between space-x-4 py-6 xl:w-1/3">
      <div className="space-y-0.5">
        <Label htmlFor="insight" className="text-lg">
          {t("pages.settings.general.autostart.name")}
        </Label>
      </div>

      {autoStartEnabled != null ? (
        <Switch checked={autoStartEnabled} onCheckedChange={toggleAutoStart} />
      ) : (
        <Skeleton className="h-[24px] w-[44px] rounded-full" />
      )}
    </div>
  );
};
