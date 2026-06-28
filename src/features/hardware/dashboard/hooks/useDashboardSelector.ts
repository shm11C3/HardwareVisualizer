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
const CURRENT_VISIBLE_ITEMS_VERSION = 1;

export const useDashboardSelector = () => {
  const [visibleItems, setVisibleItems] = useTauriStore<
    DashboardSelectItemType[]
  >("dashboardVisibleItems", DEFAULT_VISIBLE_ITEMS);
  const [visibleItemsVersion, setVisibleItemsVersion] = useTauriStore<number>(
    "dashboardVisibleItemsVersion",
    0,
  );
  const { toggleTitleIconVisibility } = useTitleIconVisualSelector();

  useEffect(() => {
    if (visibleItems == null || visibleItemsVersion == null) {
      return;
    }
    if (visibleItemsVersion >= CURRENT_VISIBLE_ITEMS_VERSION) {
      return;
    }

    const knownVisibleItems = visibleItems.filter((item) =>
      DEFAULT_VISIBLE_ITEMS.includes(item),
    );
    const missingDefaultItems = DEFAULT_VISIBLE_ITEMS.filter(
      (item) => !knownVisibleItems.includes(item),
    );
    const migratedItems = [...knownVisibleItems, ...missingDefaultItems];

    const migrateVisibleItems = async () => {
      if (
        migratedItems.length !== visibleItems.length ||
        migratedItems.some((item, index) => item !== visibleItems[index])
      ) {
        await setVisibleItems(migratedItems);
      }
      await setVisibleItemsVersion(CURRENT_VISIBLE_ITEMS_VERSION);
    };

    void migrateVisibleItems();
  }, [
    visibleItems,
    visibleItemsVersion,
    setVisibleItems,
    setVisibleItemsVersion,
  ]);

  useEffect(() => {
    if (visibleItems == null) {
      return;
    }

    toggleTitleIconVisibility("dashboard", visibleItems.includes("title"));
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
