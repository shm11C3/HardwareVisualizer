import type { DragEndEvent } from "@dnd-kit/core";
import { arraySwap } from "@dnd-kit/sortable";
import { useTauriStore } from "@/hooks/useTauriStore";
import type { DashboardItemType } from "../types/dashboardItem";

export const useSortableDashboard = () => {
  const [dashboardItemMap, setDashboardItemMap] = useTauriStore<
    DashboardItemType[]
  >("dashboardItem", [
    "cpu",
    "gpu",
    "memory",
    "storage",
    "network",
    "process",
    "motherboard",
  ]);

  const handleDragOver = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    if (!dashboardItemMap) return;

    const oldIndex = dashboardItemMap.indexOf(active.id as DashboardItemType);
    const newIndex = dashboardItemMap.indexOf(over.id as DashboardItemType);

    setDashboardItemMap(arraySwap(dashboardItemMap, oldIndex, newIndex));
  };

  return {
    dashboardItemMap,
    handleDragOver,
  };
};
