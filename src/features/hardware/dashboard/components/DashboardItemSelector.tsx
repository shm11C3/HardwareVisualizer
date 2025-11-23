import { FunnelIcon } from "@phosphor-icons/react";
import { useId } from "react";
import { useTranslation } from "react-i18next";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { DashboardItemType } from "@/features/hardware/dashboard/types/dashboardItem";

export const DashboardItemSelector = ({
  visibleItems,
  toggleItem,
}: {
  visibleItems: DashboardItemType[] | null;
  toggleItem: (item: DashboardItemType) => void;
}) => {
  const { t } = useTranslation();

  const cpuId = useId();
  const gpuId = useId();
  const memoryId = useId();
  const storageId = useId();
  const processId = useId();
  const networkId = useId();

  if (!visibleItems) return null;

  const itemLabels: Record<DashboardItemType, string> = {
    cpu: "CPU",
    gpu: "GPU",
    memory: "RAM",
    storage: t("shared.storage"),
    process: t("shared.process"),
    network: t("shared.network"),
  };

  const items: { id: string; type: DashboardItemType }[] = [
    { id: cpuId, type: "cpu" },
    { id: gpuId, type: "gpu" },
    { id: memoryId, type: "memory" },
    { id: storageId, type: "storage" },
    { id: processId, type: "process" },
    { id: networkId, type: "network" },
  ];

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          className="rounded-lg bg-zinc-200 p-2 hover:bg-zinc-300 dark:bg-slate-800 dark:hover:bg-slate-700"
          type="button"
        >
          <FunnelIcon size={32} />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel>
          {t("pages.dashboard.visibleItems")}
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        {items.map(({ type }) => (
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
