import { XIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { useElevationAvailability } from "@/hooks/useElevationAvailability";

/**
 * Tells the user once per launch that Run as administrator on startup was not
 * applied because the app is installed outside Program Files (#2216). The
 * saved setting is kept (DP-06); the user can turn it off from here. Non-modal
 * so it never stacks on top of another dialog.
 */
export const ElevationUnavailableNotice = ({
  settingsLoaded,
}: {
  settingsLoaded: boolean;
}) => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();
  const availability = useElevationAvailability();
  const [dismissedThisLaunch, setDismissedThisLaunch] = useState(false);

  if (
    !settingsLoaded ||
    !settings.elevatedStartupMode ||
    availability !== "unprotectedLocation" ||
    dismissedThisLaunch
  ) {
    return null;
  }

  return (
    <aside
      className="fixed right-4 bottom-4 z-50 flex max-w-md items-start gap-3 rounded-xl border border-border bg-background/95 p-4 shadow-lg backdrop-blur-sm"
      aria-label={t("elevationUnavailable.startupNotice.title")}
    >
      <div className="min-w-0 flex-1">
        <p className="font-semibold">
          {t("elevationUnavailable.startupNotice.title")}
        </p>
        <p className="mt-1 text-muted-foreground text-sm">
          {t("elevationUnavailable.startupNotice.description")}
        </p>
        <Button
          type="button"
          variant="link"
          className="mt-1 h-auto p-0"
          onClick={() => void updateSettingAtom("elevatedStartupMode", false)}
        >
          {t("elevationUnavailable.startupNotice.turnOff")}
        </Button>
      </div>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="-mt-2 -mr-2 shrink-0"
        onClick={() => setDismissedThisLaunch(true)}
        aria-label={t("elevationUnavailable.startupNotice.dismiss")}
      >
        <XIcon size={18} />
      </Button>
    </aside>
  );
};
