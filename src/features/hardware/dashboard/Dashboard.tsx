import {
  closestCorners,
  DndContext,
  PointerSensor,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import { rectSortingStrategy, SortableContext } from "@dnd-kit/sortable";
import {
  CpuIcon,
  DesktopIcon,
  GraphicsCardIcon,
  HardDrivesIcon,
  MemoryIcon,
  NetworkIcon,
} from "@phosphor-icons/react";
import { platform } from "@tauri-apps/plugin-os";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";
import { Skeleton } from "@/components/ui/skeleton";
import { DashboardItemSelector } from "@/features/hardware/dashboard/components/DashboardItemSelector";
import { ProcessesTable } from "@/features/hardware/dashboard/components/ProcessTable";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { cn } from "@/lib/utils";
import { useHardwareInfoAtom } from "../hooks/useHardwareInfoAtom";
import {
  CPUInfo,
  CPUSpecifications,
  GPUInfo,
  GPUSpecifications,
  MemoryInfo,
  MemorySpecifications,
  MotherboardDataInfo,
  NetworkInfo,
  StorageDataInfo,
} from "./components/DashboardItems";
import { ExportHardwareInfo } from "./components/ExportHardwareInfo";
import { SortableItem } from "./components/SortableItem";
import { useDashboardSelector } from "./hooks/useDashboardSelector";
import { useSortableDashboard } from "./hooks/useSortableDashboard";
import type { DashboardItemType } from "./types/dashboardItem";

type DataTypeKey =
  | "cpu"
  | "memory"
  | "storage"
  | "gpu"
  | "network"
  | "process"
  | "motherboard";

const EMPTY_EXCLUDED_ITEMS: readonly DashboardItemType[] = [];
const SPECIFICATIONS_ITEM_STORE_KEY = "systemSpecificationsItem";
const SPECIFICATIONS_VISIBLE_ITEMS_STORE_KEY =
  "systemSpecificationsVisibleItems";
const SPECIFICATIONS_VISIBLE_ITEMS_VERSION_STORE_KEY =
  "systemSpecificationsVisibleItemsVersion";

export const Dashboard = ({
  excludedItems = EMPTY_EXCLUDED_ITEMS,
  responsive = false,
  specificationsMode = false,
}: {
  excludedItems?: readonly DashboardItemType[];
  responsive?: boolean;
  specificationsMode?: boolean;
}) => {
  const { hardwareInfo } = useHardwareInfoAtom();
  const { dashboardItemMap, handleDragOver } = useSortableDashboard({
    storeKey: specificationsMode
      ? SPECIFICATIONS_ITEM_STORE_KEY
      : "dashboardItem",
  });
  const { settings } = useSettingsAtom();
  const { t } = useTranslation();
  const sensors = useSensors(useSensor(PointerSensor));
  const { visibleItems, toggleItem } = useDashboardSelector({
    visibleItemsKey: specificationsMode
      ? SPECIFICATIONS_VISIBLE_ITEMS_STORE_KEY
      : "dashboardVisibleItems",
    visibleItemsVersionKey: specificationsMode
      ? SPECIFICATIONS_VISIBLE_ITEMS_VERSION_STORE_KEY
      : "dashboardVisibleItemsVersion",
    syncDashboardTitleVisibility: !specificationsMode,
  });
  const os = platform();

  const dataAreaKey2Title: Partial<Record<DataTypeKey, string>> = {
    cpu: "CPU",
    memory: "RAM",
    storage: t("shared.storage"),
    gpu: "GPU",
    network: t("shared.network"),
    motherboard: t("shared.motherboard"),
  };

  const dashboardItemKeyToItems: Record<
    DashboardItemType,
    { component: JSX.Element | null; icon?: JSX.Element }
  > = {
    cpu: {
      icon: <CpuIcon size={24} color={`rgb(${settings.lineGraphColor.cpu})`} />,
      component: specificationsMode ? <CPUSpecifications /> : <CPUInfo />,
    },
    gpu: {
      icon: (
        <GraphicsCardIcon
          size={24}
          color={`rgb(${settings.lineGraphColor.gpu})`}
        />
      ),
      component:
        hardwareInfo.gpus != null && hardwareInfo.gpus.length > 0 ? (
          specificationsMode ? (
            <GPUSpecifications />
          ) : (
            <GPUInfo />
          )
        ) : null,
    },
    memory: {
      icon: (
        <MemoryIcon
          size={24}
          color={`rgb(${settings.lineGraphColor.memory})`}
        />
      ),
      component: specificationsMode ? <MemorySpecifications /> : <MemoryInfo />,
    },
    process: {
      component: <ProcessesTable />,
    },
    storage: {
      icon: <HardDrivesIcon size={24} color="var(--color-storage)" />,
      component: <StorageDataInfo />,
    },
    network: {
      icon: <NetworkIcon size={24} color="oklch(74.6% 0.16 232.661)" />,
      component: <NetworkInfo showUnavailableState={specificationsMode} />,
    },
    motherboard: {
      icon: <DesktopIcon size={24} color="oklch(70% 0.14 150)" />,
      component:
        os === "windows" || os === "macos" ? <MotherboardDataInfo /> : null,
    },
  };

  if (!dashboardItemMap) {
    return (
      <div
        className={cn(
          "grid gap-4",
          responsive ? "grid-cols-1 lg:grid-cols-2" : "grid-cols-2",
        )}
      >
        <Skeleton className="h-[400px] w-full rounded-md" />
        <Skeleton className="h-[400px] w-full rounded-md" />
        <Skeleton className="h-[400px] w-full rounded-md" />
        <Skeleton className="h-[400px] w-full rounded-md" />
        <Skeleton className="h-[400px] w-full rounded-md" />
        <Skeleton className="h-[400px] w-full rounded-md" />
      </div>
    );
  }

  const displayedDashboardItemMap = dashboardItemMap.filter(
    (item) => !excludedItems.includes(item),
  );

  return (
    <>
      <div className="mr-4 flex justify-end gap-3">
        <DashboardItemSelector
          visibleItems={visibleItems}
          toggleItem={toggleItem}
          excludedItems={
            specificationsMode ? [...excludedItems, "title"] : excludedItems
          }
        />
        <ExportHardwareInfo includeRuntimeStats={!specificationsMode} />
      </div>
      <DndContext
        sensors={sensors}
        collisionDetection={closestCorners}
        onDragOver={handleDragOver}
      >
        <SortableContext
          items={displayedDashboardItemMap}
          strategy={rectSortingStrategy}
        >
          <div
            className={cn(
              "grid gap-4",
              responsive ? "grid-cols-1 lg:grid-cols-2" : "grid-cols-2",
            )}
          >
            {displayedDashboardItemMap
              .filter(
                (key) =>
                  dashboardItemKeyToItems[key].component != null &&
                  (visibleItems?.includes(key) ?? true),
              )
              .map((key) => (
                <SortableItem key={key} id={key}>
                  {key !== "process" ? (
                    <DataArea
                      title={dataAreaKey2Title[key]}
                      icon={dashboardItemKeyToItems[key].icon}
                    >
                      {dashboardItemKeyToItems[key].component}
                    </DataArea>
                  ) : (
                    <div className="p-4">
                      {dashboardItemKeyToItems[key].component}
                    </div>
                  )}
                </SortableItem>
              ))}
          </div>
        </SortableContext>
      </DndContext>
    </>
  );
};

const DataArea = ({
  children,
  title,
  icon,
  border = false,
  className = "rounded-2xl bg-card",
}: {
  children: React.ReactNode;
  title?: string | undefined;
  icon?: JSX.Element | undefined;
  border?: boolean;
  className?: string;
}) => {
  return (
    <div className="p-0 lg:p-4">
      {
        <div
          className={cn(
            border && "border border-zinc-400 dark:border-zinc-600",
            className,
          )}
        >
          <div className="flex items-center pt-4 pb-2 pl-10">
            {icon && <div className="mr-2 mb-0.5">{icon}</div>}
            {title && (
              <h3 className="align-middle font-bold text-md lg:text-xl">
                {title}
              </h3>
            )}
          </div>
          <div className="px-4 pb-4">{children}</div>
        </div>
      }
    </div>
  );
};
