import { useAtomValue } from "jotai";
import { useTranslation } from "react-i18next";
import { groupSensorNotices } from "@/features/hardware/insights/cooling/utils/sensorNotice";
import {
  coolingFanNoticeAtom,
  coolingPowerNoticeAtom,
} from "@/features/hardware/store/coolingSensorNotices";

/** Zone (3): explain omitted power/fan lanes using the cause we can prove. */
export const SensorStatusNote = () => {
  const { t } = useTranslation();
  const powerNotice = useAtomValue(coolingPowerNoticeAtom);
  const fanNotice = useAtomValue(coolingFanNoticeAtom);
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
