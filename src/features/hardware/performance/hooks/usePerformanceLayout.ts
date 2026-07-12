import type { DragEndEvent } from "@dnd-kit/core";
import { arrayMove } from "@dnd-kit/sortable";
import { useEffect } from "react";
import { useTauriStore } from "@/hooks/useTauriStore";
import {
  DEFAULT_PERFORMANCE_CUSTOM_LAYOUT,
  DEFAULT_PERFORMANCE_PRESET,
  normalizePerformanceCustomLayout,
  normalizePerformancePreset,
  type PerformanceLayoutPreset,
  type PerformancePanelId,
  performanceCustomLayoutsEqual,
} from "../types/performanceLayout";

export const usePerformanceLayout = () => {
  const [storedPreset, setStoredPreset, isPresetPending] =
    useTauriStore<unknown>(
      "performanceLayoutPreset",
      DEFAULT_PERFORMANCE_PRESET,
    );
  const [storedCustomLayout, setStoredCustomLayout, isCustomLayoutPending] =
    useTauriStore<unknown>(
      "performanceCustomLayout",
      DEFAULT_PERFORMANCE_CUSTOM_LAYOUT,
    );

  const preset = normalizePerformancePreset(storedPreset);
  const customLayout = normalizePerformanceCustomLayout(storedCustomLayout);

  useEffect(() => {
    if (isPresetPending || storedPreset === preset) {
      return;
    }
    void setStoredPreset(preset);
  }, [isPresetPending, preset, setStoredPreset, storedPreset]);

  useEffect(() => {
    if (
      isCustomLayoutPending ||
      performanceCustomLayoutsEqual(storedCustomLayout, customLayout)
    ) {
      return;
    }
    void setStoredCustomLayout(customLayout);
  }, [
    customLayout,
    isCustomLayoutPending,
    setStoredCustomLayout,
    storedCustomLayout,
  ]);

  const setPreset = (nextPreset: PerformanceLayoutPreset) =>
    setStoredPreset(nextPreset);

  const togglePanel = async (panel: PerformancePanelId) => {
    const isVisible = customLayout.visible.includes(panel);
    if (isVisible && customLayout.visible.length === 1) {
      return false;
    }

    await setStoredCustomLayout({
      ...customLayout,
      visible: isVisible
        ? customLayout.visible.filter((candidate) => candidate !== panel)
        : [...customLayout.visible, panel],
    });
    return true;
  };

  const handlePanelDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) {
      return;
    }

    const oldIndex = customLayout.order.indexOf(
      active.id as PerformancePanelId,
    );
    const newIndex = customLayout.order.indexOf(over.id as PerformancePanelId);
    if (oldIndex < 0 || newIndex < 0) {
      return;
    }

    void setStoredCustomLayout({
      ...customLayout,
      order: arrayMove(customLayout.order, oldIndex, newIndex),
    });
  };

  return {
    preset,
    setPreset,
    customLayout,
    togglePanel,
    handlePanelDragEnd,
    isPending: isPresetPending || isCustomLayoutPending,
  };
};
