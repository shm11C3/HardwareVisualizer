import { CheckCircleIcon, ProhibitInsetIcon } from "@phosphor-icons/react";
import { Info } from "lucide-react";
import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { getDefaultLHMSettings } from "../libreHardwareMonitorImport/lhmSettings";

/**
 * TODO データソースの優先順位設定を追加する
 */
export const AdvancedSettings = () => {
  const { t } = useTranslation();

  return (
    <div className="mt-8 p-4">
      <h3 className="py-3 font-bold text-2xl">
        {t("pages.settings.advanced.name")}
      </h3>
      <div className="p-4">
        <LibreHardwareMonitorImportSettings />
      </div>
    </div>
  );
};

const LibreHardwareMonitorImportSettings = () => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();
  const [testingConnection, setTestingConnection] = useState(false);
  const [connectionStatus, setConnectionStatus] = useState<
    "idle" | "success" | "error"
  >("idle");
  const [defaultOpen, setDefaultOpen] = useState(false);

  const toggleLibreHardwareMonitorImportId = useId();
  const hostInputId = useId();
  const portInputId = useId();
  const httpsCheckboxId = useId();
  const refreshIntervalId = useId();
  const timeoutId = useId();

  const toggleImport = async (value: boolean) => {
    setDefaultOpen(value);
    await updateSettingAtom(
      "libreHardwareMonitorImport",
      getDefaultLHMSettings(value, {
        currentSettings: settings.libreHardwareMonitorImport,
      }),
    );
  };

  const updateImportSetting = async <
    K extends keyof NonNullable<typeof settings.libreHardwareMonitorImport>,
  >(
    key: K,
    value: NonNullable<typeof settings.libreHardwareMonitorImport>[K],
  ) => {
    await updateSettingAtom(
      "libreHardwareMonitorImport",
      getDefaultLHMSettings(true, {
        currentSettings: settings.libreHardwareMonitorImport
          ? {
              ...settings.libreHardwareMonitorImport,
              [key]: value,
            }
          : null,
      }),
    );
    // 設定変更時は接続状態をリセット
    setConnectionStatus("idle");
  };

  const testConnection = async () => {
    setTestingConnection(true);
    setConnectionStatus("idle");

    try {
      // await commands.testLibreHardwareMonitorConnection(); TODO
      setConnectionStatus("success");
    } catch (error) {
      console.error("Connection test failed:", error);
      setConnectionStatus("error");
    } finally {
      setTestingConnection(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center space-x-4 py-3">
        <div className="w-full rounded-lg border border-zinc-800 p-4 dark:border-gray-100">
          <div className="flex flex-row items-center justify-between">
            <div className="space-y-0.5">
              <Label
                htmlFor={toggleLibreHardwareMonitorImportId}
                className="flex items-center text-lg"
              >
                {settings.libreHardwareMonitorImport?.enabled ? (
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
                {t("pages.settings.advanced.lhm.title")}
              </Label>
              <p className="text-muted-foreground text-sm">
                {t("pages.settings.advanced.lhm.description")}
              </p>
            </div>

            <Switch
              id={toggleLibreHardwareMonitorImportId}
              checked={settings.libreHardwareMonitorImport?.enabled}
              onCheckedChange={toggleImport}
            />
          </div>

          {settings.libreHardwareMonitorImport?.enabled && (
            <Accordion
              type="single"
              collapsible
              className="w-full"
              defaultValue={
                defaultOpen ? "libreHardwareMonitorSettings" : undefined
              }
            >
              <AccordionItem value="libreHardwareMonitorSettings">
                <AccordionTrigger>
                  {t("pages.settings.advanced.lhm.connectionSettings")}
                </AccordionTrigger>
                <AccordionContent>
                  <div className="flex flex-col space-y-4 p-4">
                    <div className="grid grid-cols-2 gap-4">
                      <div className="space-y-2">
                        <Label htmlFor={hostInputId}>
                          {t("pages.settings.advanced.lhm.host")}
                        </Label>
                        <Input
                          id={hostInputId}
                          type="text"
                          placeholder={t(
                            "pages.settings.advanced.lhm.hostPlaceholder",
                          )}
                          value={settings.libreHardwareMonitorImport.host}
                          onChange={(e) =>
                            updateImportSetting("host", e.target.value)
                          }
                        />
                      </div>
                      <div className="space-y-2">
                        <Label htmlFor={portInputId}>
                          {t("pages.settings.advanced.lhm.port")}
                        </Label>
                        <Input
                          id={portInputId}
                          type="number"
                          placeholder={t(
                            "pages.settings.advanced.lhm.portPlaceholder",
                          )}
                          value={settings.libreHardwareMonitorImport.port}
                          onChange={(e) =>
                            updateImportSetting("port", Number(e.target.value))
                          }
                          min={1}
                          max={65535}
                        />
                      </div>
                    </div>

                    <div className="flex items-center space-x-2">
                      <Checkbox
                        id={httpsCheckboxId}
                        checked={settings.libreHardwareMonitorImport.useHttps}
                        onCheckedChange={(checked) =>
                          updateImportSetting("useHttps", !!checked)
                        }
                      />
                      <Label htmlFor={httpsCheckboxId}>
                        {t("pages.settings.advanced.lhm.useHttps")}
                      </Label>
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                      <div className="space-y-2">
                        <Label htmlFor={refreshIntervalId}>
                          {t("pages.settings.advanced.lhm.refreshInterval")}
                        </Label>
                        <Input
                          id={refreshIntervalId}
                          type="number"
                          placeholder={t(
                            "pages.settings.advanced.lhm.refreshIntervalPlaceholder",
                          )}
                          value={
                            settings.libreHardwareMonitorImport.refreshInterval
                          }
                          onChange={(e) =>
                            updateImportSetting(
                              "refreshInterval",
                              Number(e.target.value),
                            )
                          }
                          min={1}
                          max={3600}
                        />
                      </div>
                      <div className="space-y-2">
                        <Label htmlFor={timeoutId}>
                          {t("pages.settings.advanced.lhm.timeout")}
                        </Label>
                        <Input
                          id={timeoutId}
                          type="number"
                          placeholder={t(
                            "pages.settings.advanced.lhm.timeoutPlaceholder",
                          )}
                          value={settings.libreHardwareMonitorImport.timeout}
                          onChange={(e) =>
                            updateImportSetting(
                              "timeout",
                              Number(e.target.value),
                            )
                          }
                          min={1}
                          max={60000}
                        />
                      </div>
                    </div>

                    <div className="flex items-center justify-between">
                      <TooltipProvider>
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <div className="flex items-center space-x-2">
                              <Info size={16} />
                              <span className="text-muted-foreground text-sm">
                                {t("pages.settings.advanced.lhm.info")}
                              </span>
                            </div>
                          </TooltipTrigger>
                          <TooltipContent>
                            <p className="max-w-sm whitespace-pre-wrap text-sm">
                              {t("pages.settings.advanced.lhm.infoTooltip")}
                            </p>
                          </TooltipContent>
                        </Tooltip>
                      </TooltipProvider>

                      <div className="flex items-center space-x-2">
                        <Button
                          onClick={testConnection}
                          disabled={
                            testingConnection ||
                            !settings.libreHardwareMonitorImport.enabled
                          }
                          variant="outline"
                          size="sm"
                        >
                          {testingConnection
                            ? t("pages.settings.advanced.lhm.testing")
                            : t("pages.settings.advanced.lhm.connectionTest")}
                        </Button>

                        {connectionStatus === "success" && (
                          <div className="flex items-center space-x-1 text-emerald-600">
                            <CheckCircleIcon size={16} />
                            <span className="text-sm">
                              {t(
                                "pages.settings.advanced.lhm.connectionSuccess",
                              )}
                            </span>
                          </div>
                        )}

                        {connectionStatus === "error" && (
                          <div className="flex items-center space-x-1 text-red-600">
                            <ProhibitInsetIcon size={16} />
                            <span className="text-sm">
                              {t(
                                "pages.settings.advanced.lhm.connectionFailed",
                              )}
                            </span>
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                </AccordionContent>
              </AccordionItem>
            </Accordion>
          )}
        </div>
      </div>
    </div>
  );
};
