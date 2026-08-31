import { useTranslation } from "react-i18next";

/**
 * Zone (3): a single honest line naming the lanes the timeline above is
 * still missing, rather than silently omitting them.
 *
 * Fully capability-dependent since #2022: the power lane (#2021) and the
 * fan lane both render wherever the archive carries their readings, so on a
 * machine with neither missing there is nothing left to say and the note
 * disappears entirely rather than claiming a pending sensor that has since
 * arrived.
 *
 * Each flag is only true on evidence (see `claimsPowerUnsupported` and
 * `claimsFanUnsupported`). While an answer is still unknown it reads the
 * same as supported: the note under-claims rather than telling a user with
 * a working sensor that their machine has none.
 */
export const UnsupportedSensorNote = ({
  powerUnsupported,
  fanUnsupported,
}: {
  powerUnsupported: boolean;
  fanUnsupported: boolean;
}) => {
  const { t } = useTranslation();

  if (!powerUnsupported && !fanUnsupported) {
    return null;
  }

  const messageKey =
    powerUnsupported && fanUnsupported
      ? "pages.insights.cooling.unsupportedSensorsNote"
      : powerUnsupported
        ? "pages.insights.cooling.unsupportedSensorsNotePowerOnly"
        : "pages.insights.cooling.unsupportedSensorsNoteFanOnly";

  return (
    <p
      className="px-1 text-muted-foreground text-xs"
      data-testid="cooling-unsupported-sensor-note"
    >
      {t(messageKey)}
    </p>
  );
};
