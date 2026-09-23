import { useTranslation } from "react-i18next";
import { useDatabaseConversion } from "@/features/settings/hooks/useDatabaseConversion";
import { DatabaseConversionStateBody } from "./DatabaseConversionStateBody";

/**
 * Explicit entry point for the native DuckDB database conversion (#2136).
 * Renders nothing when this build does not include the feature
 * (`state.kind === "notSupported"`), so it disappears entirely instead of
 * showing a control nothing behind it can act on.
 *
 * The permanent entry point: the app-root prompt dialog
 * (`DatabaseConversionPromptDialog`) offers the same flow once, right after
 * an update, but this section is what stays reachable afterward. Both share
 * `DatabaseConversionStateBody` for the state-driven content rather than
 * forking the rendering logic.
 */
export const DatabaseConversionSettings = () => {
  const { t } = useTranslation();
  const conversion = useDatabaseConversion();

  if (conversion.state.kind === "notSupported") {
    return null;
  }

  return (
    <div className="py-4">
      <h4 className="font-bold text-xl">
        {t("pages.settings.insights.databaseConversion.title")}
      </h4>
      <p className="mt-2 whitespace-pre-wrap text-sm">
        {t("pages.settings.insights.databaseConversion.description")}
      </p>

      <DatabaseConversionStateBody {...conversion} />
    </div>
  );
};
