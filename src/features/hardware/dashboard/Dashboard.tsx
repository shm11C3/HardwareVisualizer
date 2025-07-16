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
  GraphicsCardIcon,
  HardDrivesIcon,
  MemoryIcon,
  NetworkIcon,
} from "@phosphor-icons/react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";
import { Skeleton } from "@/components/ui/skeleton";
import { ProcessesTable } from "@/features/hardware/dashboard/components/ProcessTable";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { cn } from "@/lib/utils";
import {
  CPUInfo,
  GPUInfo,
  MemoryInfo,
  NetworkInfo,
  StorageDataInfo,
} from "./components/DashboardItems";
import { ExportHardwareInfo } from "./components/ExportHardwareInfo";
import { SortableItem } from "./components/SortableItem";
import { useSortableDashboard } from "./hooks/useSortableDashboard";
import type { DashboardItemType } from "./types/dashboardItem";

type DataTypeKey = "cpu" | "memory" | "storage" | "gpu" | "network" | "process";

export const Dashboard = () => {
  const { dashboardItemMap, handleDragOver } = useSortableDashboard();
  const { settings } = useSettingsAtom();
  const { t } = useTranslation();
  const sensors = useSensors(useSensor(PointerSensor));

  const dataAreaKey2Title: Partial<Record<DataTypeKey, string>> = {
    cpu: "CPU",
    memory: "RAM",
    storage: t("shared.storage"),
    gpu: "GPU",
    network: t("shared.network"),
  };

  const dashboardItemKeyToItems: Record<
    DashboardItemType,
    { component: JSX.Element; icon?: JSX.Element }
  > = {
    cpu: {
      icon: <CpuIcon size={24} color={`rgb(${settings.lineGraphColor.cpu})`} />,
      component: <CPUInfo />,
    },
    gpu: {
      icon: (
        <GraphicsCardIcon
          size={24}
          color={`rgb(${settings.lineGraphColor.gpu})`}
        />
      ),
      component: <GPUInfo />,
    },
    memory: {
      icon: (
        <MemoryIcon
          size={24}
          color={`rgb(${settings.lineGraphColor.memory})`}
        />
      ),
      component: <MemoryInfo />,
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
      component: <NetworkInfo />,
    },
  };

  if (!dashboardItemMap) {
    return (
      <div className="grid grid-cols-2 gap-4">
        <Skeleton className="h-[400px] w-full rounded-md" />
        <Skeleton className="h-[400px] w-full rounded-md" />
        <Skeleton className="h-[400px] w-full rounded-md" />
        <Skeleton className="h-[400px] w-full rounded-md" />
        <Skeleton className="h-[400px] w-full rounded-md" />
        <Skeleton className="h-[400px] w-full rounded-md" />
      </div>
    );
  }

  return (
    <>
      <ExportHardwareInfo />
      <DndContext
        sensors={sensors}
        collisionDetection={closestCorners}
        onDragOver={handleDragOver}
      >
        <SortableContext
          items={dashboardItemMap}
          strategy={rectSortingStrategy}
        >
          <div className="grid grid-cols-2 gap-4">
            {dashboardItemMap.map((key) => (
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
  className = "rounded-2xl bg-zinc-300/50 dark:bg-slate-950/50",
}: {
  children: React.ReactNode;
  title?: string;
  icon?: JSX.Element;
  border?: boolean;
  className?: string;
}) => {
  return (
    <div className="p-4">
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
              <h3 className="align-middle font-bold text-xl">{title}</h3>
            )}
          </div>
          <div className="px-4 pb-4">{children}</div>
        </div>
      }
    </div>
  );
};
