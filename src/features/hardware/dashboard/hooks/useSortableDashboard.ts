import type { DragEndEvent, DragStartEvent } from "@dnd-kit/core";
import { arrayMove } from "@dnd-kit/sortable";
import { useAtom } from "jotai";
import { useEffect, useState } from "react";
import { useHardwareInfoAtom } from "../../hooks/useHardwareInfoAtom";
import { dashboardLayoutAtom } from "../atom/dashboardAtom";
import type { DashboardItemType } from "../types/dashboardItem";

export const useSortableDashboard = () => {
  const { init } = useHardwareInfoAtom();
  const [dashboardItemMap, setDashboardItemMap] = useAtom(dashboardLayoutAtom);
  const [activeId, setActiveId] = useState<DashboardItemType | null>(null);

  const handleDragStart = (event: DragStartEvent) => {
    setActiveId(event.active.id as DashboardItemType);
  };

  const handleDragOver = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const oldIndex = dashboardItemMap.indexOf(active.id as DashboardItemType);
    const newIndex = dashboardItemMap.indexOf(over.id as DashboardItemType);
    setDashboardItemMap(arrayMove(dashboardItemMap, oldIndex, newIndex));
  };

  const handleDragEnd = () => {
    setActiveId(null);
  };

  const handleDragCancel = () => {
    setActiveId(null);
  };

  useEffect(() => {
    init();
  }, [init]);

  return {
    activeId,
    dashboardItemMap,
    handleDragStart,
    handleDragOver,
    handleDragEnd,
    handleDragCancel,
  };
};
