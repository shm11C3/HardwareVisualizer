import { useAtom } from "jotai";
import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { commands } from "@/rspc/bindings";
import { settingAtoms } from "@/store/ui";

export const DataRetentionSettings = () => {
  const {
    settings,
    setHardwareArchiveRefreshIntervalDays,
    setScheduledDataDeletion,
  } = useSettingsAtom();
  const { t } = useTranslation();
  const [hasSettingChanged, setHasSettingChanged] = useAtom(
    settingAtoms.isRequiredRestart,
  );

  const holdingPeriodId = useId();
  const scheduledDataDeletionId = useId();

  const changeNumberOfDays = async (value: number) => {
    await setHardwareArchiveRefreshIntervalDays(value);
    setHasSettingChanged(true);
  };

  const handleScheduledDataDeletion = async (value: boolean) => {
    await setScheduledDataDeletion(value);
    setHasSettingChanged(true);
  };

  return (
    <div className="py-4">
      <h4 className="font-bold text-xl">
        {t("pages.settings.insights.scheduledDataDeletion")}
      </h4>

      <p className="mt-2 whitespace-pre-wrap text-sm">
        {t("pages.settings.insights.holdingPeriod.description")}
      </p>

      <div className="py-4">
        <Label className="my-4 text-lg" htmlFor={holdingPeriodId}>
          {t("pages.settings.insights.holdingPeriod.title")}
        </Label>
        <div className="flex items-center justify-between">
          <div className="mt-2 flex items-center">
            <Input
              id={holdingPeriodId}
              type="number"
              placeholder={t(
                "pages.settings.insights.holdingPeriod.placeHolder",
              )}
              value={settings.hardwareArchive.refreshIntervalDays}
              onChange={(e) => changeNumberOfDays(Number(e.target.value))}
              min={1}
              max={100000}
              disabled={!settings.hardwareArchive.scheduledDataDeletion}
            />
            <span className="ml-2">{t("shared.time.days")}</span>
          </div>
          <div className="flex items-center space-x-2">
            <Checkbox
              id={scheduledDataDeletionId}
              checked={settings.hardwareArchive.scheduledDataDeletion}
              onCheckedChange={handleScheduledDataDeletion}
            />
            <Label
              htmlFor={scheduledDataDeletionId}
              className="flex items-center space-x-2 text-lg"
            >
              {t("pages.settings.insights.scheduledDataDeletionButton")}
            </Label>
          </div>
        </div>
      </div>
      <div className="flex items-center justify-end py-2">
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                onClick={commands.restartApp}
                disabled={!hasSettingChanged}
              >
                {t("pages.settings.insights.needRestart.restart")}
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              <p className="whitespace-pre-wrap">
                {t("pages.settings.insights.needRestart.description")}
              </p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </div>
    </div>
  );
};
