import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { type AmbientSensorCandidate, commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

/** How often the list refreshes while the settings screen is open. */
const POLL_INTERVAL_MS = 5_000;

/**
 * Choose which SwitchBot device the ambient source reads (#2062).
 *
 * A capture in one room found four SwitchBot devices reading between
 * 25.2 °C and 27.3 °C — a spread wider than the rise Cooling Insight
 * treats as a sustained observation. Which one is used therefore changes
 * the analysis, so the app does not pick: it shows what the radio is
 * hearing, with each device's current reading, and waits.
 *
 * The reading is the identifying detail rather than a model name.
 * Model identity cannot be trusted from these broadcasts, and the
 * temperature is what actually tells the owner which device sits near
 * the intake — the placement the guidance above asks for.
 */
export const AmbientSensorPicker = ({
  selectedDeviceId,
  onSelect,
}: {
  selectedDeviceId: string | null;
  onSelect: (deviceId: string) => Promise<boolean>;
}) => {
  const { t } = useTranslation();
  const [candidates, setCandidates] = useState<AmbientSensorCandidate[] | null>(
    null,
  );

  const refresh = useCallback(async () => {
    const result = await commands.getAmbientSensorCandidates();
    if (isError(result)) {
      // Logged rather than raised: the list refreshes on a timer, and a
      // dialog every few seconds would be worse than a stale list.
      console.error(result.error);
      return;
    }
    setCandidates(result.data);
  }, []);

  useEffect(() => {
    void refresh();
    const id = setInterval(() => void refresh(), POLL_INTERVAL_MS);
    return () => clearInterval(id);
  }, [refresh]);

  const handleSelect = async (deviceId: string) => {
    if (!(await onSelect(deviceId))) {
      return;
    }
    void refresh();
  };

  // Null means the first poll has not returned; an empty array means the
  // radio genuinely heard nothing. Both show the same line, because from
  // the user's side they are the same situation — nothing to choose yet.
  if (!candidates?.length) {
    return (
      <p className="text-muted-foreground text-sm">
        {t("pages.settings.insights.ambientSensor.picker.searching")}
      </p>
    );
  }

  return (
    <div className="space-y-2">
      <Label className="text-sm">
        {t("pages.settings.insights.ambientSensor.picker.title")}
      </Label>
      <RadioGroup
        value={selectedDeviceId ?? ""}
        onValueChange={(value) => void handleSelect(value)}
      >
        {candidates.map((candidate) => (
          <div key={candidate.deviceId} className="flex items-center gap-2">
            <RadioGroupItem
              value={candidate.deviceId}
              id={`ambient-${candidate.deviceId}`}
            />
            <Label
              htmlFor={`ambient-${candidate.deviceId}`}
              className="font-normal text-sm"
            >
              {t("pages.settings.insights.ambientSensor.picker.device", {
                shortId: candidate.shortId,
                temperature: candidate.temperatureCelsius.toFixed(1),
                humidity:
                  candidate.humidityPercent == null
                    ? "—"
                    : candidate.humidityPercent.toFixed(0),
              })}
            </Label>
          </div>
        ))}
      </RadioGroup>
    </div>
  );
};
