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
import { useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { LineChartIcon } from "@/components/icons/LineChartIcon";
import { BurnInShift } from "@/components/shared/BurnInShift";
import { NeedRestart } from "@/components/shared/System";
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
import { gpuTempAtom } from "@/features/hardware/store/chart";
import {
  type ChartDataType,
  chartHardwareTypes,
} from "@/features/hardware/types/hardwareDataType";
import { LicensePage } from "@/features/settings/components/LicensePage";
import { PreviewChart } from "@/features/settings/components/Preview";
import { BackgroundImageList } from "@/features/settings/components/SelectBackgroundImage";
import { UploadImage } from "@/features/settings/components/UploadImage";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { Settings as SettingTypes } from "@/features/settings/types/settingsType";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { RGB2HEX } from "@/lib/color";
import { openURL } from "@/lib/openUrl";
import {
  type BurnInShiftMode,
  type BurnInShiftOptions,
  type BurnInShiftPreset,
  type ClientSettings,
  commands,
  type LineGraphType,
  type Theme,
} from "@/rspc/bindings";
import { settingAtoms } from "@/store/ui";
import { AdvancedSettings } from "./components/AdvancedSettings";

const SettingGraphType = () => {
  const { settings, toggleDisplayTarget } = useSettingsAtom();
  const selectedGraphTypes = settings.displayTargets;

  const cpuId = useId();
  const memoryId = useId();
  const gpuId = useId();

  const toggleGraphType = async (type: ChartDataType) => {
    await toggleDisplayTarget(type);
  };

  return (
    <div className="py-6">
      <div className="flex items-center space-x-2 py-3">
        <Checkbox
          id={cpuId}
          checked={selectedGraphTypes.includes("cpu")}
          onCheckedChange={() => toggleGraphType("cpu")}
        />
        <Label htmlFor={cpuId} className="flex items-center space-x-2 text-lg">
          CPU
        </Label>
      </div>
      <div className="flex items-center space-x-2 py-3">
        <Checkbox
          id={memoryId}
          checked={selectedGraphTypes.includes("memory")}
          onCheckedChange={() => toggleGraphType("memory")}
        />
        <Label
          htmlFor={memoryId}
          className="flex items-center space-x-2 text-lg"
        >
          RAM
        </Label>
      </div>
      <div className="flex items-center space-x-2 py-3">
        <Checkbox
          id={gpuId}
          checked={selectedGraphTypes.includes("gpu")}
          onCheckedChange={() => toggleGraphType("gpu")}
        />
        <Label htmlFor={gpuId} className="flex items-center space-x-2 text-lg">
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

  const toggleDarkMode = async (mode: Theme) => {
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
          <SelectItem
            value="system"
            className="focus:bg-gray-200 dark:focus:bg-gray-400"
          >
            {t("pages.settings.general.colorMode.system")}
          </SelectItem>
          <SelectItem
            value="light"
            className="focus:bg-gray-200 dark:focus:bg-gray-400"
          >
            {t("pages.settings.general.colorMode.light")}
          </SelectItem>
          <SelectItem
            value="dark"
            className="focus:bg-gray-400 dark:focus:bg-gray-700"
          >
            {t("pages.settings.general.colorMode.dark")}
          </SelectItem>
          <SelectItem
            value="darkPlus"
            className="focus:bg-black dark:focus:bg-black"
          >
            {t("pages.settings.general.colorMode.darkPlus")}
          </SelectItem>
          <SelectItem
            value="sky"
            className="focus:bg-sky-300 dark:focus:bg-sky-700"
          >
            {t("pages.settings.general.colorMode.sky")}
          </SelectItem>
          <SelectItem
            value="grove"
            className="focus:bg-emerald-300 dark:focus:bg-emerald-700"
          >
            {t("pages.settings.general.colorMode.grove")}
          </SelectItem>
          <SelectItem
            value="sunset"
            className="focus:bg-orange-400 dark:focus:bg-orange-700"
          >
            {t("pages.settings.general.colorMode.sunset")}
          </SelectItem>
          <SelectItem
            value="nebula"
            className="focus:bg-purple-300 dark:focus:bg-purple-900"
          >
            {t("pages.settings.general.colorMode.nebula")}
          </SelectItem>
          <SelectItem
            value="orbit"
            className="focus:bg-slate-300 dark:focus:bg-slate-500"
          >
            {t("pages.settings.general.colorMode.orbit")}
          </SelectItem>
          <SelectItem
            value="cappuccino"
            className="focus:bg-amber-300 dark:focus:bg-amber-500"
          >
            {t("pages.settings.general.colorMode.cappuccino")}
          </SelectItem>
          <SelectItem
            value="espresso"
            className="focus:bg-amber-500 dark:focus:bg-amber-800"
          >
            {t("pages.settings.general.colorMode.espresso")}
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
    updateSettingAtom("backgroundImgOpacity", value[0]);
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

  const settingGraphSwitch = async (value: boolean) => {
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
            onCheckedChange={settingGraphSwitch}
          />
        </div>
      </div>
    </div>
  );
};

const SettingColorInput = ({
  label,
  hardwareType,
}: {
  label: string;
  hardwareType: ChartDataType;
}) => {
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

const SettingBurnInShift = () => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();

  const [defaultOpen, setDefaultOpen] = useState(false);

  const toggleBurnInShiftId = useId();

  const toggleBurnInShift = async (value: boolean) => {
    setDefaultOpen(value);
    await updateSettingAtom("burnInShift", value);
  };
  const selectShiftPreset = async (value: BurnInShiftPreset) => {
    await updateSettingAtom("burnInShiftPreset", value);
  };
  const selectShiftMode = async (value: BurnInShiftMode) => {
    await updateSettingAtom("burnInShiftMode", value);
  };
  const toggleIdleOnly = async (value: boolean) => {
    await updateSettingAtom("burnInShiftIdleOnly", value);
  };

  return (
    <>
      <div className="flex w-full items-center justify-between space-x-4 py-6 xl:w-1/3">
        <div className="space-y-0.5">
          <Label htmlFor={toggleBurnInShiftId} className="text-lg">
            {t("pages.settings.general.burnInShift.name")}
          </Label>
        </div>

        <Switch
          id={toggleBurnInShiftId}
          checked={settings.burnInShift}
          onCheckedChange={toggleBurnInShift}
        />
      </div>
      {settings.burnInShift && (
        <Accordion
          type="single"
          collapsible
          className="w-full xl:w-1/3"
          defaultValue={defaultOpen ? "burnInShiftSettings" : undefined}
        >
          <AccordionItem value="burnInShiftSettings">
            <AccordionTrigger>
              {t("pages.settings.general.burnInShift.detailSettings")}
            </AccordionTrigger>
            <AccordionContent>
              <div className="flex flex-col space-y-2">
                <div className="py-4">
                  <BurnInShiftPresetRadio
                    settings={settings}
                    selectShiftPreset={selectShiftPreset}
                  />
                </div>
                <div className="py-4">
                  <BurnInShiftModeRadio
                    settings={settings}
                    selectShiftMode={selectShiftMode}
                  />
                </div>

                <BurnInShiftIdleOnlyCheckbox
                  settings={settings}
                  toggleIdleOnly={toggleIdleOnly}
                />

                <BurnInShiftOverrideCheckbox />
                {settings.burnInShiftOptions !== null && (
                  <BurnInShiftOptionInputs />
                )}
              </div>
            </AccordionContent>
          </AccordionItem>
        </Accordion>
      )}
    </>
  );
};

const BurnInShiftPresetRadio = ({
  settings,
  selectShiftPreset,
}: {
  settings: ClientSettings;
  selectShiftPreset: (value: BurnInShiftPreset) => Promise<void>;
}) => {
  const { t } = useTranslation();
  const radioBurnInShiftGentle = useId();
  const radioBurnInShiftBalanced = useId();
  const radioBurnInShiftAggressive = useId();

  return (
    <>
      <Label className="text-lg">
        {t("pages.settings.general.burnInShift.preset.name")}
      </Label>
      <RadioGroup
        className="mt-2 flex space-x-2"
        defaultValue={settings.burnInShiftPreset}
        onValueChange={selectShiftPreset}
      >
        <div className="flex items-center space-x-2">
          <RadioGroupItem value="gentle" id={radioBurnInShiftGentle} />
          <Label
            className="flex items-center space-x-2 text-md"
            htmlFor={radioBurnInShiftGentle}
          >
            <span>{t("pages.settings.general.burnInShift.preset.gentle")}</span>
          </Label>
        </div>
        <div className="flex items-center space-x-2">
          <RadioGroupItem value="balanced" id={radioBurnInShiftBalanced} />
          <Label
            className="flex items-center space-x-2 text-md"
            htmlFor={radioBurnInShiftBalanced}
          >
            <span>
              {t("pages.settings.general.burnInShift.preset.balanced")}
            </span>
          </Label>
        </div>
        <div className="flex items-center space-x-2">
          <RadioGroupItem value="aggressive" id={radioBurnInShiftAggressive} />
          <Label
            className="flex items-center space-x-2 text-md"
            htmlFor={radioBurnInShiftAggressive}
          >
            <span>
              {t("pages.settings.general.burnInShift.preset.aggressive")}
            </span>
          </Label>
        </div>
      </RadioGroup>
    </>
  );
};

const BurnInShiftModeRadio = ({
  settings,
  selectShiftMode,
}: {
  settings: ClientSettings;
  selectShiftMode: (value: BurnInShiftMode) => Promise<void>;
}) => {
  const { t } = useTranslation();
  const radioBurnInShiftJump = useId();
  const radioBurnInShiftDrift = useId();
  return (
    <>
      <Label className="text-lg">
        {t("pages.settings.general.burnInShift.mode.name")}
      </Label>
      <RadioGroup
        className="mt-2 flex space-x-2"
        defaultValue={settings.burnInShiftMode}
        onValueChange={selectShiftMode}
      >
        <div className="flex items-center space-x-2">
          <RadioGroupItem value="jump" id={radioBurnInShiftJump} />
          <Label
            className="flex items-center space-x-2 text-md"
            htmlFor={radioBurnInShiftJump}
          >
            <span>{t("pages.settings.general.burnInShift.mode.jump")}</span>
          </Label>
        </div>
        <div className="flex items-center space-x-2">
          <RadioGroupItem value="drift" id={radioBurnInShiftDrift} />
          <Label
            className="flex items-center space-x-2 text-md"
            htmlFor={radioBurnInShiftDrift}
          >
            <span>{t("pages.settings.general.burnInShift.mode.drift")}</span>
          </Label>
        </div>
      </RadioGroup>
    </>
  );
};

const BurnInShiftIdleOnlyCheckbox = ({
  settings,
  toggleIdleOnly,
}: {
  settings: ClientSettings;
  toggleIdleOnly: (value: boolean) => Promise<void>;
}) => {
  const { t } = useTranslation();
  const burnInShiftIdleOnlyId = useId();

  return (
    <div className="flex items-center space-x-2 py-4">
      <Checkbox
        id={burnInShiftIdleOnlyId}
        checked={settings.burnInShiftIdleOnly}
        onCheckedChange={toggleIdleOnly}
      />
      <Label
        htmlFor={burnInShiftIdleOnlyId}
        className="flex items-center space-x-2 text-lg"
      >
        {t("pages.settings.general.burnInShift.idleOnly.name")}
      </Label>
    </div>
  );
};

const BurnInShiftOverrideCheckbox = () => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();
  const id = useId();

  const handleOverrideChange = (checked: boolean) => {
    updateSettingAtom(
      "burnInShiftOptions",
      checked ? ({} as BurnInShiftOptions) : null,
    );
  };

  return (
    <div className="flex items-center space-x-2 py-4">
      <Checkbox
        id={id}
        checked={settings.burnInShiftOptions !== null}
        onCheckedChange={handleOverrideChange}
      />
      <Label htmlFor={id} className="flex items-center space-x-2 text-lg">
        {t("pages.settings.general.burnInShift.override.name")}
      </Label>
    </div>
  );
};

const BurnInShiftOptionInputs = () => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();

  const intervalMsId = useId();
  const amplitudePxIdX = useId();
  const amplitudePxIdY = useId();
  const idleThresholdMsId = useId();
  const driftDurationSecId = useId();

  const inputVariants = "grid w-full max-w-3xs items-center gap-3";

  const updateBurnInShiftOption = async (
    field: keyof BurnInShiftOptions,
    value: number | [number, number] | null,
  ) => {
    const currentOptions = settings.burnInShiftOptions || {};
    const updatedOptions = { ...currentOptions, [field]: value };
    await updateSettingAtom(
      "burnInShiftOptions",
      updatedOptions as BurnInShiftOptions,
    );
  };

  const handleInputChange = (
    field: keyof BurnInShiftOptions,
    inputValue: string,
  ) => {
    if (inputValue === "") {
      updateBurnInShiftOption(field, null);
      return;
    }

    const numValue = Number(inputValue);
    if (!Number.isNaN(numValue)) {
      updateBurnInShiftOption(field, numValue);
    }
  };

  const handleAmplitudeChange = (axis: 0 | 1, inputValue: string) => {
    const currentAmplitude = settings.burnInShiftOptions?.amplitudePx || [0, 0];

    if (inputValue === "") {
      updateBurnInShiftOption("amplitudePx", null);
      return;
    }

    const numValue = Number(inputValue);
    if (!Number.isNaN(numValue)) {
      const newAmplitude: [number, number] = [...currentAmplitude];
      newAmplitude[axis] = numValue;
      updateBurnInShiftOption("amplitudePx", newAmplitude);
    }
  };

  return (
    <div className="flex flex-col space-y-4 py-4">
      <div className={inputVariants}>
        <Label htmlFor={intervalMsId}>
          {t("pages.settings.general.burnInShift.override.options.intervalMs")}
        </Label>
        <Input
          type="number"
          id={intervalMsId}
          placeholder="Not set"
          value={settings.burnInShiftOptions?.intervalMs ?? ""}
          onChange={(e) => handleInputChange("intervalMs", e.target.value)}
        />
      </div>
      <div className="flex space-x-4">
        <div className={inputVariants}>
          <Label htmlFor={amplitudePxIdX}>
            {t(
              "pages.settings.general.burnInShift.override.options.amplitudePxX",
            )}
          </Label>
          <Input
            type="number"
            id={amplitudePxIdX}
            placeholder="Not set"
            value={settings.burnInShiftOptions?.amplitudePx?.[0] ?? ""}
            onChange={(e) => handleAmplitudeChange(0, e.target.value)}
          />
        </div>

        <div className={inputVariants}>
          <Label htmlFor={amplitudePxIdY}>
            {t(
              "pages.settings.general.burnInShift.override.options.amplitudePxY",
            )}
          </Label>
          <Input
            type="number"
            id={amplitudePxIdY}
            placeholder="Not set"
            value={settings.burnInShiftOptions?.amplitudePx?.[1] ?? ""}
            onChange={(e) => handleAmplitudeChange(1, e.target.value)}
          />
        </div>
      </div>
      <div className={inputVariants}>
        <Label htmlFor={idleThresholdMsId}>
          {t(
            "pages.settings.general.burnInShift.override.options.idleThresholdMs",
          )}
        </Label>
        <Input
          type="number"
          id={idleThresholdMsId}
          placeholder="Not set"
          value={settings.burnInShiftOptions?.idleThresholdMs ?? ""}
          onChange={(e) => handleInputChange("idleThresholdMs", e.target.value)}
        />
      </div>
      <div className={inputVariants}>
        <Label htmlFor={driftDurationSecId}>
          {t(
            "pages.settings.general.burnInShift.override.options.driftDurationSec",
          )}
        </Label>
        <Input
          type="number"
          id={driftDurationSecId}
          placeholder="Not set"
          value={settings.burnInShiftOptions?.driftDurationSec ?? ""}
          onChange={(e) =>
            handleInputChange("driftDurationSec", e.target.value)
          }
        />
      </div>
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

const About = ({ onShowLicense }: { onShowLicense: () => void }) => {
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
          variant="secondary"
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
          variant="secondary"
          className="rounded-full text-sm"
        >
          <span className="px-1">
            {t("pages.settings.about.checkLatestVersion")}
          </span>
          <ArrowSquareOutIcon size={16} />
        </Button>
        <Button
          onClick={onShowLicense}
          variant="secondary"
          className="rounded-full text-sm"
        >
          {t("pages.settings.about.license")}
        </Button>
      </div>
    </div>
  );
};

export const Settings = () => {
  const { t } = useTranslation();
  const [showLicensePage, setShowLicensePage] = useState(false);
  const { settings } = useSettingsAtom();

  if (showLicensePage) {
    return <LicensePage onBack={() => setShowLicensePage(false)} />;
  }

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
          <SettingBurnInShift />
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
            <BurnInShift enabled={settings.burnInShift}>
              <PreviewChart />
            </BurnInShift>
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

      <AdvancedSettings />

      <div className="p-4">
        <h3 className="py-3 font-bold text-2xl">
          {t("pages.settings.about.name")}
        </h3>
        <About onShowLicense={() => setShowLicensePage(true)} />
      </div>
    </>
  );
};
