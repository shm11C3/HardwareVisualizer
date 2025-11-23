import { useTauriStore } from "@/hooks/useTauriStore";
import type { DashboardItemType } from "../types/dashboardItem";

export const useDashboardSelector = () => {
  const [visibleItems, setVisibleItems] = useTauriStore<DashboardItemType[]>(
    "dashboardVisibleItems",
    [],
  );

  const toggleItem = async (item: DashboardItemType) => {
    if (!visibleItems) return;

    const newItems = visibleItems.includes(item)
      ? visibleItems.filter((i) => i !== item)
      : [...visibleItems, item];

    // Prevent empty selection
    if (newItems.length === 0) return;

    await setVisibleItems(newItems);
  };

  return { visibleItems, toggleItem };
};
