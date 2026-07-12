import { XIcon } from "@phosphor-icons/react";
import { useAtomValue, useSetAtom } from "jotai";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  displayTargetAtom,
  sideMenuOpenAtom,
} from "@/features/menu/hooks/useMenu";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { useTauriStore } from "@/hooks/useTauriStore";
import { cn } from "@/lib/utils";
import type { SelectedDisplayType } from "@/types/ui";

export const GROUPED_NAVIGATION_ANNOUNCEMENT_VERSION = 1;

export const NavigationRestructureNotice = ({
  settingsLoaded,
}: {
  settingsLoaded: boolean;
}) => {
  const { t } = useTranslation();
  const { settings, acknowledgeNavigationRestructureAnnouncementAtom } =
    useSettingsAtom();
  const setDisplayTargetAtom = useSetAtom(displayTargetAtom);
  const [, setStoredDisplayTarget] = useTauriStore<SelectedDisplayType>(
    "display",
    "performance",
  );
  const isMenuOpen = useAtomValue(sideMenuOpenAtom);

  if (
    !settingsLoaded ||
    settings.navigationLayout !== "grouped" ||
    settings.uiAnnouncementVersion >= GROUPED_NAVIGATION_ANNOUNCEMENT_VERSION
  ) {
    return null;
  }

  const openNavigationSettings = () => {
    setDisplayTargetAtom("settings");
    void setStoredDisplayTarget("settings");
    window.setTimeout(() => {
      document.getElementById("classicNavigation")?.focus();
    }, 250);
  };

  return (
    <aside
      className={cn(
        "fixed top-4 right-16 z-50 mx-auto flex max-w-2xl items-start gap-3 rounded-xl border border-border bg-background/95 p-4 shadow-lg backdrop-blur-sm",
        isMenuOpen ? "left-[17rem] max-sm:hidden" : "left-20 sm:left-4",
      )}
      aria-label={t("navigation.notice.title")}
    >
      <div className="min-w-0 flex-1">
        <p className="font-semibold">{t("navigation.notice.title")}</p>
        <p className="mt-1 text-muted-foreground text-sm">
          {t("navigation.notice.description")}
        </p>
        <Button
          type="button"
          variant="link"
          className="mt-1 h-auto p-0"
          onClick={openNavigationSettings}
        >
          {t("navigation.notice.openSettings")}
        </Button>
      </div>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="-mt-2 -mr-2 shrink-0"
        onClick={() => void acknowledgeNavigationRestructureAnnouncementAtom()}
        aria-label={t("navigation.notice.dismiss")}
      >
        <XIcon size={18} />
      </Button>
    </aside>
  );
};
