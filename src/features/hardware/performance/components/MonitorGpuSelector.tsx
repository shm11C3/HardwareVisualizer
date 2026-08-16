import { useGpuAdapters } from "@/features/hardware/hooks/useGpuAdapters";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { GpuAdapterSelector } from "./GpuAdapterSelector";

/**
 * Monitor's adapter attribution, kept in its own component.
 *
 * `useGpuAdapters` subscribes to atoms the event listener rewrites on every
 * sample, so calling it from the Performance parent would rerender the whole
 * screen — panels, toolbar, and all — once a second. The subscription belongs
 * where the value is rendered.
 */
export const MonitorGpuSelector = () => {
  const { settings } = useSettingsAtom();
  const { adapters, effectiveGpuId, selectGpu } = useGpuAdapters();

  // Naming the adapter behind a series the user has turned off is noise.
  if (!settings.displayTargets.includes("gpu")) {
    return null;
  }

  return (
    <GpuAdapterSelector
      adapters={adapters}
      selectedId={effectiveGpuId}
      onSelect={selectGpu}
    />
  );
};
