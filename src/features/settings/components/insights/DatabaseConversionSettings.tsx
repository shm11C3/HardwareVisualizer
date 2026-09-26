import { useTranslation } from "react-i18next";
import { useDatabaseConversion } from "@/features/settings/hooks/useDatabaseConversion";
import { useDatabaseConversionNoticeShown } from "@/features/settings/hooks/useDatabaseConversionNoticeShown";
import {
  DatabaseConversionStateBody,
  showsDatabaseConversionNotice,
} from "./DatabaseConversionStateBody";

/**
 * Explicit entry point for the native DuckDB database conversion (#2136).
 * Renders nothing when this build does not include the feature
 * (`state.kind === "notSupported"`), so it disappears entirely instead of
 * showing a control nothing behind it can act on.
 *
 * The app-root prompt dialog (`DatabaseConversionPromptDialog`) offers the
 * same flow once, right after an update; this section is what stays
 * reachable until the conversion is done. Both share
 * `DatabaseConversionStateBody` for the state-driven content rather than
 * forking the rendering logic.
 *
 * The conversion is a one-time operation with nothing to configure
 * afterwards, so once the database is native this section disappears too.
 * The only thing shown after completion is the one-time notice, right after
 * the conversion finishes, and it is styled as such.
 */
export const DatabaseConversionSettings = () => {
  const { t } = useTranslation();
  const conversion = useDatabaseConversion();
  const [noticeShown, , noticeShownPending] =
    useDatabaseConversionNoticeShown();

  if (conversion.state.kind === "notSupported") {
    return null;
  }

  const converted = conversion.state.kind === "nativeAuthoritative";
  if (
    converted &&
    !showsDatabaseConversionNotice(
      conversion.state,
      conversion.justCompleted,
      noticeShown,
      noticeShownPending,
    )
  ) {
    return null;
  }

  return (
    <div className="py-4">
      <h4 className="font-bold text-xl">
        {t("pages.settings.insights.databaseConversion.title")}
      </h4>
      {/* The description offers the conversion, so it goes once it is done. */}
      {!converted && (
        <p className="mt-2 whitespace-pre-wrap text-sm">
          {t("pages.settings.insights.databaseConversion.description")}
        </p>
      )}

      <DatabaseConversionStateBody {...conversion} />
    </div>
  );
};
