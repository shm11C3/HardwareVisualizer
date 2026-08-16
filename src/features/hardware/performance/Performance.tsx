import {
  ArrowsInSimpleIcon,
  ArrowsOutSimpleIcon,
  CheckIcon,
  GaugeIcon,
  SlidersHorizontalIcon,
} from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { BurnInShift } from "@/components/shared/BurnInShift";
import { Skeleton } from "@/components/ui/skeleton";
import { UsageGraphPanel } from "@/features/hardware/usage/Usage";
import { cn } from "@/lib/utils";
import { CompactStrip } from "./components/CompactStrip";
import { InstrumentStrip } from "./components/InstrumentStrip";
import { PanelColumnsSelector } from "./components/PanelColumnsSelector";
import { PanelGrid } from "./components/PanelGrid";
import { PerformanceViewSwitcher } from "./components/PerformanceViewSwitcher";
import { usePerformanceLayout } from "./hooks/usePerformanceLayout";

export const Performance = ({
  isFullScreen = false,
  showTitle = true,
  embedded = false,
}: {
  isFullScreen?: boolean;
  showTitle?: boolean;
  embedded?: boolean;
}) => {
  const { t } = useTranslation();
  const {
    view,
    setView,
    columns,
    setColumns,
    compactExpanded,
    setCompactExpanded,
    customLayout,
    togglePanel,
    handlePanelDragEnd,
    isPending,
  } = usePerformanceLayout();
  const [editing, setEditing] = useState(false);
  const isMonitor = view === "monitor";
  const isCompactFullScreen = view === "compact" && compactExpanded;

  useEffect(() => {
    if (!isCompactFullScreen) {
      return;
    }

    const exitOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        void setCompactExpanded(false);
      }
    };

    // The rest of the app stays mounted behind the layer, so hide it from
    // assistive technology and keyboard focus while the mini monitor is up.
    const appRoot = document.getElementById("root");
    appRoot?.setAttribute("inert", "");
    appRoot?.setAttribute("aria-hidden", "true");
    window.addEventListener("keydown", exitOnEscape);

    return () => {
      appRoot?.removeAttribute("inert");
      appRoot?.removeAttribute("aria-hidden");
      window.removeEventListener("keydown", exitOnEscape);
    };
  }, [isCompactFullScreen, setCompactExpanded]);

  const content = (
    <main
      className={cn(
        "mx-auto w-full pb-8",
        embedded
          ? "pt-2"
          : cn("min-h-screen pt-12 pr-4", isFullScreen ? "pl-4" : "pl-16"),
        !isMonitor && "2xl:w-3/4 2xl:px-4",
      )}
      data-performance-view={view}
      data-testid="performance-screen"
    >
      <header
        className={cn(
          "mb-4 flex flex-wrap items-center gap-3",
          !showTitle && "justify-end",
        )}
      >
        {showTitle && (
          <div className="flex min-w-0 items-center gap-2">
            <GaugeIcon size={32} />
            <h2 className="font-bold text-3xl text-foreground">
              {t("navigation.performance")}
            </h2>
          </div>
        )}
        <div className={cn("flex items-center gap-2", showTitle && "ml-auto")}>
          {view === "compact" && (
            <button
              type="button"
              onClick={() => void setCompactExpanded(true)}
              className="flex min-h-9 items-center gap-1.5 rounded-md border border-border px-3 text-muted-foreground text-sm hover:bg-muted hover:text-foreground"
              data-testid="performance-compact-expand"
            >
              <ArrowsOutSimpleIcon size={15} />
              {t("pages.performance.expandCompact")}
            </button>
          )}
          {view === "panels" && (
            <PanelColumnsSelector
              columns={columns}
              onColumnsChange={setColumns}
            />
          )}
          {view === "panels" && (
            <button
              type="button"
              onClick={() => setEditing((current) => !current)}
              aria-pressed={editing}
              className={cn(
                "flex min-h-9 items-center gap-1.5 rounded-md border border-border px-3 text-sm",
                editing
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:bg-muted hover:text-foreground",
              )}
              data-testid="performance-edit-toggle"
            >
              {editing ? (
                <CheckIcon size={15} />
              ) : (
                <SlidersHorizontalIcon size={15} />
              )}
              {editing
                ? t("pages.performance.doneEditing")
                : t("pages.performance.editPanels")}
            </button>
          )}
          <PerformanceViewSwitcher view={view} onViewChange={setView} />
        </div>
      </header>

      {isPending ? (
        <PerformanceSkeleton />
      ) : view === "compact" ? (
        <CompactStrip />
      ) : isMonitor ? (
        <UsageGraphPanel
          fitToContainer
          height={embedded ? "calc(100dvh - 12rem)" : "calc(100dvh - 8rem)"}
          testId="performance-usage-graphs"
        />
      ) : (
        <div className="space-y-4">
          <InstrumentStrip />
          <PanelGrid
            layout={customLayout}
            columns={columns}
            editing={editing}
            onPanelToggle={togglePanel}
            onPanelDragEnd={handlePanelDragEnd}
          />
        </div>
      )}
    </main>
  );

  if (isCompactFullScreen) {
    // Mini-monitor mode: the strip is the whole screen. Rendered in a portal
    // so it sits outside the inert app root rather than merely on top of it.
    return createPortal(
      <div
        // The portal target sits outside the app shell, so the theme's
        // foreground color has to be restated here rather than inherited.
        // Burn-in Shift must be re-established too: a mini monitor shows
        // sustained static content, which is exactly what the shift protects.
        className="fixed inset-0 z-70 bg-background text-foreground [&>.burnin-root>.burnin-shift>div]:h-full [&>.burnin-root>.burnin-shift]:h-full [&>.burnin-root]:h-full"
        data-testid="performance-compact-fullscreen"
      >
        <BurnInShift enabled paddingOverride={12}>
          <div className="relative flex h-full flex-col px-3 pt-11 pb-2">
            <button
              type="button"
              onClick={() => void setCompactExpanded(false)}
              className="absolute top-0 right-0 z-10 flex min-h-9 items-center gap-1.5 rounded-full border border-border bg-card px-3 text-muted-foreground text-sm hover:bg-muted hover:text-foreground"
              data-testid="performance-compact-collapse"
            >
              <ArrowsInSimpleIcon size={15} />
              {t("pages.performance.exitCompactFullScreen")}
            </button>
            <CompactStrip fillViewport />
          </div>
        </BurnInShift>
      </div>,
      document.body,
    );
  }

  if (embedded) {
    return content;
  }

  return (
    <BurnInShift enabled paddingOverride={isMonitor ? 0 : undefined}>
      {content}
    </BurnInShift>
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
