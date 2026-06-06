import { useAtom } from "jotai";
import { useEffect, useId, useState } from "react";
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

const storageHealthRetentionPresets = [
  { labelKey: "halfYear", value: 183 },
  { labelKey: "oneYear", value: 365 },
  { labelKey: "threeYears", value: 365 * 3 },
  { labelKey: "fiveYears", value: 365 * 5 },
] as const;

export const DataRetentionSettings = () => {
  const {
    settings,
    setHardwareArchiveRefreshIntervalDays,
    setScheduledDataDeletion,
    setStorageHealthRetentionDays,
  } = useSettingsAtom();
  const { t } = useTranslation();
  const [hasSettingChanged, setHasSettingChanged] = useAtom(
    settingAtoms.isRequiredRestart,
  );

  const holdingPeriodId = useId();
  const scheduledDataDeletionId = useId();
  const storageHealthRetentionId = useId();
  const storageHealthRetentionDays =
    settings.storageHealth.retentionDays ?? 365 * 3;
  const [storageHealthRetentionInput, setStorageHealthRetentionInput] =
    useState(() => String(storageHealthRetentionDays));

  useEffect(() => {
    setStorageHealthRetentionInput(String(storageHealthRetentionDays));
  }, [storageHealthRetentionDays]);

  const changeNumberOfDays = async (value: number) => {
    await setHardwareArchiveRefreshIntervalDays(value);
    setHasSettingChanged(true);
  };

  const handleScheduledDataDeletion = async (value: boolean) => {
    await setScheduledDataDeletion(value);
    setHasSettingChanged(true);
  };

  const changeStorageHealthRetentionDays = async (value: number) => {
    if (value === storageHealthRetentionDays) {
      return true;
    }

    const saved = await setStorageHealthRetentionDays(value);
    if (!saved) {
      return false;
    }

    setHasSettingChanged(true);
    return true;
  };

  const parseStorageHealthRetentionDays = (value: string) => {
    const trimmed = value.trim();
    if (!trimmed) {
      return null;
    }

    const parsed = Number(trimmed);
    if (!Number.isFinite(parsed) || parsed < 1) {
      return null;
    }

    return Math.trunc(parsed);
  };

  const commitStorageHealthRetentionInput = async () => {
    const nextValue = parseStorageHealthRetentionDays(
      storageHealthRetentionInput,
    );
    if (nextValue === null) {
      setStorageHealthRetentionInput(String(storageHealthRetentionDays));
      return;
    }

    if (nextValue === storageHealthRetentionDays) {
      setStorageHealthRetentionInput(String(storageHealthRetentionDays));
      return;
    }

    const saved = await changeStorageHealthRetentionDays(nextValue);
    if (!saved) {
      setStorageHealthRetentionInput(String(storageHealthRetentionDays));
    }
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
      <div className="py-4">
        <Label className="my-4 text-lg" htmlFor={storageHealthRetentionId}>
          {t("pages.settings.insights.storageHealth.retention.title")}
        </Label>
        <p className="mt-2 whitespace-pre-wrap text-sm">
          {t("pages.settings.insights.storageHealth.retention.description")}
        </p>
        <div className="mt-2 flex items-center">
          <Input
            id={storageHealthRetentionId}
            type="number"
            placeholder={t(
              "pages.settings.insights.storageHealth.retention.placeHolder",
            )}
            value={storageHealthRetentionInput}
            onChange={(e) => setStorageHealthRetentionInput(e.target.value)}
            onBlur={() => {
              void commitStorageHealthRetentionInput();
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.currentTarget.blur();
              }
            }}
            min={1}
            max={100000}
          />
          <span className="ml-2">{t("shared.time.days")}</span>
        </div>
        <div className="mt-3 flex flex-wrap gap-2">
          {storageHealthRetentionPresets.map((preset) => {
            const isSelected = storageHealthRetentionDays === preset.value;

            return (
              <Button
                key={preset.value}
                type="button"
                size="sm"
                variant={isSelected ? "default" : "outline"}
                aria-pressed={isSelected}
                onClick={async () => {
                  const saved = await changeStorageHealthRetentionDays(
                    preset.value,
                  );
                  if (saved) {
                    setStorageHealthRetentionInput(String(preset.value));
                  }
                }}
              >
                {t(
                  `pages.settings.insights.storageHealth.retention.presets.${preset.labelKey}`,
                )}
              </Button>
            );
          })}
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
