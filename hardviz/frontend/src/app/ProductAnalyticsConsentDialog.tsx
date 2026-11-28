import { ExternalLinkIcon } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { useTauriDialog } from "@/hooks/tauri/useTauriDialog";
import { openURL } from "@/lib/openUrl";
import {
  needsConsent,
  productAnalyticsLinkKeys,
  productAnalyticsLinks,
} from "@/lib/productAnalytics";
import {
  commands,
  type ProductAnalyticsClientSettings,
  type ProductAnalyticsDecision,
} from "@/rspc/bindings";
import type { SelectedDisplayType } from "@/types/ui";

const SENT_ITEMS = [
  "usage",
  "features",
  "hardware",
  "build",
  "tokens",
] as const;

type ProductAnalyticsConsentDialogProps = {
  /** Every higher-priority startup flow is neither requested nor pending. */
  arbitrationFree: boolean;
  displayTarget: SelectedDisplayType | null;
  productAnalytics: ProductAnalyticsClientSettings;
  /** Another startup AlertDialog is open; wait so modals never stack. */
  deferred?: boolean;
  onRequestChange?: (requested: boolean) => void;
};

const reportPromptOpen = (open: boolean) =>
  commands.setProductAnalyticsPromptOpen(open).catch((err: unknown) => {
    console.error("Failed to report the consent dialog state:", err);
  });

/**
 * Asks for Analytics Consent once, at the lowest startup priority (ADR 0028).
 *
 * Whether it opens is decided once per launch, the first time the arbitration
 * is free: it opens then when consent is needed and the main window is not on
 * Settings, otherwise it is skipped for this launch and nothing is written. It
 * never re-evaluates later in the launch. Closing without choosing counts as
 * "Don't share", and both buttons carry the same weight. A failed write fails
 * closed in both directions: nothing is shared.
 */
export const ProductAnalyticsConsentDialog = ({
  arbitrationFree,
  displayTarget,
  productAnalytics,
  deferred = false,
  onRequestChange,
}: ProductAnalyticsConsentDialogProps) => {
  const { t, i18n } = useTranslation();
  const { error } = useTauriDialog();
  const { setProductAnalyticsConsentAtom } = useSettingsAtom();
  const [offer, setOffer] = useState<"undecided" | "offer" | "skip">(
    "undecided",
  );
  const [closed, setClosed] = useState(false);
  const [saving, setSaving] = useState(false);
  const titleRef = useRef<HTMLHeadingElement>(null);

  useLayoutEffect(() => {
    if (!arbitrationFree || offer !== "undecided") return;
    setOffer(
      needsConsent(productAnalytics) && displayTarget !== "settings"
        ? "offer"
        : "skip",
    );
  }, [arbitrationFree, offer, productAnalytics, displayTarget]);

  const requested = offer === "offer" && !closed;
  useLayoutEffect(() => {
    onRequestChange?.(requested);
  }, [requested, onRequestChange]);

  // While it shows unanswered, the backend treats quitting the app as
  // closing it without a choice, which declines.
  const showing = requested && !deferred;
  useEffect(() => {
    if (!showing) return;
    void reportPromptOpen(true);
    return () => {
      void reportPromptOpen(false);
    };
  }, [showing]);

  if (!showing) return null;

  const choose = async (decision: ProductAnalyticsDecision) => {
    if (saving) return;
    setSaving(true);
    // Answered: a quit during the write must not turn a grant into a decline.
    await reportPromptOpen(false);
    try {
      if (!(await setProductAnalyticsConsentAtom(decision))) {
        await error(t("productAnalyticsConsent.errors.save"));
      }
    } catch (err) {
      console.error("Failed to save the analytics consent:", err);
      await error(t("productAnalyticsConsent.errors.save"));
    } finally {
      setSaving(false);
      setClosed(true);
    }
  };

  const links = productAnalyticsLinks(i18n.resolvedLanguage ?? i18n.language);
  const openLink = async (url: string) => {
    try {
      await openURL(url);
    } catch (err) {
      console.error("Failed to open the analytics link:", err);
      await error(t("productAnalyticsConsent.errors.openLink"));
    }
  };

  return (
    <AlertDialog open>
      {/* Scroll inside the dialog so the actions stay reachable in compact
          windows. Focus starts on the title, inside the dialog, so neither
          choice is emphasized. */}
      <AlertDialogContent
        className="max-h-[calc(100dvh-2rem)] overflow-y-auto text-foreground"
        onOpenAutoFocus={(event) => {
          event.preventDefault();
          titleRef.current?.focus();
        }}
        onEscapeKeyDown={(event) => {
          event.preventDefault();
          void choose("declined");
        }}
      >
        <AlertDialogHeader>
          <AlertDialogTitle
            ref={titleRef}
            tabIndex={-1}
            className="outline-none"
          >
            {t("productAnalyticsConsent.title")}
          </AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div className="space-y-3 text-left text-sm leading-relaxed text-muted-foreground">
              <p>{t("productAnalyticsConsent.intro")}</p>
              <div className="space-y-1">
                <div className="font-medium text-foreground">
                  {t("productAnalyticsConsent.sentTitle")}
                </div>
                <ul className="list-disc space-y-1 pl-5">
                  {SENT_ITEMS.map((item) => (
                    <li key={item}>
                      {t(`productAnalyticsConsent.sent.${item}`)}
                    </li>
                  ))}
                </ul>
              </div>
              <p>{t("productAnalyticsConsent.neverSent")}</p>
              <div className="flex flex-wrap gap-x-1 gap-y-0">
                {productAnalyticsLinkKeys.map((key) => (
                  <Button
                    key={key}
                    className="h-auto px-1 py-1"
                    onClick={() => void openLink(links[key])}
                    size="sm"
                    type="button"
                    variant="link"
                  >
                    {t(`productAnalyticsConsent.links.${key}`)}
                    <ExternalLinkIcon className="size-3" />
                  </Button>
                ))}
              </div>
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter className="gap-2">
          <Button
            disabled={saving}
            onClick={() => void choose("declined")}
            type="button"
            variant="outline"
          >
            {t("productAnalyticsConsent.actions.decline")}
          </Button>
          <Button
            disabled={saving}
            onClick={() => void choose("granted")}
            type="button"
            variant="outline"
          >
            {t("productAnalyticsConsent.actions.share")}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
};
