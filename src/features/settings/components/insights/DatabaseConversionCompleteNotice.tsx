import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";

const ONE_YEAR_RETENTION_DAYS = 365;

/**
 * Shown once, right after a successful conversion (#2136, decided
 * 2026-09-22): the native database affords a longer Retention Period in
 * the same space, and the user can act on that or keep the current value.
 * Nothing changes unless they choose. `onDismiss` (called on either
 * choice) is expected to persist "shown" as UI-local state, so this never
 * appears again once acted on.
 */
export const DatabaseConversionCompleteNotice = ({
  onDismiss,
}: {
  onDismiss: () => void;
}) => {
  const { t } = useTranslation();
  const { settings, setHardwareArchiveRetentionDays } = useSettingsAtom();
  const retentionDays = settings.hardwareArchive.retentionDays;

  const setToOneYear = async () => {
    // Only dismiss - which the caller persists as "shown, never again" -
    // once the save is confirmed. Dismissing on a failed save would lose
    // the user's "set to one year" choice for good: the notice never
    // reappears to offer it again, and `settings.json` keeps whatever
    // value it already had.
    const saved = await setHardwareArchiveRetentionDays(
      ONE_YEAR_RETENTION_DAYS,
    );
    if (saved) {
      onDismiss();
    }
  };

  return (
    <div className="mt-4 rounded-md border p-4">
      <h5 className="font-semibold">
        {t("pages.settings.insights.databaseConversion.notice.title")}
      </h5>
      <p className="mt-2 whitespace-pre-wrap text-sm">
        {t("pages.settings.insights.databaseConversion.notice.description", {
          days: retentionDays,
        })}
      </p>
      <div className="mt-3 flex gap-2">
        <Button type="button" onClick={setToOneYear}>
          {t("pages.settings.insights.databaseConversion.notice.setToOneYear")}
        </Button>
        <Button type="button" variant="outline" onClick={onDismiss}>
          {t("pages.settings.insights.databaseConversion.notice.keep")}
        </Button>
      </div>
    </div>
  );
};
