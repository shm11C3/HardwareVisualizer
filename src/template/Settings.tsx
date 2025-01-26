import { gpuTempAtom } from "@/atom/chart";
import { useSettingsAtom } from "@/atom/useSettingsAtom";
import { PreviewChart } from "@/components/charts/Preview";
import { BackgroundImageList } from "@/components/forms/SelectBackgroundImage/SelectBackgroundImage";
import { UploadImage } from "@/components/forms/UploadImage/UploadImage";
import { LineChartIcon } from "@/components/icons/LineChartIcon";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { defaultColorRGB, sizeOptions } from "@/consts/chart";
import { RGB2HEX } from "@/lib/color";
import { openURL } from "@/lib/openUrl";
import type { ClientSettings, LineGraphType } from "@/rspc/bindings";
import {
  type ChartDataType,
  chartHardwareTypes,
} from "@/types/hardwareDataType";
import type { Settings as SettingTypes } from "@/types/settingsType";
import { ArrowSquareOut, DotOutline, GithubLogo } from "@phosphor-icons/react";
import { useSetAtom } from "jotai";
import { useTranslation } from "react-i18next";

const SettingGraphType = () => {
  const { settings, toggleDisplayTarget } = useSettingsAtom();
  const { t } = useTranslation();
  const selectedGraphTypes = settings.displayTargets;

  const toggleGraphType = async (type: ChartDataType) => {
    await toggleDisplayTarget(type);
  };

  return (
    <div className="flex items-center space-x-4 py-6">
      <Label htmlFor="graphType" className="text-lg self-start">
        {t("pages.settings.general.hardwareType")}
      </Label>
      <div>
        <label className="flex items-center space-x-2">
          <input
            type="checkbox"
            checked={selectedGraphTypes.includes("cpu")}
            onChange={() => toggleGraphType("cpu")}
          />
          <span>CPU</span>
        </label>
        <label className="flex items-center space-x-2">
          <input
            type="checkbox"
            checked={selectedGraphTypes.includes("memory")}
            onChange={() => toggleGraphType("memory")}
          />
          <span>Memory</span>
        </label>
        <label className="flex items-center space-x-2">
          <input
            type="checkbox"
            checked={selectedGraphTypes.includes("gpu")}
            onChange={() => toggleGraphType("gpu")}
          />
          <span>GPU</span>
        </label>
      </div>
    </div>
  );
};

