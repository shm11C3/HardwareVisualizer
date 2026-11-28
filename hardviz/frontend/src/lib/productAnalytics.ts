import type { ProductAnalyticsClientSettings } from "@/rspc/bindings";

/**
 * Analytics Consent predicates shared by the consent dialog, the Settings
 * section, and the event hook (ADR 0028). The backend enforces consent on every
 * event too; these only decide what the UI shows and which IPC calls to skip.
 */

/** Consent is granted for the contract version this build ships. */
export const isEffectivelyGranted = ({
  consent,
  consentVersion,
  currentConsentVersion,
}: ProductAnalyticsClientSettings) =>
  consent === "granted" && consentVersion >= currentConsentVersion;

/**
 * The consent dialog's condition. A `declined` installation is never asked
 * again, whatever its version: a contract expansion widens what a granted user
 * shares, not what a declined user shares.
 */
export const needsConsent = ({
  consent,
  consentVersion,
  currentConsentVersion,
}: ProductAnalyticsClientSettings) =>
  consent === "undecided" ||
  (consent === "granted" && consentVersion < currentConsentVersion);

const REPO_URL = "https://github.com/shm11c3/HardwareVisualizer";
const DOC_BASE = `${REPO_URL}/blob/develop/docs/user`;

export type ProductAnalyticsLinkKey =
  | "dataContract"
  | "privacyNotice"
  | "clientSource"
  | "serviceSource";

/** Link order is the display order. */
export const productAnalyticsLinkKeys: readonly ProductAnalyticsLinkKey[] = [
  "dataContract",
  "privacyNotice",
  "clientSource",
  "serviceSource",
];

export const productAnalyticsLinks = (
  language?: string | null,
): Record<ProductAnalyticsLinkKey, string> => {
  const ja = language?.toLowerCase().startsWith("ja") ?? false;
  const page = `${DOC_BASE}/${ja ? "product-analytics.ja.md" : "product-analytics.md"}`;

  return {
    privacyNotice: page,
    dataContract: ja ? page : `${page}#what-is-sent-contract-version-1`,
    clientSource: `${REPO_URL}/tree/develop/hardviz/app/src/analytics`,
    serviceSource: "https://github.com/shm11c3/hardviz-product-analytics",
  };
};
