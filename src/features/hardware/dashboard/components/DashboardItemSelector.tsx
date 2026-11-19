import { platform } from "@tauri-apps/plugin-os";
import { ListFilterIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  type DashboardSelectItemType,
  dashBoardItems,
} from "@/features/hardware/dashboard/types/dashboardItem";

const EMPTY_EXCLUDED_ITEMS: readonly DashboardSelectItemType[] = [];

export const DashboardItemSelector = ({
  visibleItems,
  toggleItem,
  excludedItems = EMPTY_EXCLUDED_ITEMS,
}: {
  visibleItems: DashboardSelectItemType[] | null;
  toggleItem: (item: DashboardSelectItemType) => void;
  excludedItems?: readonly DashboardSelectItemType[];
}) => {
  const { t } = useTranslation();
  const os = platform();

  if (!visibleItems) return null;

  const itemLabels: Record<DashboardSelectItemType, string> = {
    title: t("shared.title"),
    cpu: "CPU",
    gpu: "GPU",
    memory: "RAM",
    storage: t("shared.storage"),
    process: t("shared.process"),
    network: t("shared.network"),
    motherboard: t("shared.motherboard"),
  };

  const items: DashboardSelectItemType[] = [
    ...(excludedItems.includes("title") ? [] : (["title"] as const)),
    ...dashBoardItems.filter(
      (item) =>
        !excludedItems.includes(item) &&
        (item !== "motherboard" || os === "windows" || os === "macos"),
    ),
  ];

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          aria-label={t("pages.dashboard.visibleItems")}
          title={t("pages.dashboard.visibleItems")}
        >
          <ListFilterIcon className="size-5" aria-hidden="true" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel>
          {t("pages.dashboard.visibleItems")}
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        {items.map((type) => (
          <DropdownMenuCheckboxItem
            key={type}
            checked={visibleItems.includes(type)}
            onCheckedChange={() => toggleItem(type)}
            onSelect={(e) => e.preventDefault()}
          >
            {itemLabels[type]}
          </DropdownMenuCheckboxItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
};
