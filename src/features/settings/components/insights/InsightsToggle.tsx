import { CheckCircleIcon, ProhibitInsetIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { NeedRestart } from "@/components/shared/System";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";

export const InsightsToggle = () => {
  const [alertOpen, setAlertOpen] = useState(false);
  const { t } = useTranslation();
  const { settings, toggleHardwareArchiveAtom } = useSettingsAtom();

  const handleCheckedChange = async (value: boolean) => {
    await toggleHardwareArchiveAtom(value);
    setAlertOpen(true);
  };

  return (
    <>
      <div className="space-y-6">
        <div className="flex items-center space-x-4 py-3">
          <div className="flex w-full flex-row items-center justify-between rounded-lg border border-zinc-800 p-4 dark:border-gray-100">
            <div className="space-y-0.5">
              <Label htmlFor="insight" className="flex items-center text-lg">
                {settings.hardwareArchive.enabled ? (
                  <CheckCircleIcon
                    className="fill-emerald-700 pr-2 dark:fill-emerald-300"
                    size={28}
                  />
                ) : (
                  <ProhibitInsetIcon
                    className="fill-neutral-600 pr-2 dark:fill-neutral-400"
                    size={28}
                  />
                )}
                {t(
                  `pages.settings.insights.${settings.hardwareArchive.enabled ? "disable" : "enable"}`,
                )}
              </Label>
            </div>

            <Switch
              checked={settings.hardwareArchive.enabled}
              onCheckedChange={handleCheckedChange}
            />
          </div>
        </div>
      </div>
      <NeedRestart alertOpen={alertOpen} setAlertOpen={setAlertOpen} />
    </>
  );
};
