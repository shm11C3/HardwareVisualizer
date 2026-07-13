import type { DragEndEvent } from "@dnd-kit/core";
import { arrayMove } from "@dnd-kit/sortable";
import { useCallback, useEffect, useRef } from "react";
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
  const latestCustomLayoutRef = useRef(customLayout);
  const customLayoutMutationQueueRef = useRef(Promise.resolve());

  useEffect(() => {
    latestCustomLayoutRef.current = customLayout;
  }, [customLayout]);

  const enqueueCustomLayoutMutation = useCallback(
    (
      mutate: (current: typeof customLayout) => typeof customLayout | undefined,
    ) => {
      const mutation = customLayoutMutationQueueRef.current.then(async () => {
        const previousLayout = latestCustomLayoutRef.current;
        const nextLayout = mutate(previousLayout);
        if (nextLayout == null) {
          return false;
        }

        latestCustomLayoutRef.current = nextLayout;
        try {
          await setStoredCustomLayout(nextLayout);
          return true;
        } catch (error) {
          if (
            performanceCustomLayoutsEqual(
              latestCustomLayoutRef.current,
              nextLayout,
            )
          ) {
            latestCustomLayoutRef.current = previousLayout;
          }
          throw error;
        }
      });

      customLayoutMutationQueueRef.current = mutation.then(
        () => undefined,
        () => undefined,
      );
      return mutation;
    },
    [setStoredCustomLayout],
  );

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
    void enqueueCustomLayoutMutation(() => customLayout);
  }, [
    customLayout,
    enqueueCustomLayoutMutation,
    isCustomLayoutPending,
    storedCustomLayout,
  ]);

  const setPreset = (nextPreset: PerformanceLayoutPreset) =>
    setStoredPreset(nextPreset);

  const togglePanel = (panel: PerformancePanelId) =>
    enqueueCustomLayoutMutation((currentLayout) => {
      const isVisible = currentLayout.visible.includes(panel);
      if (isVisible && currentLayout.visible.length === 1) {
        return undefined;
      }

      return {
        ...currentLayout,
        visible: isVisible
          ? currentLayout.visible.filter((candidate) => candidate !== panel)
          : [...currentLayout.visible, panel],
      };
    });

  const handlePanelDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) {
      return;
    }

    void enqueueCustomLayoutMutation((currentLayout) => {
      const oldIndex = currentLayout.order.indexOf(
        active.id as PerformancePanelId,
      );
      const newIndex = currentLayout.order.indexOf(
        over.id as PerformancePanelId,
      );
      if (oldIndex < 0 || newIndex < 0) {
        return undefined;
      }

      return {
        ...currentLayout,
        order: arrayMove(currentLayout.order, oldIndex, newIndex),
      };
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
