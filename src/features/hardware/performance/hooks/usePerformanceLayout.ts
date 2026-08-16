import type { DragEndEvent } from "@dnd-kit/core";
import { arrayMove } from "@dnd-kit/sortable";
import { useCallback, useEffect, useRef } from "react";
import { useTauriStore } from "@/hooks/useTauriStore";
import {
  DEFAULT_PERFORMANCE_CUSTOM_LAYOUT,
  DEFAULT_PERFORMANCE_PANEL_COLUMNS,
  DEFAULT_PERFORMANCE_VIEW,
  normalizePerformanceCustomLayout,
  normalizePerformancePanelColumns,
  normalizePerformanceView,
  type PerformancePanelColumns,
  type PerformancePanelId,
  type PerformanceView,
  performanceCustomLayoutsEqual,
} from "../types/performanceLayout";

const reportCustomLayoutPersistenceError = (error: unknown) => {
  console.error("Failed to persist custom Performance layout:", error);
};

export const usePerformanceLayout = () => {
  // The store key predates the preset-to-view consolidation; legacy preset
  // values keep resolving through normalizePerformanceView.
  const [storedView, setStoredView, isViewPending] = useTauriStore<unknown>(
    "performanceLayoutPreset",
    DEFAULT_PERFORMANCE_VIEW,
  );
  const [storedCustomLayout, setStoredCustomLayout, isCustomLayoutPending] =
    useTauriStore<unknown>(
      "performanceCustomLayout",
      DEFAULT_PERFORMANCE_CUSTOM_LAYOUT,
    );
  const [storedColumns, setStoredColumns, isColumnsPending] =
    useTauriStore<unknown>(
      "performancePanelColumns",
      DEFAULT_PERFORMANCE_PANEL_COLUMNS,
    );
  // Mini-monitor mode survives a restart so a dedicated screen keeps showing
  // only the strip without being re-armed on every launch.
  const [storedCompactExpanded, setStoredCompactExpanded, isExpandedPending] =
    useTauriStore<boolean>("performanceCompactExpanded", false);

  const view = normalizePerformanceView(storedView);
  const columns = normalizePerformancePanelColumns(storedColumns);
  const customLayout = normalizePerformanceCustomLayout(storedCustomLayout);
  const latestCustomLayoutRef = useRef(customLayout);
  const customLayoutMutationQueueRef = useRef(Promise.resolve());
  const pendingCustomLayoutMutationCountRef = useRef(0);

  useEffect(() => {
    if (pendingCustomLayoutMutationCountRef.current === 0) {
      latestCustomLayoutRef.current = customLayout;
    }
  }, [customLayout]);

  const enqueueCustomLayoutMutation = useCallback(
    (
      mutate: (current: typeof customLayout) => typeof customLayout | undefined,
    ) => {
      pendingCustomLayoutMutationCountRef.current += 1;
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
      const trackedMutation = mutation.finally(() => {
        pendingCustomLayoutMutationCountRef.current -= 1;
      });

      customLayoutMutationQueueRef.current = trackedMutation.then(
        () => undefined,
        () => undefined,
      );
      return trackedMutation;
    },
    [setStoredCustomLayout],
  );

  useEffect(() => {
    if (isViewPending || storedView === view) {
      return;
    }
    void setStoredView(view);
  }, [isViewPending, view, setStoredView, storedView]);

  useEffect(() => {
    if (
      isCustomLayoutPending ||
      performanceCustomLayoutsEqual(storedCustomLayout, customLayout)
    ) {
      return;
    }
    void enqueueCustomLayoutMutation(() => customLayout).catch(
      reportCustomLayoutPersistenceError,
    );
  }, [
    customLayout,
    enqueueCustomLayoutMutation,
    isCustomLayoutPending,
    storedCustomLayout,
  ]);

  const setView = (nextView: PerformanceView) => setStoredView(nextView);

  const setColumns = (nextColumns: PerformancePanelColumns) =>
    setStoredColumns(nextColumns);

  const togglePanel = (panel: PerformancePanelId) =>
    enqueueCustomLayoutMutation((currentLayout) => {
      const isVisible = currentLayout.visible.includes(panel);
      return {
        ...currentLayout,
        visible: isVisible
          ? currentLayout.visible.filter((candidate) => candidate !== panel)
          : [...currentLayout.visible, panel],
      };
    }).catch((error) => {
      reportCustomLayoutPersistenceError(error);
      return false;
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
    }).catch(reportCustomLayoutPersistenceError);
  };

  return {
    view,
    setView,
    columns,
    setColumns,
    compactExpanded: storedCompactExpanded === true,
    // Stable across renders (useTauriStore memoizes it), so effects can key
    // off it without resubscribing every render.
    setCompactExpanded: setStoredCompactExpanded,
    customLayout,
    togglePanel,
    handlePanelDragEnd,
    isPending:
      isViewPending ||
      isCustomLayoutPending ||
      isColumnsPending ||
      isExpandedPending,
  };
};
