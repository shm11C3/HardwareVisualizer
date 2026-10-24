import { useLayoutEffect, useState } from "react";
import type { PerformanceGuidanceContext } from "@/lib/externalComponentGuidance";
import type { SelectedDisplayType } from "@/types/ui";
import { CloseToTrayFirstRunDialog } from "./CloseToTrayFirstRunDialog";
import { DatabaseConversionPromptDialog } from "./DatabaseConversionPromptDialog";
import { ExternalComponentGuidanceDialog } from "./ExternalComponentGuidanceDialog";
import { NsisMigrationNoticeDialog } from "./NsisMigrationNoticeDialog";

type Dialog = "close" | "guidance" | "conversion" | "nsis";

/** App-owned arbitration: a late reading or close request never covers an active flow. */
export const StartupDialogs = ({
  settingsLoaded,
  settingsSettled,
  displayTarget,
  performanceContext,
  nsisMigrationNoticeDismissed,
}: {
  settingsLoaded: boolean;
  settingsSettled: boolean;
  displayTarget: SelectedDisplayType | null;
  performanceContext: PerformanceGuidanceContext;
  nsisMigrationNoticeDismissed: boolean;
}) => {
  const [active, setActive] = useState<Dialog | null>(null);
  const [closeRequested, setCloseRequested] = useState(false);
  const [guidanceRequested, setGuidanceRequested] = useState(false);
  const [conversionRequested, setConversionRequested] = useState(false);
  const [nsisRequested, setNsisRequested] = useState(false);
  const [guidancePending, setGuidancePending] = useState(true);
  const [conversionPending, setConversionPending] = useState(true);

  useLayoutEffect(() => {
    const requests = {
      close: closeRequested,
      guidance: guidanceRequested,
      conversion: conversionRequested,
      nsis: nsisRequested,
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
              : null,
    );
  }, [
    active,
    closeRequested,
    guidanceRequested,
    conversionRequested,
    nsisRequested,
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
        performanceContext={performanceContext}
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
      />
    </>
  );
};
