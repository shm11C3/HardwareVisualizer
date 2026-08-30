import { useTranslation } from "react-i18next";

/**
 * Zone (3): a single honest line naming the lanes the timeline above is
 * still missing, rather than silently omitting them.
 *
 * The line is capability-dependent, not a fixed string: since #2021 the
 * power lane renders wherever the archive carries CPU package power, so on
 * those machines the note must stop claiming power is unavailable. Where
 * no power was recorded it keeps naming power alongside fan speed, because
 * from the reader's side those two absences are the same thing.
 */
export const UnsupportedSensorNote = ({
  powerSupported,
}: {
  powerSupported: boolean;
}) => {
  const { t } = useTranslation();

  return (
    <p
      className="px-1 text-muted-foreground text-xs"
      data-testid="cooling-unsupported-sensor-note"
    >
      {t(
        powerSupported
          ? "pages.insights.cooling.unsupportedSensorsNoteFanOnly"
          : "pages.insights.cooling.unsupportedSensorsNote",
      )}
    </p>
  );
};
