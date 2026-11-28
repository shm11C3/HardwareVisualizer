import { describe, expect, it } from "vitest";
import type { ProductAnalyticsClientSettings } from "@/rspc/bindings";
import {
  isEffectivelyGranted,
  needsConsent,
  productAnalyticsLinks,
} from "./productAnalytics";

const settings = (
  consent: ProductAnalyticsClientSettings["consent"],
  consentVersion: number,
  currentConsentVersion = 2,
): ProductAnalyticsClientSettings => ({
  consent,
  consentVersion,
  currentConsentVersion,
});

describe("Analytics Consent predicates", () => {
  it("counts a grant as effective only at the current contract version", () => {
    expect(isEffectivelyGranted(settings("granted", 2))).toBe(true);
    expect(isEffectivelyGranted(settings("granted", 1))).toBe(false);
    expect(isEffectivelyGranted(settings("declined", 2))).toBe(false);
    expect(isEffectivelyGranted(settings("undecided", 0))).toBe(false);
  });

  it("asks again after a contract change only for a granted installation", () => {
    expect(needsConsent(settings("undecided", 0))).toBe(true);
    expect(needsConsent(settings("granted", 1))).toBe(true);
    expect(needsConsent(settings("granted", 2))).toBe(false);
    // A declined installation shares nothing under any contract, so it is
    // never asked again.
    expect(needsConsent(settings("declined", 0))).toBe(false);
    expect(needsConsent(settings("declined", 1))).toBe(false);
  });
});

describe("productAnalyticsLinks", () => {
  it("points the data contract at the English section anchor", () => {
    const links = productAnalyticsLinks("en");
    expect(links.privacyNotice).toMatch(/\/docs\/user\/product-analytics\.md$/);
    expect(links.dataContract).toBe(
      `${links.privacyNotice}#what-is-sent-contract-version-1`,
    );
  });

  it("uses the Japanese page without an anchor for ja locales", () => {
    const links = productAnalyticsLinks("ja-JP");
    expect(links.privacyNotice).toMatch(/product-analytics\.ja\.md$/);
    expect(links.dataContract).toBe(links.privacyNotice);
  });

  it("falls back to English for other or unknown languages", () => {
    expect(productAnalyticsLinks("ru").privacyNotice).toMatch(
      /product-analytics\.md$/,
    );
    expect(productAnalyticsLinks(undefined).privacyNotice).toMatch(
      /product-analytics\.md$/,
    );
  });
});
