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
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { platform } from "@tauri-apps/plugin-os";
import { AlertTriangleIcon, GripVerticalIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { useTauriStore } from "@/hooks/useTauriStore";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

export type TrayMetric = "cpu" | "gpu" | "gpu-temp";

export type TrayWidgetStore = {
  enabled: boolean;
  metricOrder: TrayMetric[];
  visibleMetrics: TrayMetric[];
  updateIntervalSecs: number;
  metrics?: TrayMetric[];
};

type PersistedTrayWidgetStore = Partial<TrayWidgetStore>;

const DEFAULT_TRAY_WIDGET_SETTINGS: TrayWidgetStore = {
  enabled: false,
  metricOrder: ["cpu", "gpu", "gpu-temp"],
  visibleMetrics: ["cpu", "gpu", "gpu-temp"],
  updateIntervalSecs: 1,
};

const CONFIGURABLE_METRICS: TrayMetric[] = ["cpu", "gpu", "gpu-temp"];
const MACOS_UNAVAILABLE_METRICS: TrayMetric[] = ["gpu-temp"];
const UPDATE_INTERVALS = [1, 2, 5] as const;

export const TrayWidgetSettings = () => {
  const { t } = useTranslation();
  const unavailableMetrics =
    platform() === "macos" ? MACOS_UNAVAILABLE_METRICS : [];
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );
  const [settings, setSettings, isPending] =
    useTauriStore<PersistedTrayWidgetStore>(
      "trayWidget",
      DEFAULT_TRAY_WIDGET_SETTINGS,
    );
  const [isAvailable, setIsAvailable] = useState(true);
  const normalizedSettings = normalizeSettings(settings, unavailableMetrics);

  useEffect(() => {
    const checkAvailability = async () => {
      try {
        const result = await commands.isCloseToTrayAvailable();

        if (isError(result)) {
          setIsAvailable(false);
          console.error(
            "Failed to check tray widget availability:",
            result.error,
          );
          return;
        }

        setIsAvailable(result.data);
      } catch (err) {
        setIsAvailable(false);
        console.error("Failed to check tray widget availability:", err);
      }
    };

    checkAvailability();
  }, []);

  const updateSettings = async (patch: Partial<TrayWidgetStore>) => {
    if (!normalizedSettings) {
      return;
    }

    await setSettings({ ...normalizedSettings, ...patch });
  };

  const toggleMetric = async (metric: TrayMetric, checked: boolean) => {
    if (!normalizedSettings) {
      return;
    }

    const visibleMetrics = checked
      ? [...normalizedSettings.visibleMetrics, metric].filter(
          (value, index, self) => self.indexOf(value) === index,
        )
      : normalizedSettings.visibleMetrics.filter((value) => value !== metric);

    await updateSettings({
      visibleMetrics:
        visibleMetrics.length > 0
          ? visibleMetrics
          : normalizedSettings.visibleMetrics,
    });
  };

  const handleDragEnd = async (event: DragEndEvent) => {
    if (!normalizedSettings) {
      return;
    }

    const { active, over } = event;

    if (!over || active.id === over.id) {
      return;
    }

    const oldIndex = normalizedSettings.metricOrder.indexOf(
      active.id as TrayMetric,
    );
    const newIndex = normalizedSettings.metricOrder.indexOf(
      over.id as TrayMetric,
    );

    if (oldIndex === -1 || newIndex === -1) {
      return;
    }

    await updateSettings({
      metricOrder: arrayMove(
        normalizedSettings.metricOrder,
        oldIndex,
        newIndex,
      ),
    });
  };

  const enabled = isAvailable && (normalizedSettings?.enabled ?? false);
  const metricOrder =
    normalizedSettings?.metricOrder ?? DEFAULT_TRAY_WIDGET_SETTINGS.metricOrder;
  const visibleMetrics =
    normalizedSettings?.visibleMetrics ??
    DEFAULT_TRAY_WIDGET_SETTINGS.visibleMetrics;

  return (
    <div className="space-y-4 border-[var(--border)] border-t py-6 xl:w-1/2">
      <div className="flex w-full items-center justify-between gap-4">
        <div className="space-y-1">
          <Label htmlFor="trayWidgetEnabled" className="text-lg">
            {t("pages.settings.general.trayWidget.name")}
          </Label>
          <p className="text-muted-foreground text-sm">
            {t("pages.settings.general.trayWidget.description")}
          </p>
        </div>

        <Switch
          id="trayWidgetEnabled"
          checked={enabled}
          disabled={isPending || !isAvailable}
          onCheckedChange={(value) => updateSettings({ enabled: value })}
        />
      </div>

      {!isAvailable && (
        <div className="flex items-start gap-2 rounded-md border border-yellow-500/40 bg-yellow-500/10 p-3 text-sm">
          <AlertTriangleIcon className="mt-0.5 size-4 shrink-0 text-yellow-600" />
          <p>{t("pages.settings.general.trayWidget.unavailable")}</p>
        </div>
      )}

      {enabled && (
        <div className="grid gap-4 sm:grid-cols-[1fr_180px]">
          <div className="space-y-3">
            <Label className="text-sm">
              {t("pages.settings.general.trayWidget.metrics")}
            </Label>

            <div className="space-y-2">
              <DndContext
                collisionDetection={closestCenter}
                onDragEnd={handleDragEnd}
                sensors={sensors}
              >
                <SortableContext
                  items={metricOrder}
                  strategy={verticalListSortingStrategy}
                >
                  {metricOrder.map((metric) => (
                    <SortableMetricRow
                      checked={visibleMetrics.includes(metric)}
                      disabled={isPending}
                      disableUncheck={
                        visibleMetrics.includes(metric) &&
                        visibleMetrics.length === 1
                      }
                      key={metric}
                      metric={metric}
                      onCheckedChange={toggleMetric}
                    />
                  ))}
                </SortableContext>
              </DndContext>
            </div>
          </div>

          <div className="space-y-2">
            <div className="space-y-2">
              <Label htmlFor="trayWidgetInterval" className="text-sm">
                {t("pages.settings.general.trayWidget.updateInterval")}
              </Label>
              <Select
                disabled={isPending}
                onValueChange={(value) =>
                  updateSettings({ updateIntervalSecs: Number(value) })
                }
                value={String(normalizedSettings?.updateIntervalSecs ?? 1)}
              >
                <SelectTrigger id="trayWidgetInterval">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {UPDATE_INTERVALS.map((interval) => (
                    <SelectItem key={interval} value={String(interval)}>
                      {t("pages.settings.general.trayWidget.seconds", {
                        count: interval,
                      })}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

type SortableMetricRowProps = {
  checked: boolean;
  disabled: boolean;
  disableUncheck: boolean;
  metric: TrayMetric;
  onCheckedChange: (metric: TrayMetric, checked: boolean) => Promise<void>;
};

const SortableMetricRow = ({
  checked,
  disabled,
  disableUncheck,
  metric,
  onCheckedChange,
}: SortableMetricRowProps) => {
  const { t } = useTranslation();
  const { attributes, listeners, setNodeRef, transform, transition } =
    useSortable({ id: metric, disabled });
  const metricName = t(`pages.settings.general.trayWidget.metric.${metric}`);

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div
      className="flex min-h-10 items-center justify-between gap-3 rounded-md border border-transparent px-2 transition-colors data-[dragging=true]:border-[var(--border)] data-[dragging=true]:bg-[var(--muted)]"
      data-dragging={transform != null}
      ref={setNodeRef}
      style={style}
    >
      <div className="flex items-center gap-3">
        <button
          {...attributes}
          {...listeners}
          aria-label={t("pages.settings.general.trayWidget.dragMetric", {
            metric: metricName,
          })}
          className="flex size-8 cursor-grab items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-[var(--muted)] hover:text-[var(--foreground)] disabled:cursor-default disabled:opacity-50"
          disabled={disabled}
          type="button"
        >
          <GripVerticalIcon className="size-4" />
        </button>
        <Checkbox
          id={`trayWidgetMetric-${metric}`}
          checked={checked}
          disabled={disabled || disableUncheck}
          onCheckedChange={(value) => onCheckedChange(metric, value === true)}
        />
        <Label htmlFor={`trayWidgetMetric-${metric}`} className="text-sm">
          {metricName}
        </Label>
      </div>
    </div>
  );
};

export const normalizeSettings = (
  settings: PersistedTrayWidgetStore | null,
  unavailableMetrics: TrayMetric[] = [],
): TrayWidgetStore | null => {
  if (!settings) {
    return null;
  }

  const legacyMetrics = normalizeMetricList(settings.metrics ?? []);
  const metricOrder = normalizeMetricOrder(
    settings.metricOrder?.length ? settings.metricOrder : legacyMetrics,
    unavailableMetrics,
  );
  const visibleMetrics = normalizeVisibleMetrics(
    settings.visibleMetrics?.length ? settings.visibleMetrics : legacyMetrics,
    metricOrder,
    unavailableMetrics,
  );
  const updateIntervalSecs = settings.updateIntervalSecs;

  return {
    enabled: settings.enabled ?? DEFAULT_TRAY_WIDGET_SETTINGS.enabled,
    metricOrder,
    visibleMetrics,
    updateIntervalSecs:
      typeof updateIntervalSecs === "number" &&
      UPDATE_INTERVALS.includes(
        updateIntervalSecs as (typeof UPDATE_INTERVALS)[number],
      )
        ? updateIntervalSecs
        : DEFAULT_TRAY_WIDGET_SETTINGS.updateIntervalSecs,
  };
};

export const normalizeMetricOrder = (
  metrics: TrayMetric[],
  unavailableMetrics: TrayMetric[] = [],
) => {
  const metricOrder = normalizeMetricList(metrics).filter(
    (metric) => !unavailableMetrics.includes(metric),
  );

  for (const metric of CONFIGURABLE_METRICS) {
    if (!unavailableMetrics.includes(metric) && !metricOrder.includes(metric)) {
      metricOrder.push(metric);
    }
  }

  return metricOrder;
};

export const normalizeVisibleMetrics = (
  metrics: TrayMetric[],
  metricOrder: TrayMetric[],
  unavailableMetrics: TrayMetric[] = [],
) => {
  const visibleMetrics = normalizeMetricList(metrics).filter(
    (metric) =>
      metricOrder.includes(metric) && !unavailableMetrics.includes(metric),
  );
  const fallbackMetrics = metricOrder.filter(
    (metric) => !unavailableMetrics.includes(metric),
  );

  return visibleMetrics.length > 0 ? visibleMetrics : fallbackMetrics;
};

const normalizeMetricList = (metrics: TrayMetric[]) => {
  const normalized: TrayMetric[] = [];

  for (const metric of metrics) {
    if (CONFIGURABLE_METRICS.includes(metric) && !normalized.includes(metric)) {
      normalized.push(metric);
    }
  }

  return normalized;
};
