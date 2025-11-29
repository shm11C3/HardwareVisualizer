import { useEffect } from "react";
import { useTauriStore } from "@/hooks/useTauriStore";
import { useTitleIconVisualSelector } from "@/hooks/useTitleIconVisualSelector";
import {
  type DashboardSelectItemType,
  dashBoardItems,
} from "../types/dashboardItem";

const DEFAULT_VISIBLE_ITEMS: DashboardSelectItemType[] = [
  ...dashBoardItems,
  "title",
] as const;

export const useDashboardSelector = () => {
  const [visibleItems, setVisibleItems] = useTauriStore<
    DashboardSelectItemType[]
  >("dashboardVisibleItems", DEFAULT_VISIBLE_ITEMS);
  const { toggleTitleIconVisibility } = useTitleIconVisualSelector();

  useEffect(() => {
    toggleTitleIconVisibility(
      "dashboard",
      visibleItems?.includes("title") || false,
    );
  }, [visibleItems, toggleTitleIconVisibility]);

  const toggleItem = async (item: DashboardSelectItemType) => {
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
