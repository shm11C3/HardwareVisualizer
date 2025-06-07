import { LineChartIcon } from "@/components/icons/LineChartIcon";
import { NeedRestart } from "@/components/shared/System";
import { gpuTempAtom } from "@/features/hardware/store/chart";
import { PreviewChart } from "@/features/settings/components/Preview";
import { BackgroundImageList } from "@/features/settings/components/SelectBackgroundImage";
import { UploadImage } from "@/features/settings/components/UploadImage";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { settingAtoms } from "@/store/ui";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { TypographyP } from "@/components/ui/typography";
import { defaultColorRGB, sizeOptions } from "@/features/hardware/consts/chart";
import {
  type ChartDataType,
  chartHardwareTypes,
} from "@/features/hardware/types/hardwareDataType";
import type { Settings as SettingTypes } from "@/features/settings/types/settingsType";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { RGB2HEX } from "@/lib/color";
import { openURL } from "@/lib/openUrl";
import {
  type ClientSettings,
  type LineGraphType,
  commands,
} from "@/rspc/bindings";
import {
  ArrowSquareOutIcon,
  CheckCircleIcon,
  DotOutlineIcon,
  GithubLogoIcon,
  ProhibitInsetIcon,
} from "@phosphor-icons/react";
import { getVersion } from "@tauri-apps/api/app";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { useAtom, useSetAtom } from "jotai";
import { Info } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const SettingGraphType = () => {
  const { settings, toggleDisplayTarget } = useSettingsAtom();
  const selectedGraphTypes = settings.displayTargets;

  const toggleGraphType = async (type: ChartDataType) => {
    await toggleDisplayTarget(type);
  };

  return (
    <div className="py-6">
      <div className="flex items-center space-x-2 py-3">
        <Checkbox
          id="graphType-cpu"
          checked={selectedGraphTypes.includes("cpu")}
          onCheckedChange={() => toggleGraphType("cpu")}
        />
        <Label
          htmlFor="graphType-cpu"
          className="flex items-center space-x-2 text-lg"
        >
          CPU
        </Label>
      </div>
      <div className="flex items-center space-x-2 py-3">
        <Checkbox
          id="graphType-ram"
          checked={selectedGraphTypes.includes("memory")}
          onCheckedChange={() => toggleGraphType("memory")}
        />
        <Label
          htmlFor="graphType-ram"
          className="flex items-center space-x-2 text-lg"
        >
          RAM
        </Label>
      </div>
      <div className="flex items-center space-x-2 py-3">
        <Checkbox
          id="graphType-gpu"
          checked={selectedGraphTypes.includes("gpu")}
          onCheckedChange={() => toggleGraphType("gpu")}
        />
        <Label
          htmlFor="graphType-gpu"
          className="flex items-center space-x-2 text-lg"
        >
          GPU
        </Label>
      </div>
    </div>
  );
};

const SettingLanguage = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t, i18n } = useTranslation();

  const supported = Object.keys(i18n.services.resourceStore.data);
  const displaySupported: Record<string, string> = {
    en: t("lang.en"),
    ja: t("lang.ja"),
  };

  const changeLanguage = async (value: string) => {
    await updateSettingAtom("language", value);
  };

  return (
    <div className="flex w-full items-center justify-between space-x-4 py-6 xl:w-1/3">
      <Label htmlFor="language" className="text-lg">
        {t("pages.settings.general.language")}
      </Label>
      <Select value={settings.language} onValueChange={changeLanguage}>
        <SelectTrigger className="w-[180px]">
          <SelectValue placeholder="Language" />
        </SelectTrigger>
        <SelectContent>
          {supported.map((lang) => (
            <SelectItem key={lang} value={lang}>
              {displaySupported[lang]}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
};

const SettingColorMode = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t } = useTranslation();

  const toggleDarkMode = async (mode: "light" | "dark") => {
    await updateSettingAtom("theme", mode);
  };

  return (
    <div className="flex w-full items-center justify-between space-x-4 py-6 xl:w-1/3">
      <Label htmlFor="darkMode" className="text-lg">
        {t("pages.settings.general.colorMode.name")}
      </Label>
      <Select value={settings.theme} onValueChange={toggleDarkMode}>
        <SelectTrigger className="w-[180px]">
          <SelectValue placeholder="Theme" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="light">
            {t("pages.settings.general.colorMode.light")}
          </SelectItem>
          <SelectItem value="dark">
            {t("pages.settings.general.colorMode.dark")}
          </SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
};

const SettingLineChartSize = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t } = useTranslation();

  const sizeIndex = sizeOptions.indexOf(
    settings.graphSize as SettingTypes["graphSize"],
  );

  const changeGraphSize = async (value: number[]) => {
    await updateSettingAtom("graphSize", sizeOptions[value[0]]);
  };

  return (
    <div className="w-full py-6">
      <Label className="mb-2 block text-lg">
        {t("pages.settings.customTheme.graphStyle.size")}
      </Label>
      <Slider
        min={0}
        max={sizeOptions.length - 1}
        step={1}
        value={[sizeIndex]}
        onValueChange={changeGraphSize}
        className="mt-4 w-full"
      />
      <div className="mt-2 flex items-center justify-between text-sm">
        {sizeOptions.map((size) => (
          <DotOutlineIcon
            key={size}
            className="text-slate-600 dark:text-gray-400"
            size={32}
          />
        ))}
      </div>
    </div>
  );
};

