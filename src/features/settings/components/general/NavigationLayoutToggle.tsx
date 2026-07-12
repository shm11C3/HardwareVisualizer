import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useSettingsAtom } from "../../hooks/useSettingsAtom";

export const NavigationLayoutToggle = () => {
  const { t } = useTranslation();
  const { settings, setNavigationLayoutAtom } = useSettingsAtom();

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
        id="classicNavigation"
        checked={settings.navigationLayout === "classic"}
        onCheckedChange={(classic) =>
          setNavigationLayoutAtom(classic ? "classic" : "grouped")
        }
        aria-label={t("pages.settings.general.navigationLayout.name")}
      />
    </div>
  );
};
