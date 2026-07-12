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
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVerticalIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import type {
  PerformanceCustomLayout,
  PerformancePanelId,
} from "../types/performanceLayout";

export const CustomLayoutEditor = ({
  layout,
  onPanelToggle,
  onPanelDragEnd,
}: {
  layout: PerformanceCustomLayout;
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

  return (
    <section className="rounded-xl border border-border bg-muted/40 p-3">
      <div className="mb-3">
        <h3 className="font-semibold text-sm">
          {t("pages.performance.customizePanels")}
        </h3>
        <p className="text-muted-foreground text-xs">
          {t("pages.performance.customizePanelsDescription")}
        </p>
      </div>
      <DndContext
        collisionDetection={closestCenter}
        sensors={sensors}
        onDragEnd={onPanelDragEnd}
      >
        <SortableContext
          items={layout.order}
          strategy={verticalListSortingStrategy}
        >
          <div className="grid gap-2 lg:grid-cols-3">
            {layout.order.map((panel) => (
              <SortablePanelControl
                key={panel}
                panel={panel}
                checked={layout.visible.includes(panel)}
                disableUncheck={
                  layout.visible.includes(panel) && layout.visible.length === 1
                }
                onPanelToggle={onPanelToggle}
              />
            ))}
          </div>
        </SortableContext>
      </DndContext>
    </section>
  );
};

const SortablePanelControl = ({
  panel,
  checked,
  disableUncheck,
  onPanelToggle,
}: {
  panel: PerformancePanelId;
  checked: boolean;
  disableUncheck: boolean;
  onPanelToggle: (panel: PerformancePanelId) => Promise<boolean>;
}) => {
  const { t } = useTranslation();
  const { attributes, listeners, setNodeRef, transform, transition } =
    useSortable({ id: panel });
  const label = t(`pages.performance.panels.${panel}`);

  return (
    <div
      ref={setNodeRef}
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
      }}
      className="flex min-h-11 items-center gap-2 rounded-lg border border-border bg-background/70 px-2"
      data-testid={`performance-panel-control-${panel}`}
    >
      <button
        {...attributes}
        {...listeners}
        type="button"
        className="flex size-8 shrink-0 cursor-grab items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        aria-label={t("pages.performance.reorderPanel", { panel: label })}
      >
        <GripVerticalIcon className="size-4" />
      </button>
      <Checkbox
        id={`performance-panel-${panel}`}
        checked={checked}
        disabled={disableUncheck}
        onCheckedChange={() => void onPanelToggle(panel)}
      />
      <Label
        htmlFor={`performance-panel-${panel}`}
        className="min-w-0 truncate text-sm"
      >
        {label}
      </Label>
    </div>
  );
};