const SettingLineChartType = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t } = useTranslation();

  const changeLineGraphType = async (
    value: ClientSettings["lineGraphType"],
  ) => {
    await updateSettingAtom("lineGraphType", value);
  };

  const lineGraphTypes: LineGraphType[] = [
    "default",
    "step",
    "linear",
    "basis",
  ] as const;

  const lineGraphLabels: Record<LineGraphType, string> = {
    default: "Default",
    step: "Step",
    linear: "Linear",
    basis: "Soft",
  };

  return (
    <div className="py-6">
      <Label htmlFor="lineGraphType" className="mb-2 text-lg">
        {t("pages.settings.customTheme.graphStyle.lineType")}
      </Label>
      <RadioGroup
        className="mt-4 flex items-center space-x-4"
        defaultValue={settings.lineGraphType}
        onValueChange={changeLineGraphType}
      >
        {lineGraphTypes.map((type) => {
          const id = `radio-line-chart-type-${type}`;

          return (
            <div key={type} className="flex items-center space-x-2">
              <RadioGroupItem value={type} id={id} />
              <Label
                className="flex items-center space-x-1 py-2 text-lg"
                htmlFor={id}
              >
                <LineChartIcon type={type} />
                <span>{lineGraphLabels[type]}</span>
              </Label>
            </div>
          );
        })}
      </RadioGroup>
    </div>
  );
};

const SettingBackGroundOpacity = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t } = useTranslation();

  const changeBackGroundOpacity = async (value: number[]) => {
    await updateSettingAtom("backgroundImgOpacity", value[0]);
  };

  return (
    settings.selectedBackgroundImg && (
      <div className="max-w-96 py-3">
        <Label className="mb-2 block text-lg">
          {t("pages.settings.backgroundImage.opacity")}
        </Label>
        <Slider
          min={0}
          max={100}
          step={1}
          value={[settings.backgroundImgOpacity]}
          onValueChange={changeBackGroundOpacity}
          className="mt-4 w-full"
        />
      </div>
    )
  );
};

const SettingGraphSwitch = ({
  type,
}: {
  type: Extract<
    keyof ClientSettings,
    | "lineGraphBorder"
    | "lineGraphFill"
    | "lineGraphMix"
    | "lineGraphShowLegend"
    | "lineGraphShowScale"
    | "lineGraphShowTooltip"
  >;
}) => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t } = useTranslation();

  const SettingGraphSwitch = async (value: boolean) => {
    await updateSettingAtom(type, value);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center space-x-4 py-3">
        <div className="flex w-full flex-row items-center justify-between rounded-lg border border-zinc-800 p-4 dark:border-gray-100">
          <div className="space-y-0.5">
            <Label htmlFor={type} className="text-lg">
              {t(`pages.settings.customTheme.graphStyle.${type}`)}
            </Label>
          </div>

          <Switch
            checked={settings[type]}
            onCheckedChange={SettingGraphSwitch}
          />
        </div>
      </div>
    </div>
  );
};

const SettingColorInput = ({
  label,
  hardwareType,
}: { label: string; hardwareType: ChartDataType }) => {
  const { settings, updateLineGraphColorAtom } = useSettingsAtom();

  const updateGraphColor = async (value: string) => {
    await updateLineGraphColorAtom(hardwareType, value);
  };

  // カンマ区切りのRGB値を16進数に変換
  const hexValue = RGB2HEX(settings.lineGraphColor[hardwareType]);

  return (
    <div className="grid grid-cols-2 gap-4 py-3">
      <Label htmlFor={hardwareType} className="text-lg">
        {label}
      </Label>

      <input
        type="color"
        className="h-6 w-6 cursor-pointer border-none p-0"
        value={hexValue}
        onChange={(e) => updateGraphColor(e.target.value)}
      />
    </div>
  );
};

