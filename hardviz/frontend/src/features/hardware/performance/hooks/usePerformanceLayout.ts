import type { DragEndEvent } from "@dnd-kit/core";
import { arrayMove } from "@dnd-kit/sortable";
import { useCallback, useEffect, useRef } from "react";
import {
  DEFAULT_PERFORMANCE_CUSTOM_LAYOUT,
  DEFAULT_PERFORMANCE_PANEL_COLUMNS,
  DEFAULT_PERFORMANCE_POWER_MODE,
  DEFAULT_PERFORMANCE_VIEW,
  normalizePerformanceCustomLayout,
  normalizePerformanceDiscoveredPanels,
  normalizePerformancePanelColumns,
  normalizePerformancePowerMode,
  normalizePerformanceView,
  type PerformancePanelColumns,
  type PerformancePanelId,
  type PerformancePowerMode,
  type PerformanceView,
  performanceCustomLayoutsEqual,
} from "@/features/hardware/performance/types/performanceLayout";
import { useTauriStore } from "@/hooks/tauri/useTauriStore";

const reportCustomLayoutPersistenceError = (error: unknown) => {
  console.error("Failed to persist custom Performance layout:", error);
};

const reportPowerModePersistenceError = (error: unknown) => {
  console.error("Failed to persist Performance Power Draw mode:", error);
};

const reportDiscoveredPanelsPersistenceError = (error: unknown) => {
  console.error("Failed to persist discovered Performance panels:", error);
};

// Module-level so the useTauriStore default keeps a stable identity.
const NO_DISCOVERED_PANELS: PerformancePanelId[] = [];

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
  const [storedPowerMode, setStoredPowerMode, isPowerModePending] =
    useTauriStore<unknown>(
      // The persisted key predates sharing this mode with Panels. Keep it so
      // existing Monitor selections continue to apply to both views.
      "performanceMonitorPowerMode",
      DEFAULT_PERFORMANCE_POWER_MODE,
    );
  // Which hidden panels the user has already seen in the edit strip. Older
  // saved layouts cannot distinguish an intentional hide from an untouched old
  // default, so an absent key yields a one-time badge on upgrade. This never
  // changes visibility.
  const [
    storedDiscoveredPanels,
    setStoredDiscoveredPanels,
    isDiscoveredPanelsPending,
  ] = useTauriStore<unknown>(
    "performanceDiscoveredPanels",
    NO_DISCOVERED_PANELS,
  );

  const view = normalizePerformanceView(storedView);
  const discoveredPanels = normalizePerformanceDiscoveredPanels(
    storedDiscoveredPanels,
  );
  const columns = normalizePerformancePanelColumns(storedColumns);
  const customLayout = normalizePerformanceCustomLayout(storedCustomLayout);
  const powerMode = normalizePerformancePowerMode(storedPowerMode);
  const latestCustomLayoutRef = useRef(customLayout);
  const customLayoutMutationQueueRef = useRef(Promise.resolve());
  const pendingCustomLayoutMutationCountRef = useRef(0);
  const powerModeMutationQueueRef = useRef(Promise.resolve());
  const latestDiscoveredPanelsRef = useRef(discoveredPanels);
  const discoveredPanelsMutationQueueRef = useRef(Promise.resolve());
  const pendingDiscoveredPanelsMutationCountRef = useRef(0);

  useEffect(() => {
    if (pendingDiscoveredPanelsMutationCountRef.current === 0) {
      latestDiscoveredPanelsRef.current = normalizePerformanceDiscoveredPanels(
        storedDiscoveredPanels,
      );
    }
  }, [storedDiscoveredPanels]);

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

  const setPowerMode = useCallback(
    (nextMode: PerformancePowerMode) => {
      const mutation = powerModeMutationQueueRef.current.then(() =>
        setStoredPowerMode(nextMode),
      );
      powerModeMutationQueueRef.current = mutation.then(
        () => undefined,
        () => undefined,
      );
      return mutation.catch(reportPowerModePersistenceError);
    },
    [setStoredPowerMode],
  );

  const markPanelsDiscovered = useCallback(
    (panels: PerformancePanelId[]) => {
      pendingDiscoveredPanelsMutationCountRef.current += 1;
      const mutation = discoveredPanelsMutationQueueRef.current.then(
        async () => {
          const previous = latestDiscoveredPanelsRef.current;
          const additions = panels.filter((panel) => !previous.includes(panel));
          if (additions.length === 0) {
            return;
          }

          const next = normalizePerformanceDiscoveredPanels([
            ...previous,
            ...additions,
          ]);
          latestDiscoveredPanelsRef.current = next;
          try {
            await setStoredDiscoveredPanels(next);
          } catch (error) {
            latestDiscoveredPanelsRef.current = previous;
            throw error;
          }
        },
      );
      const trackedMutation = mutation.finally(() => {
        pendingDiscoveredPanelsMutationCountRef.current -= 1;
      });
      discoveredPanelsMutationQueueRef.current = trackedMutation.then(
        () => undefined,
        () => undefined,
      );
      return trackedMutation.catch(reportDiscoveredPanelsPersistenceError);
    },
    [setStoredDiscoveredPanels],
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

  useEffect(() => {
    if (isPowerModePending || storedPowerMode === powerMode) {
      return;
    }
    // Persist the normalized fallback too, so an obsolete value cannot keep
    // being re-read as unknown on every launch.
    void setPowerMode(powerMode);
  }, [isPowerModePending, powerMode, setPowerMode, storedPowerMode]);

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
    powerMode,
    setPowerMode,
    compactExpanded: storedCompactExpanded === true,
    // Stable across renders (useTauriStore memoizes it), so effects can key
    // off it without resubscribing every render.
    setCompactExpanded: setStoredCompactExpanded,
    customLayout,
    discoveredPanels,
    markPanelsDiscovered,
    togglePanel,
    handlePanelDragEnd,
    isPending:
      isViewPending ||
      isCustomLayoutPending ||
      isColumnsPending ||
      isExpandedPending ||
      isPowerModePending ||
      isDiscoveredPanelsPending,
  };
};
