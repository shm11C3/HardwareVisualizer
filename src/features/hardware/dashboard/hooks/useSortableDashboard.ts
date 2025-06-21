import type { DragEndEvent } from "@dnd-kit/core";
import { arraySwap } from "@dnd-kit/sortable";
import { useAtom } from "jotai";
import { useEffect } from "react";
import { useHardwareInfoAtom } from "../../hooks/useHardwareInfoAtom";
import { dashboardLayoutAtom } from "../atom/dashboardAtom";
import type { DashboardItemType } from "../types/dashboardItem";

export const useSortableDashboard = () => {
  const { init } = useHardwareInfoAtom();
  const [dashboardItemMap, setDashboardItemMap] = useAtom(dashboardLayoutAtom);

  const handleDragOver = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const oldIndex = dashboardItemMap.indexOf(active.id as DashboardItemType);
    const newIndex = dashboardItemMap.indexOf(over.id as DashboardItemType);

    setDashboardItemMap((prev) => arraySwap(prev, oldIndex, newIndex));
  };

  useEffect(() => {
    init();
  }, [init]);

  return {
    dashboardItemMap,
    handleDragOver,
  };
};
