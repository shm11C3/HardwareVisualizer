import type { DragEndEvent } from "@dnd-kit/core";
import { arraySwap } from "@dnd-kit/sortable";
import { useEffect } from "react";
import { useTauriStore } from "@/hooks/useTauriStore";
import { useHardwareInfoAtom } from "../../hooks/useHardwareInfoAtom";
import type { DashboardItemType } from "../types/dashboardItem";

export const useSortableDashboard = () => {
  const { init } = useHardwareInfoAtom();
  const [dashboardItemMap, setDashboardItemMap] = useTauriStore<
    DashboardItemType[]
  >("dashboardItem", ["cpu", "gpu", "memory", "storage", "network", "process"]);

  const handleDragOver = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    if (!dashboardItemMap) return;

    const oldIndex = dashboardItemMap.indexOf(active.id as DashboardItemType);
    const newIndex = dashboardItemMap.indexOf(over.id as DashboardItemType);

    setDashboardItemMap(arraySwap(dashboardItemMap, oldIndex, newIndex));
  };

  useEffect(() => {
    init();
  }, [init]);

  return {
    dashboardItemMap,
    handleDragOver,
  };
};
