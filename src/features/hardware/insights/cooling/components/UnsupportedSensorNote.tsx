import { useTranslation } from "react-i18next";

/**
 * Zone (3): a single honest line naming the lanes the timeline above is
 * still missing, rather than silently omitting them.
 *
 * The line is capability-dependent, not a fixed string: since #2021 the
 * power lane renders wherever the archive carries CPU package power, so on
 * those machines the note must stop claiming power is unavailable.
 *
 * `powerUnsupported` is only true on evidence (see
 * `claimsPowerUnsupported`). While the answer is still unknown the note
 * names the fan alone - true regardless of what the fetch returns - rather
 * than asserting an absence it cannot yet see.
 */
export const UnsupportedSensorNote = ({
  powerUnsupported,
}: {
  powerUnsupported: boolean;
}) => {
  const { t } = useTranslation();

  return (
    <p
      className="px-1 text-muted-foreground text-xs"
      data-testid="cooling-unsupported-sensor-note"
    >
      {t(
        powerUnsupported
          ? "pages.insights.cooling.unsupportedSensorsNote"
          : "pages.insights.cooling.unsupportedSensorsNoteFanOnly",
      )}
    </p>
  );
};
