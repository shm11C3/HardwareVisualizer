import { useAtomValue } from "jotai";
import type {
  PerformanceCustomLayout,
  PerformancePanelId,
} from "@/features/hardware/performance/types/performanceLayout";
import { powerDrawAvailableAtom } from "@/features/hardware/store/power";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";

/**
 * Panels the editor can actually show, in layout order, and the subset of
 * them that is hidden. Power is available only while a reading exists and a
 * display target is selected, so an unavailable Power panel is never offered
 * (or counted) as a hidden panel.
 */
export const useAvailablePerformancePanels = (
  layout: PerformanceCustomLayout,
): {
  availablePanels: PerformancePanelId[];
  hiddenPanels: PerformancePanelId[];
} => {
  const powerAvailable = useAtomValue(powerDrawAvailableAtom);
  const { settings } = useSettingsAtom();
  const availablePanels = layout.order.filter(
    (panel) =>
      panel !== "power" ||
      (powerAvailable && settings.powerDisplayTargets.length > 0),
  );
  const hiddenPanels = availablePanels.filter(
    (panel) => !layout.visible.includes(panel),
  );

  return { availablePanels, hiddenPanels };
};
