import { useAtomValue } from "jotai";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useNavigationLayoutFocusRequest } from "@/hooks/useNavigationLayoutFocusRequest";
import { useSettingsAtom } from "@/hooks/useSettingsAtom";
import { navigationMutationPendingAtom } from "@/store/settings";

export const NavigationLayoutToggle = () => {
  const { t } = useTranslation();
  const { settings, setNavigationLayoutAtom } = useSettingsAtom();
  const navigationMutationPending = useAtomValue(navigationMutationPendingAtom);
  const { focusRequested, clearFocusRequest } =
    useNavigationLayoutFocusRequest();
  const switchRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!focusRequested) return;

    switchRef.current?.focus();
    clearFocusRequest();
  }, [focusRequested, clearFocusRequest]);

  return (
    <div className="flex w-full items-center justify-between gap-4 py-6 xl:w-1/2">
      <div className="space-y-1">
        <Label htmlFor="classicNavigation" className="text-lg">
          {t("pages.settings.general.navigationLayout.name")}
        </Label>
        <p className="text-muted-foreground text-sm">
          {t("pages.settings.general.navigationLayout.description")}
        </p>
      </div>

      <Switch
        ref={switchRef}
        id="classicNavigation"
        checked={settings.navigationLayout === "classic"}
        disabled={navigationMutationPending}
        onCheckedChange={(classic) =>
          setNavigationLayoutAtom(classic ? "classic" : "grouped")
        }
        aria-label={t("pages.settings.general.navigationLayout.name")}
      />
    </div>
  );
};