const SettingLanguage = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t, i18n } = useTranslation();

  const supported = Object.keys(i18n.services.resourceStore.data);
  const displaySupported: Record<string, string> = {
    en: "English",
    ja: "日本語",
  };

  const changeLanguage = async (value: string) => {
    await updateSettingAtom("language", value);
  };

  return (
    <div className="flex items-center space-x-4 py-6">
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
    <div className="flex items-center space-x-4 py-6">
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
    <div className="py-6 w-full">
      <Label className="block text-lg mb-2">
        {t("pages.settings.customTheme.graphStyle.size")}
      </Label>
      <Slider
        min={0}
        max={sizeOptions.length - 1}
        step={1}
        value={[sizeIndex]}
        onValueChange={changeGraphSize}
        className="w-full mt-4"
      />
      <div className="flex justify-between items-center text-sm mt-2">
        {sizeOptions.map((size) => (
          <DotOutline
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
      <Label htmlFor="lineGraphType" className="text-lg mb-2">
        {t("pages.settings.customTheme.graphStyle.lineType")}
      </Label>
      <RadioGroup
        className="flex items-center space-x-4 mt-4"
        defaultValue={settings.lineGraphType}
        onValueChange={changeLineGraphType}
      >
        {lineGraphTypes.map((type) => {
          const id = `radio-line-chart-type-${type}`;

          return (
            <div key={type} className="flex items-center space-x-2">
              <RadioGroupItem value={type} id={id} />
              <Label
                className="text-lg flex items-center space-x-1 py-2"
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
      <>
        <Label className="block text-lg mb-2">
          {t("pages.settings.backgroundImage.opacity")}
        </Label>
        <Slider
          min={0}
          max={100}
          step={1}
          value={[settings.backgroundImgOpacity]}
          onValueChange={changeBackGroundOpacity}
          className="w-full mt-4"
        />
      </>
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
        <div className="w-full flex flex-row items-center justify-between rounded-lg border p-4 border-zinc-800 dark:border-gray-100">
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
    <div className="grid grid-cols-2 gap-4 py-6">
      <Label htmlFor={hardwareType} className="text-lg">
        {label}
      </Label>

      <input
        type="color"
        className="w-6 h-6 border-none p-0 cursor-pointer"
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
    <div className="flex items-center space-x-4 py-6">
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

const About = () => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();

  return (
    <div className="p-4">
      <h3 className="text-2xl font-bold py-3">
        {t("pages.settings.about.name")}
      </h3>
      <div className="py-2">
        <p className="text-sm text-gray-500">
          {t("pages.settings.about.version", { version: settings.version })}
        </p>
        <p className="text-sm text-gray-500">
          {t("pages.settings.about.author", { author: "shm11C3" })}
        </p>
        <div className="py-4 flex items-center space-x-4">
          <Button
            onClick={() =>
              openURL("https://github.com/shm11C3/HardwareVisualizer")
            }
            className="text-sm  rounded-full"
          >
            <GithubLogo size={32} />
            <span className="px-1">
              {t("pages.settings.about.checkGitHub")}
            </span>
            <ArrowSquareOut size={16} />
          </Button>
          <Button
            onClick={() =>
              openURL(
                "https://github.com/shm11C3/HardwareVisualizer/releases/latest",
              )
            }
            className="text-sm  rounded-full"
          >
            <span className="px-1">
              {t("pages.settings.about.checkLatestVersion")}
            </span>
            <ArrowSquareOut size={16} />
          </Button>
          <Button
            onClick={() =>
              openURL(
                "https://github.com/shm11C3/HardwareVisualizer?tab=MIT-1-ov-file#readme",
              )
            }
            className="text-sm rounded-full"
          >
            <span className="px-1">{t("pages.settings.about.license")}</span>
            <ArrowSquareOut size={16} />
          </Button>
        </div>
      </div>
    </div>
  );
};

/**
 * @todo
 * - [x] About 欄の実装
 * - [ ] ハードウェア種別をカスタマイズに移動
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
        <h3 className="text-2xl font-bold py-3">
          {t("pages.settings.general.name")}
        </h3>
        <SettingLanguage />
        <SettingColorMode />
        <SettingTemperatureUnit />
        <SettingGraphType />
      </div>
      <div className="mt-8 p-4">
        <h3 className="text-2xl font-bold py-3 px-2">
          {t("pages.settings.customTheme.name")}
        </h3>
        <div className="xl:grid xl:grid-cols-6 gap-12 p-4 items-start">
          <div className="col-span-2 py-2">
            <h4 className="text-xl font-bold">
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
            <h4 className="text-xl font-bold">
              {t("pages.settings.customTheme.lineColor")}
            </h4>
            <div className="md:flex lg:block">
              <SettingColorInput label="CPU" hardwareType="cpu" />
              <SettingColorInput label="RAM" hardwareType="memory" />
              <SettingColorInput label="GPU" hardwareType="gpu" />
            </div>
            <SettingColorReset />
          </div>
          <div className="col-span-3 py-2 ml-10">
            <h4 className="text-xl font-bold">
              {t("pages.settings.customTheme.preview")}
            </h4>
            <PreviewChart />
          </div>
          <div className="col-span-3 py-6  order-2 xl:order-none">
            <h3 className="text-2xl font-bold py-3">
              {t("pages.settings.backgroundImage.name")}
            </h3>
            <div className="p-4">
              <div className="py-3">
                <UploadImage />
                <BackgroundImageList />
              </div>
              <div className="py-3 max-w-96">
                <SettingBackGroundOpacity />
              </div>
            </div>
          </div>
        </div>
      </div>
      <div className="px-4">
        <About />
      </div>
    </>
  );
};

export default Settings;
