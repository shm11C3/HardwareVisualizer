import { useTranslation } from "react-i18next";

/**
 * Zone (3): a single honest line noting that package power and fan-speed
 * lanes are not implemented yet, rather than silently omitting them.
 */
export const UnsupportedSensorNote = () => {
  const { t } = useTranslation();

  return (
    <p
      className="px-1 text-muted-foreground text-xs"
      data-testid="cooling-unsupported-sensor-note"
    >
      {t("pages.insights.cooling.unsupportedSensorsNote")}
    </p>
  );
};
