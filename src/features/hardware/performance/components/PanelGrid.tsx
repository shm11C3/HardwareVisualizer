import {
  closestCenter,
  DndContext,
  type DragEndEvent,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import {
  rectSortingStrategy,
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import {
  ChartLineIcon,
  CpuIcon,
  DesktopIcon,
  EyeSlashIcon,
  PlusIcon,
} from "@phosphor-icons/react";
import { GripVerticalIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ProcessesTable } from "@/features/hardware/dashboard/components/ProcessTable";
import { UsageGraphPanel } from "@/features/hardware/usage/Usage";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { cn } from "@/lib/utils";
import type {
  PerformanceCustomLayout,
  PerformancePanelColumns,
  PerformancePanelId,
} from "../types/performanceLayout";
import { MotherboardSensorsPanel } from "./MotherboardSensorsPanel";
import { PerCorePanel } from "./PerCorePanel";

const PanelBody = ({ panel }: { panel: PerformancePanelId }) => {
  if (panel === "usageGraphs") {
    return (
      <UsageGraphPanel
        height="min(48rem, calc(100dvh - 16rem))"
        className="space-y-4 p-4"
        testId="performance-usage-graphs"
      />
    );
  }

  if (panel === "processTable") {
    return <ProcessesTable headingStyle="panel" />;
  }

  if (panel === "perCore") {
    return <PerCorePanel />;
  }

  return <MotherboardSensorsPanel />;
};

const SortablePanel = ({
  panel,
  editing,
  onHide,
}: {
  panel: PerformancePanelId;
  editing: boolean;
  onHide: (panel: PerformancePanelId) => void;
}) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const { attributes, listeners, setNodeRef, transform, transition } =
    useSortable({ id: panel, disabled: !editing });
  const label = t(`pages.performance.panels.${panel}`);
  // Classic Hardware Dashboard card icons; the Live Process Table brings its
  // own icon inside its card.
  const panelIcons: Partial<Record<PerformancePanelId, React.ReactNode>> = {
    usageGraphs: (
      <ChartLineIcon size={18} color={`rgb(${settings.lineGraphColor.cpu})`} />
    ),
    perCore: (
      <CpuIcon size={18} color={`rgb(${settings.lineGraphColor.cpu})`} />
    ),
    motherboardSensors: <DesktopIcon size={18} color="oklch(70% 0.14 150)" />,
  };
  // The Live Process Table already renders as its own card, so it gets no
  // outer panel chrome; its edit controls appear as a slim row above it.
  const chromeless = panel === "processTable";

  return (
    <section
      ref={setNodeRef}
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
      }}
      className={chromeless ? undefined : "overflow-hidden rounded-2xl bg-card"}
      data-testid={`performance-panel-${panel}`}
    >
      <div
        className={cn(
          "flex min-h-9 items-center gap-1 px-4 pt-2.5 pb-0.5",
          chromeless && !editing && "hidden",
          // A chromeless panel carries its own heading, so the edit row shows
          // controls only instead of stacking a second title above it.
          chromeless && "justify-end",
        )}
      >
        {!chromeless && (
          <>
            {panelIcons[panel]}
            <h3 className="font-mono font-semibold text-[11px] text-muted-foreground uppercase tracking-[0.18em]">
              {label}
            </h3>
          </>
        )}
        {editing && (
          <>
            <button
              type="button"
              onClick={() => onHide(panel)}
              className="ml-auto flex size-7 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
              aria-label={t("pages.performance.hidePanel", { panel: label })}
            >
              <EyeSlashIcon size={15} />
            </button>
            <button
              {...attributes}
              {...listeners}
              type="button"
              className="flex size-7 shrink-0 cursor-grab items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
              aria-label={t("pages.performance.reorderPanel", { panel: label })}
            >
              <GripVerticalIcon className="size-4" />
            </button>
          </>
        )}
      </div>
      <PanelBody panel={panel} />
    </section>
  );
};

/**
 * Ordered stack of Performance panels. Only visible panels mount, so a hidden
 * panel stops subscribing to live updates entirely. Reordering and visibility
 * controls exist only while the edit mode is active.
 */
export const PanelGrid = ({
  layout,
  columns,
  editing,
  onPanelToggle,
  onPanelDragEnd,
}: {
  layout: PerformanceCustomLayout;
  columns: PerformancePanelColumns;
  editing: boolean;
  onPanelToggle: (panel: PerformancePanelId) => Promise<boolean>;
  onPanelDragEnd: (event: DragEndEvent) => void;
}) => {
  const { t } = useTranslation();
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );
  const hiddenPanels = layout.order.filter(
    (panel) => !layout.visible.includes(panel),
  );
  const isTwoColumn = columns === 2;

  return (
    <div className="space-y-4">
      <DndContext
        collisionDetection={closestCenter}
        sensors={sensors}
        onDragEnd={onPanelDragEnd}
      >
        <SortableContext
          items={layout.order}
          strategy={
            isTwoColumn ? rectSortingStrategy : verticalListSortingStrategy
          }
        >
          {/* Two columns are a maximum: below the xl breakpoint the panels
              would be too narrow to read, so the grid collapses to one. */}
          <div
            className={cn(
              "grid items-start gap-4",
              isTwoColumn ? "grid-cols-1 xl:grid-cols-2" : "grid-cols-1",
            )}
            data-panel-columns={columns}
          >
            {layout.order.map((panel) =>
              layout.visible.includes(panel) ? (
                <SortablePanel
                  key={panel}
                  panel={panel}
                  editing={editing}
                  onHide={(hidden) => void onPanelToggle(hidden)}
                />
              ) : null,
            )}
          </div>
        </SortableContext>
      </DndContext>

      {editing && hiddenPanels.length > 0 && (
        <div
          className="flex flex-wrap items-center gap-2 rounded-xl border border-border border-dashed px-4 py-2.5 text-muted-foreground text-xs"
          data-testid="performance-hidden-panels"
        >
          <span>{t("pages.performance.hiddenPanels")}</span>
          {hiddenPanels.map((panel) => {
            const label = t(`pages.performance.panels.${panel}`);
            return (
              <button
                key={panel}
                type="button"
                onClick={() => void onPanelToggle(panel)}
                className="flex items-center gap-1 rounded-full border border-border bg-card/80 px-3 py-1 text-muted-foreground text-xs hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
                aria-label={t("pages.performance.showPanel", { panel: label })}
              >
                <PlusIcon size={12} />
                {label}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
};
