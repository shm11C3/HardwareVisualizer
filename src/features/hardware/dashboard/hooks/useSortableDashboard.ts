import type { DragEndEvent } from "@dnd-kit/core";
import { arraySwap } from "@dnd-kit/sortable";
import { useEffect } from "react";
import { useTauriStore } from "@/hooks/useTauriStore";
import { useHardwareInfoAtom } from "../../hooks/useHardwareInfoAtom";
import { type DashboardItemType, dashBoardItems } from "../types/dashboardItem";

const DEFAULT_DASHBOARD_ITEMS = [...dashBoardItems];

export const useSortableDashboard = () => {
  const { init } = useHardwareInfoAtom();
  const [dashboardItemMap, setDashboardItemMap] = useTauriStore<
    DashboardItemType[]
  >("dashboardItem", DEFAULT_DASHBOARD_ITEMS);

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

  useEffect(() => {
    if (!dashboardItemMap) return;

    const knownItems = dashboardItemMap.filter((item) =>
      dashBoardItems.includes(item),
    );
    const missingItems = DEFAULT_DASHBOARD_ITEMS.filter(
      (item) => !knownItems.includes(item),
    );
    const normalizedItems = [...knownItems, ...missingItems];

    if (
      normalizedItems.length !== dashboardItemMap.length ||
      normalizedItems.some((item, index) => item !== dashboardItemMap[index])
    ) {
      setDashboardItemMap(normalizedItems);
    }
  }, [dashboardItemMap, setDashboardItemMap]);

  return {
    dashboardItemMap,
    handleDragOver,
  };
};
