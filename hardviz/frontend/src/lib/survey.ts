import type { Platform } from "@tauri-apps/plugin-os";

const SURVEY_EN_URL = "https://hardviz.com/survey/";
const SURVEY_JA_URL = "https://hardviz.com/ja/survey/";

/**
 * The anonymous survey page on the website. The site forwards these query
 * params into the survey form's hidden fields so responses can be grouped by
 * entry point, release, and OS; the page language fills in `locale`.
 * The site has English and Japanese pages only, so other languages open the
 * English page.
 */
export const surveyUrl = ({
  language,
  appVersion,
  os,
}: {
  language?: string | null;
  appVersion: string;
  os: Platform;
}) => {
  const url = new URL(
    language?.toLowerCase().startsWith("ja") ? SURVEY_JA_URL : SURVEY_EN_URL,
  );
  url.searchParams.set("source", "app");
  url.searchParams.set("app_version", appVersion);
  url.searchParams.set("os", os);
  return url.toString();
};
