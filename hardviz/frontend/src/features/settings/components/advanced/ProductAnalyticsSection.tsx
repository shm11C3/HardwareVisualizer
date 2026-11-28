import { ChartBarIcon, ExternalLinkIcon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ActionFailure } from "@/components/ActionFailure";
import { SettingWriteFailure } from "@/components/SettingWriteFailure";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { openURL } from "@/lib/openUrl";
import {
  isEffectivelyGranted,
  productAnalyticsLinkKeys,
  productAnalyticsLinks,
} from "@/lib/productAnalytics";

/**
 * The Settings entry for Analytics Consent (ADR 0028): a switch bound to
 * grant/decline, one state line, and the documents behind the claim. It stays
 * available for both outcomes of the startup dialog.
 */
export const ProductAnalyticsSection = () => {
  const { t, i18n } = useTranslation();
  const { settings, setProductAnalyticsConsentAtom } = useSettingsAtom();
  const [saving, setSaving] = useState(false);
  const [linkOpenFailed, setLinkOpenFailed] = useState(false);
  const { productAnalytics } = settings;
  const granted = isEffectivelyGranted(productAnalytics);
  const stale =
    productAnalytics.consent === "granted" &&
    productAnalytics.consentVersion < productAnalytics.currentConsentVersion;
  const links = productAnalyticsLinks(i18n.resolvedLanguage ?? i18n.language);

  const handleChange = async (checked: boolean) => {
    setSaving(true);
    try {
      // A failure is recorded as state by the setter and rendered below.
      await setProductAnalyticsConsentAtom(checked ? "granted" : "declined");
    } catch {
      // Already logged and recorded by the setter; the switch stays as saved.
    } finally {
      setSaving(false);
    }
  };

  const handleOpenLink = async (url: string) => {
    setLinkOpenFailed(false);
    try {
      await openURL(url);
    } catch (err) {
      console.error("Failed to open the analytics link:", err);
      setLinkOpenFailed(true);
    }
  };

  return (
    <div className="w-full py-6 xl:w-1/2">
      <div className="flex w-full items-center justify-between gap-4">
        <div className="flex items-start gap-3">
          <ChartBarIcon className="mt-1 size-5 shrink-0 text-muted-foreground" />
          <div className="space-y-1">
            <Label htmlFor="productAnalytics" className="text-lg">
              {t("pages.settings.advanced.productAnalytics.name")}
            </Label>
            <p className="text-sm text-muted-foreground">
              {t("pages.settings.advanced.productAnalytics.description")}
            </p>
            <p className="text-sm text-muted-foreground">
              {granted
                ? t("pages.settings.advanced.productAnalytics.stateGranted", {
                    version: productAnalytics.consentVersion,
                  })
                : t("pages.settings.advanced.productAnalytics.stateNotSharing")}
            </p>
            {stale && (
              <p className="text-sm text-amber-600 dark:text-amber-400">
                {t("pages.settings.advanced.productAnalytics.staleNote", {
                  current: productAnalytics.currentConsentVersion,
                })}
              </p>
            )}
          </div>
        </div>

        <Switch
          id="productAnalytics"
          checked={granted}
          disabled={saving}
          onCheckedChange={(checked) => void handleChange(checked)}
        />
      </div>
      <div className="mt-2 flex flex-wrap gap-x-1">
        {productAnalyticsLinkKeys.map((key) => (
          <Button
            key={key}
            className="h-auto px-1 py-1"
            onClick={() => void handleOpenLink(links[key])}
            size="sm"
            type="button"
            variant="link"
          >
            {t(`productAnalyticsConsent.links.${key}`)}
            <ExternalLinkIcon className="size-3" />
          </Button>
        ))}
      </div>
      {linkOpenFailed && (
        <ActionFailure
          message={t("pages.settings.advanced.productAnalytics.openLinkError")}
          className="mt-2"
        />
      )}
      <SettingWriteFailure actions="productAnalytics" />
    </div>
  );
};
