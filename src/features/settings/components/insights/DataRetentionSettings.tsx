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

const storageSmartRetentionPresets = [
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
    setStorageSmartRetentionDays,
  } = useSettingsAtom();
  const { t } = useTranslation();
  const [hasSettingChanged, setHasSettingChanged] = useAtom(
    settingAtoms.isRequiredRestart,
  );

  const holdingPeriodId = useId();
  const scheduledDataDeletionId = useId();
  const storageSmartRetentionId = useId();
  const storageSmartRetentionDays =
    settings.storageSmart.retentionDays ?? 365 * 3;
  const [storageSmartRetentionInput, setStorageSmartRetentionInput] = useState(
    () => String(storageSmartRetentionDays),
  );

  useEffect(() => {
    setStorageSmartRetentionInput(String(storageSmartRetentionDays));
  }, [storageSmartRetentionDays]);

  const changeNumberOfDays = async (value: number) => {
    await setHardwareArchiveRefreshIntervalDays(value);
    setHasSettingChanged(true);
  };

  const handleScheduledDataDeletion = async (value: boolean) => {
    await setScheduledDataDeletion(value);
    setHasSettingChanged(true);
  };

  const changeStorageSmartRetentionDays = async (value: number) => {
    if (value === storageSmartRetentionDays) {
      return true;
    }

    const saved = await setStorageSmartRetentionDays(value);
    if (!saved) {
      return false;
    }

    setHasSettingChanged(true);
    return true;
  };

  const parseStorageSmartRetentionDays = (value: string) => {
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

  const commitStorageSmartRetentionInput = async () => {
    const nextValue = parseStorageSmartRetentionDays(
      storageSmartRetentionInput,
    );
    if (nextValue === null) {
      setStorageSmartRetentionInput(String(storageSmartRetentionDays));
      return;
    }

    if (nextValue === storageSmartRetentionDays) {
      setStorageSmartRetentionInput(String(storageSmartRetentionDays));
      return;
    }

    const saved = await changeStorageSmartRetentionDays(nextValue);
    if (!saved) {
      setStorageSmartRetentionInput(String(storageSmartRetentionDays));
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
        <Label className="my-4 text-lg" htmlFor={storageSmartRetentionId}>
          {t("pages.settings.insights.storageSmart.retention.title")}
        </Label>
        <p className="mt-2 whitespace-pre-wrap text-sm">
          {t("pages.settings.insights.storageSmart.retention.description")}
        </p>
        <div className="mt-2 flex items-center">
          <Input
            id={storageSmartRetentionId}
            type="number"
            placeholder={t(
              "pages.settings.insights.storageSmart.retention.placeHolder",
            )}
            value={storageSmartRetentionInput}
            onChange={(e) => setStorageSmartRetentionInput(e.target.value)}
            onBlur={() => {
              void commitStorageSmartRetentionInput();
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
          {storageSmartRetentionPresets.map((preset) => {
            const isSelected = storageSmartRetentionDays === preset.value;

            return (
              <Button
                key={preset.value}
                type="button"
                size="sm"
                variant={isSelected ? "default" : "outline"}
                aria-pressed={isSelected}
                onClick={async () => {
                  const saved = await changeStorageSmartRetentionDays(
                    preset.value,
                  );
                  if (saved) {
                    setStorageSmartRetentionInput(String(preset.value));
                  }
                }}
              >
                {t(
                  `pages.settings.insights.storageSmart.retention.presets.${preset.labelKey}`,
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
