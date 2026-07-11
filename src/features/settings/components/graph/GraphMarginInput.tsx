import { useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { cn } from "@/lib/utils";

/** Keep in sync with MAX_GRAPH_MARGIN_PX in settings_service.rs */
const MAX_GRAPH_MARGIN_PX = 200;

export const GraphMarginInput = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t } = useTranslation();
  const inputId = useId();
  const disabled = !settings.graphFitToWindow;
  const [marginInput, setMarginInput] = useState(() =>
    String(settings.graphMarginPx),
  );

  useEffect(() => {
    setMarginInput(String(settings.graphMarginPx));
  }, [settings.graphMarginPx]);

  const parseMarginPx = (value: string) => {
    const trimmed = value.trim();
    if (!trimmed) {
      return null;
    }

    const parsed = Number(trimmed);
    if (!Number.isFinite(parsed) || parsed < 0) {
      return null;
    }

    return Math.min(Math.trunc(parsed), MAX_GRAPH_MARGIN_PX);
  };

  const commitMarginInput = async () => {
    const nextValue = parseMarginPx(marginInput);
    if (nextValue === null || nextValue === settings.graphMarginPx) {
      setMarginInput(String(settings.graphMarginPx));
      return;
    }

    await updateSettingAtom("graphMarginPx", nextValue);
  };

  return (
    <div className={cn("w-full py-2", disabled && "opacity-50")}>
      <Label htmlFor={inputId} className="mb-2 block text-sm">
        {t("pages.settings.customTheme.graphStyle.graphMarginPx")}
      </Label>
      <div className="flex items-center">
        <Input
          id={inputId}
          type="number"
          className="max-w-3xs"
          value={marginInput}
          onChange={(e) => setMarginInput(e.target.value)}
          onBlur={() => {
            void commitMarginInput();
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.currentTarget.blur();
            }
          }}
          min={0}
          max={MAX_GRAPH_MARGIN_PX}
          step={1}
          disabled={disabled}
        />
        <span className="ml-2">px</span>
      </div>
    </div>
  );
};