const SettingColorReset = () => {
  const { updateLineGraphColorAtom } = useSettingsAtom();
  const { t } = useTranslation();

  const updateGraphColor = async () => {
    await Promise.all(
      chartHardwareTypes.map((type) =>
        updateLineGraphColorAtom(type, RGB2HEX(defaultColorRGB[type])),
      ),
    );
  };

  return (
    <Button
      onClick={() => updateGraphColor()}
      className="mt-4"
      variant="secondary"
      size="lg"
    >
      {t("shared.reset")}
    </Button>
  );
};

const SettingTemperatureUnit = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t } = useTranslation();
  const setData = useSetAtom(gpuTempAtom);

  const changeTemperatureUnit = async (
    value: SettingTypes["temperatureUnit"],
  ) => {
    await updateSettingAtom("temperatureUnit", value);
    setData([]);
  };

  return (
    <div className="flex w-full items-center justify-between space-x-4 py-6 xl:w-1/3">
      <Label htmlFor="temperatureUnit" className="text-lg">
        {t("pages.settings.general.temperatureUnit.name")}
      </Label>
      <Select
        value={settings.temperatureUnit}
        onValueChange={changeTemperatureUnit}
      >
        <SelectTrigger className="w-[180px]">
          <SelectValue placeholder="Temperature Unit" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="C">
            {t("pages.settings.general.temperatureUnit.celsius")}
          </SelectItem>
          <SelectItem value="F">
            {t("pages.settings.general.temperatureUnit.fahrenheit")}
          </SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
};

