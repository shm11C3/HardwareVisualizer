import { useTranslation } from "react-i18next";
import { groupSensorNotices, type SensorNotice } from "../utils/sensorNotice";

/** Zone (3): explain omitted power/fan lanes using the cause we can prove. */
export const SensorStatusNote = ({
  powerNotice,
  fanNotice,
}: {
  powerNotice: SensorNotice | null;
  fanNotice: SensorNotice | null;
}) => {
  const { t } = useTranslation();
  const groups = groupSensorNotices(powerNotice, fanNotice);

  if (groups.length === 0) {
    return null;
  }

  return (
    <div
      className="space-y-1 px-1 text-muted-foreground text-xs"
      data-testid="cooling-sensor-status-note"
    >
      {groups.map(({ notice, scope }) => (
        <p key={`${notice}-${scope}`}>
          {t(`pages.insights.cooling.sensorStatusNote.${notice}.${scope}`)}
        </p>
      ))}
    </div>
  );
};
