import { useLayoutEffect, useState } from "react";
import type { ProductAnalyticsClientSettings } from "@/rspc/bindings";
import type { SelectedDisplayType } from "@/types/ui";
import { CloseToTrayFirstRunDialog } from "./CloseToTrayFirstRunDialog";
import { DatabaseConversionPromptDialog } from "./DatabaseConversionPromptDialog";
import { ExternalComponentGuidanceDialog } from "./ExternalComponentGuidanceDialog";
import { NsisMigrationNoticeDialog } from "./NsisMigrationNoticeDialog";
import { ProductAnalyticsConsentDialog } from "./ProductAnalyticsConsentDialog";

type Dialog = "close" | "guidance" | "conversion" | "nsis" | "consent";

/** App-owned arbitration: a late reading or close request never covers an active flow. */
export const StartupDialogs = ({
  settingsLoaded,
  settingsSettled,
  displayTarget,
  nsisMigrationNoticeDismissed,
  productAnalytics,
}: {
  settingsLoaded: boolean;
  settingsSettled: boolean;
  displayTarget: SelectedDisplayType | null;
  nsisMigrationNoticeDismissed: boolean;
  productAnalytics: ProductAnalyticsClientSettings;
}) => {
  const [active, setActive] = useState<Dialog | null>(null);
  const [closeRequested, setCloseRequested] = useState(false);
  const [guidanceRequested, setGuidanceRequested] = useState(false);
  const [conversionRequested, setConversionRequested] = useState(false);
  const [nsisRequested, setNsisRequested] = useState(false);
  const [consentRequested, setConsentRequested] = useState(false);
  const [guidancePending, setGuidancePending] = useState(true);
  const [conversionPending, setConversionPending] = useState(true);
  const [nsisPending, setNsisPending] = useState(true);

  // Consent is the lowest priority: it may be offered only when no other flow
  // is requested or could still request the slot.
  const arbitrationFree =
    settingsLoaded &&
    !closeRequested &&
    !guidanceRequested &&
    !conversionRequested &&
    !nsisRequested &&
    !guidancePending &&
    !conversionPending &&
    !nsisPending;

  useLayoutEffect(() => {
    const requests = {
      close: closeRequested,
      guidance: guidanceRequested,
      conversion: conversionRequested,
      nsis: nsisRequested,
      consent: consentRequested,
    };
    // Keep the owner until its flow finishes, including conversion progress/retry.
    if (active && requests[active]) return;
    setActive(
      closeRequested
        ? "close"
        : guidanceRequested
          ? "guidance"
          : !guidancePending && conversionRequested
            ? "conversion"
            : !guidancePending && !conversionPending && nsisRequested
              ? "nsis"
              : arbitrationFree && consentRequested
                ? "consent"
                : null,
    );
  }, [
    active,
    closeRequested,
    guidanceRequested,
    conversionRequested,
    nsisRequested,
    consentRequested,
    arbitrationFree,
    guidancePending,
    conversionPending,
  ]);

  return (
    <>
      <CloseToTrayFirstRunDialog
        settingsSettled={settingsSettled}
        deferred={active !== "close"}
        onRequestChange={setCloseRequested}
      />
      <ExternalComponentGuidanceDialog
        settingsLoaded={settingsLoaded}
        displayTarget={displayTarget}
        deferred={active !== "guidance"}
        onRequestChange={setGuidanceRequested}
        onPendingChange={setGuidancePending}
      />
      <DatabaseConversionPromptDialog
        settingsLoaded={settingsLoaded}
        deferred={active !== "conversion"}
        onRequestChange={setConversionRequested}
        onPendingChange={setConversionPending}
      />
      <NsisMigrationNoticeDialog
        dismissed={nsisMigrationNoticeDismissed}
        settingsLoaded={settingsLoaded}
        deferred={active !== "nsis"}
        onRequestChange={setNsisRequested}
        onPendingChange={setNsisPending}
      />
      <ProductAnalyticsConsentDialog
        arbitrationFree={arbitrationFree}
        displayTarget={displayTarget}
        productAnalytics={productAnalytics}
        deferred={active !== "consent"}
        onRequestChange={setConsentRequested}
      />
    </>
  );
};