const SettingAutoStart = () => {
  const { t } = useTranslation();
  const [autoStartEnabled, setAutoStartEnabled] = useState<boolean | null>(
    null,
  );
  const { error } = useTauriDialog();

  const toggleAutoStart = async (value: boolean) => {
    setAutoStartEnabled(value);

    try {
      value ? await enable() : await disable();
    } catch (e) {
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

const InsightTitle = () => {
  const { t } = useTranslation();
  return (
    <div className="flex items-center space-x-4 pt-3 pb-1">
      <h3 className="font-bold text-2xl">
        {t("pages.settings.insights.name")}
      </h3>
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Info />
          </TooltipTrigger>
          <TooltipContent>
            <TypographyP className="whitespace-pre-wrap text-sm">
              {t("pages.settings.insights.about.description")}
            </TypographyP>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    </div>
  );
};

const ToggleInsight = () => {
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

const SetNumberOfDaysInsightDataRetains = () => {
  const {
    settings,
    setHardwareArchiveRefreshIntervalDays,
    setScheduledDataDeletion,
  } = useSettingsAtom();
  const { t } = useTranslation();
  const [hasSettingChanged, setHasSettingChanged] = useAtom(
    settingAtoms.isRequiredRestart,
  );

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
        <Label className="my-4 text-lg" htmlFor="holdingPeriod">
          {t("pages.settings.insights.holdingPeriod.title")}
        </Label>
        <div className="flex items-center justify-between">
          <div className="mt-2 flex items-center">
            <Input
              id="holdingPeriod"
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
              id="scheduledDataDeletion"
              checked={settings.hardwareArchive.scheduledDataDeletion}
              onCheckedChange={handleScheduledDataDeletion}
            />
            <Label
              htmlFor="scheduledDataDeletion"
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

const About = () => {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then((v) => setVersion(v));
  }, []);

  return (
    <div className="px-4 py-2">
      <p className="text-gray-500 text-sm">
        {t("pages.settings.about.version", { version })}
      </p>
      <p className="text-gray-500 text-sm">
        {t("pages.settings.about.author", { author: "shm11C3" })}
      </p>
      <div className="flex items-center space-x-4 py-4">
        <Button
          onClick={() =>
            openURL("https://github.com/shm11C3/HardwareVisualizer")
          }
          className="rounded-full text-sm"
        >
          <GithubLogoIcon size={32} />
          <span className="px-1">{t("pages.settings.about.checkGitHub")}</span>
          <ArrowSquareOutIcon size={16} />
        </Button>
        <Button
          onClick={() =>
            openURL(
              "https://github.com/shm11C3/HardwareVisualizer/releases/latest",
            )
          }
          className="rounded-full text-sm"
        >
          <span className="px-1">
            {t("pages.settings.about.checkLatestVersion")}
          </span>
          <ArrowSquareOutIcon size={16} />
        </Button>
        <Button
          onClick={() =>
            openURL(
              "https://github.com/shm11C3/HardwareVisualizer?tab=MIT-1-ov-file#readme",
            )
          }
          className="rounded-full text-sm"
        >
          <span className="px-1">{t("pages.settings.about.license")}</span>
          <ArrowSquareOutIcon size={16} />
        </Button>
      </div>
    </div>
  );
};

/**
 * @todo
 * - [x] About 欄の実装
 * - [x] ハードウェア種別をカスタマイズに移動
 * - [ ] ログを見るボタンの追加
 * - できればやる
 *   - [ ] 縦画面でのプレビューを見やすく（画面外の時はミニプレビューを追従して表示）
 *   - [ ] CPUコアごと分割表示設定の追加
 *   - [ ] アイコンをグラフスタイル欄にも追加する
 *   - [ ] スケールを縦横で分割して数字の表示非表示を選択可能にする
 *   - [ ] 設定リセットボタンの実装
 *   - [ ] 自動スタートの設定
 *   - [x] バージョンの表記
 */
const Settings = () => {
  const { t } = useTranslation();

  return (
    <>
      <div className="mt-8 p-4">
        <h3 className="py-3 font-bold text-2xl">
          {t("pages.settings.general.name")}
        </h3>
        <div className="px-4">
          <SettingLanguage />
          <SettingColorMode />
          <SettingTemperatureUnit />
          <SettingAutoStart />
        </div>
      </div>
      <div className="mt-8 p-4">
        <h3 className="py-3 font-bold text-2xl">
          {t("pages.settings.customTheme.name")}
        </h3>
        <div className="items-start gap-x-12 gap-y-4 p-4 xl:grid xl:grid-cols-6">
          <div className="col-span-2 py-2">
            <h4 className="font-bold text-xl">
              {t("pages.settings.customTheme.graphStyle.name")}
            </h4>
            <SettingGraphSwitch type="lineGraphBorder" />
            <SettingGraphSwitch type="lineGraphFill" />
            <SettingGraphSwitch type="lineGraphMix" />
            <SettingGraphSwitch type="lineGraphShowLegend" />
            <SettingGraphSwitch type="lineGraphShowScale" />
            <SettingGraphSwitch type="lineGraphShowTooltip" />
            <SettingLineChartSize />
            <SettingLineChartType />
          </div>
          <div className="col-span-1 py-2">
            <div className="py-6">
              <h4 className="font-bold text-xl">
                {t("pages.settings.customTheme.lineColor")}
              </h4>
              <div className="py-6 md:flex lg:block">
                <SettingColorInput label="CPU" hardwareType="cpu" />
                <SettingColorInput label="RAM" hardwareType="memory" />
                <SettingColorInput label="GPU" hardwareType="gpu" />
              </div>
              <SettingColorReset />
            </div>
            <div className="py-6">
              <h4 className="font-bold text-xl">
                {t("pages.settings.general.hardwareType")}
              </h4>
              <SettingGraphType />
            </div>
          </div>
          <div className="col-span-3 ml-10 py-2">
            <h4 className="font-bold text-xl">
              {t("pages.settings.customTheme.preview")}
            </h4>
            <PreviewChart />
          </div>
          <div className="order-2 col-span-3 xl:order-none">
            <h4 className="py-3 font-bold text-xl">
              {t("pages.settings.backgroundImage.name")}
            </h4>
            <div className="p-1">
              <div className="py-3">
                <UploadImage />
                <BackgroundImageList />
              </div>
              <SettingBackGroundOpacity />
            </div>
          </div>
        </div>
      </div>
      <div className="items-start gap-x-12 gap-y-4 p-4 xl:grid xl:grid-cols-6">
        <div className="col-span-2">
          <InsightTitle />
          <div className="p-4">
            <ToggleInsight />
            <SetNumberOfDaysInsightDataRetains />
          </div>
        </div>
      </div>

      <div className="p-4">
        <h3 className="py-3 font-bold text-2xl">
          {t("pages.settings.about.name")}
        </h3>
        <About />
      </div>
    </>
  );
};

export default Settings;
