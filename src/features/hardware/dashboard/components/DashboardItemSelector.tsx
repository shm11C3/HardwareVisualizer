import { FunnelIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
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
  dashBoardItemType,
} from "@/features/hardware/dashboard/types/dashboardItem";

export const DashboardItemSelector = ({
  visibleItems,
  toggleItem,
}: {
  visibleItems: DashboardSelectItemType[] | null;
  toggleItem: (item: DashboardSelectItemType) => void;
}) => {
  const { t } = useTranslation();

  if (!visibleItems) return null;

  const itemLabels: Record<DashboardSelectItemType, string> = {
    title: t("shared.title"),
    cpu: "CPU",
    gpu: "GPU",
    memory: "RAM",
    storage: t("shared.storage"),
    process: t("shared.process"),
    network: t("shared.network"),
  };

  const items: DashboardSelectItemType[] = ["title", ...dashBoardItemType];

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          className="rounded-lg bg-zinc-200 p-2 hover:bg-zinc-300 dark:bg-slate-800 dark:hover:bg-slate-700"
          type="button"
          aria-label={t("pages.dashboard.visibleItems")}
        >
          <FunnelIcon size={32} />
        </button>
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
