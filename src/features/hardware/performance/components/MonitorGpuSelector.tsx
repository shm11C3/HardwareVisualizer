import { useTranslation } from "react-i18next";
import { useGpuAdapters } from "@/features/hardware/hooks/useGpuAdapters";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { GpuAdapterSelector } from "./GpuAdapterSelector";

/**
 * Monitor's adapter attribution, kept in its own component.
 *
 * `useGpuAdapters` is built from atoms that stay referentially stable between
 * samples, but "no readings" can flip with any of them. Calling it from the
 * Performance parent would rerender the whole screen — panels, toolbar, and
 * all — when it does. The subscription belongs where the value is rendered.
 */
export const MonitorGpuSelector = () => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const { adapters, effectiveGpuId, selectGpu, hasNoReadings } =
    useGpuAdapters();

  // Naming the adapter behind a series the user has turned off is noise.
  if (!settings.displayTargets.includes("gpu")) {
    return null;
  }

  return (
    <div className="flex min-w-0 items-center gap-2">
      {/* Monitor is only the graph, so a blank series is the sole evidence the
          user gets. Say why it is blank rather than letting it read as idle. */}
      {hasNoReadings && (
        <p
          className="min-w-0 truncate text-muted-foreground text-xs"
          data-testid="performance-monitor-gpu-unavailable"
        >
          {t("pages.performance.gpuNoLiveReadings")}
        </p>
      )}
      <GpuAdapterSelector
        adapters={adapters}
        selectedId={effectiveGpuId}
        onSelect={selectGpu}
      />
    </div>
  );
};
