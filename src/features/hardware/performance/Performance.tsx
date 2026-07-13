import { GaugeIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { BurnInShift } from "@/components/shared/BurnInShift";
import { Skeleton } from "@/components/ui/skeleton";
import { ProcessesTable } from "@/features/hardware/dashboard/components/ProcessTable";
import { UsageGraphPanel } from "@/features/hardware/usage/Usage";
import { cn } from "@/lib/utils";
import { CurrentValueStrip } from "./components/CurrentValueStrip";
import { CustomLayoutEditor } from "./components/CustomLayoutEditor";
import { PerformancePresetSelector } from "./components/PerformancePresetSelector";
import { usePerformanceLayout } from "./hooks/usePerformanceLayout";
import type { PerformancePanelId } from "./types/performanceLayout";

export const Performance = ({
  isFullScreen = false,
  showTitle = true,
}: {
  isFullScreen?: boolean;
  showTitle?: boolean;
}) => {
  const { t } = useTranslation();
  const {
    preset,
    setPreset,
    customLayout,
    togglePanel,
    handlePanelDragEnd,
    isPending,
  } = usePerformanceLayout();
  const isMonitor = preset === "monitor";

  return (
    <BurnInShift enabled paddingOverride={isMonitor ? 0 : undefined}>
      <main
        className={cn(
          "mx-auto min-h-screen w-full pt-12 pr-4 pb-8",
          isFullScreen ? "pl-4" : "pl-16",
          !isMonitor && "2xl:w-3/4 2xl:px-4",
        )}
        data-performance-preset={preset}
        data-testid="performance-screen"
      >
        <header
          className={cn(
            "mb-4 flex flex-col gap-3",
            isMonitor && "lg:flex-row lg:items-center lg:justify-between",
          )}
        >
          <div className="min-w-0">
            {showTitle && (
              <div className="flex items-center gap-2">
                <GaugeIcon size={32} />
                <h2 className="font-bold text-3xl text-foreground">
                  {t("navigation.performance")}
                </h2>
              </div>
            )}
            {!isMonitor && (
              <p
                className={cn(
                  "text-muted-foreground text-sm",
                  showTitle && "mt-1 ml-10",
                )}
              >
                {t("pages.performance.description")}
              </p>
            )}
          </div>
          <PerformancePresetSelector
            preset={preset}
            onPresetChange={setPreset}
          />
        </header>

        {isPending ? (
          <PerformanceSkeleton />
        ) : (
          <PerformancePresetContent
            preset={preset}
            customLayout={customLayout}
            onPanelToggle={togglePanel}
            onPanelDragEnd={handlePanelDragEnd}
          />
        )}
      </main>
    </BurnInShift>
  );
};

const PerformancePresetContent = ({
  preset,
  customLayout,
  onPanelToggle,
  onPanelDragEnd,
}: {
  preset: ReturnType<typeof usePerformanceLayout>["preset"];
  customLayout: ReturnType<typeof usePerformanceLayout>["customLayout"];
  onPanelToggle: ReturnType<typeof usePerformanceLayout>["togglePanel"];
  onPanelDragEnd: ReturnType<typeof usePerformanceLayout>["handlePanelDragEnd"];
}) => {
  if (preset === "compact") {
    return <CurrentValueStrip />;
  }

  if (preset === "monitor") {
    return (
      <UsageGraphPanel
        fitToContainer
        height="calc(100dvh - 8rem)"
        className="rounded-xl border border-border bg-card/70"
        testId="performance-usage-graphs"
      />
    );
  }

  if (preset === "detailed") {
    return (
      <div className="space-y-4">
        <CurrentValueStrip />
        <PerformancePanel panel="usageGraphs" />
        <PerformancePanel panel="processTable" />
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <CustomLayoutEditor
        layout={customLayout}
        onPanelToggle={onPanelToggle}
        onPanelDragEnd={onPanelDragEnd}
      />
      {customLayout.order.map((panel) =>
        customLayout.visible.includes(panel) ? (
          <PerformancePanel key={panel} panel={panel} />
        ) : null,
      )}
    </div>
  );
};

const PerformancePanel = ({ panel }: { panel: PerformancePanelId }) => {
  const { t } = useTranslation();

  if (panel === "currentValues") {
    return <CurrentValueStrip />;
  }

  return (
    <section
      className="overflow-hidden rounded-xl border border-border bg-card/70 shadow-sm"
      data-testid={`performance-panel-${panel}`}
    >
      <div className="border-border border-b px-4 py-3">
        <h3 className="font-semibold text-sm">
          {t(`pages.performance.panels.${panel}`)}
        </h3>
        <p className="text-muted-foreground text-xs">
          {t(`pages.performance.panelDescriptions.${panel}`)}
        </p>
      </div>
      {panel === "usageGraphs" ? (
        <UsageGraphPanel
          height="min(48rem, calc(100dvh - 16rem))"
          className="space-y-4 p-4"
          testId="performance-usage-graphs"
        />
      ) : (
        <div className="p-4">
          <ProcessesTable />
        </div>
      )}
    </section>
  );
};

const PerformanceSkeleton = () => (
  <div className="space-y-4" data-testid="performance-layout-loading">
    <div className="grid gap-3 md:grid-cols-3">
      <Skeleton className="h-36 rounded-xl" />
      <Skeleton className="h-36 rounded-xl" />
      <Skeleton className="h-36 rounded-xl" />
    </div>
    <Skeleton className="h-80 rounded-xl" />
  </div>
);
