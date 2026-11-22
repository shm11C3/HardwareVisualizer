import { useTranslation } from "react-i18next";
import { useDatabaseConversion } from "@/features/settings/hooks/useDatabaseConversion";
import { useDatabaseConversionNoticeShown } from "@/features/settings/hooks/useDatabaseConversionNoticeShown";
import { useDatabaseConversionSettingsFocus } from "@/hooks/navigation/useDatabaseConversionSettings";
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

  const converted = conversion.state.kind === "nativeAuthoritative";
  const hidden =
    conversion.state.kind === "notSupported" ||
    (converted &&
      !showsDatabaseConversionNotice(
        conversion.state,
        conversion.justCompleted,
        noticeShown,
        noticeShownPending,
      ));
  // A native-only feature (e.g. Process Insight GPU usage) may send the user
  // here; the request waits until the section renders.
  const sectionRef = useDatabaseConversionSettingsFocus(!hidden);

  if (hidden) {
    return null;
  }

  return (
    <div ref={sectionRef} tabIndex={-1} className="scroll-mt-6 py-4">
      <h4 className="text-xl font-bold">
        {t("pages.settings.insights.databaseConversion.title")}
      </h4>
      {/* The description offers the conversion, so it goes once it is done. */}
      {!converted && (
        <p className="mt-2 text-sm whitespace-pre-wrap">
          {t("pages.settings.insights.databaseConversion.description")}
        </p>
      )}

      <DatabaseConversionStateBody {...conversion} />
    </div>
  );
};
