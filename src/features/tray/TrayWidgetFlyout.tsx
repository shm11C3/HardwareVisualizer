import { emit, listen } from "@tauri-apps/api/event";
import { Cpu, ExternalLink, Gpu, Thermometer, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

const EVENT_TRAY_WIDGET_FRAME = "tray-widget-frame";
const EVENT_TRAY_WIDGET_OPEN_MAIN_REQUESTED = "tray-widget-open-main-requested";
const EVENT_TRAY_WIDGET_HIDE_REQUESTED = "tray-widget-hide-requested";

type TrayMetric = "cpu" | "gpu" | "temp";
type TrayMetricState = "normal" | "warning" | "critical";

type TrayWidgetFlyoutItem = {
  metric: TrayMetric;
  value: string;
  state: TrayMetricState;
};

type TrayWidgetFlyoutFrame = {
  items: TrayWidgetFlyoutItem[];
  tooltip: string;
};

const INITIAL_FRAME: TrayWidgetFlyoutFrame = {
  items: [],
  tooltip: "HardwareVisualizer",
};

export const TrayWidgetFlyout = () => {
  const [frame, setFrame] = useState<TrayWidgetFlyoutFrame>(INITIAL_FRAME);
  const worstState = useMemo(() => getWorstState(frame.items), [frame.items]);

  useEffect(() => {
    let cleanup: (() => void) | undefined;
    let disposed = false;

    listen<TrayWidgetFlyoutFrame>(EVENT_TRAY_WIDGET_FRAME, (event) => {
      setFrame(event.payload);
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        cleanup = unlisten;
      })
      .catch((error) => {
        console.error("Failed to listen for tray widget frame:", error);
      });

    return () => {
      disposed = true;
      cleanup?.();
    };
  }, []);

  return (
    <main className="flex h-screen w-screen flex-col overflow-hidden border border-zinc-700/80 bg-zinc-950 px-3 py-2 text-zinc-100 shadow-2xl">
      <header className="flex h-8 shrink-0 items-center justify-between gap-2 border-zinc-800 border-b">
        <div className="flex min-w-0 items-center gap-2">
          <span
            className={`size-2 rounded-full ${stateDotClass[worstState]}`}
          />
          <h1 className="truncate font-medium text-[13px] leading-none">
            HardwareVisualizer
          </h1>
        </div>
        <div className="flex items-center gap-1">
          <button
            aria-label="Open main window"
            className="flex size-7 items-center justify-center rounded text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-50"
            onClick={() => emit(EVENT_TRAY_WIDGET_OPEN_MAIN_REQUESTED)}
            type="button"
          >
            <ExternalLink className="size-4" />
          </button>
          <button
            aria-label="Close tray widget"
            className="flex size-7 items-center justify-center rounded text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-50"
            onClick={() => emit(EVENT_TRAY_WIDGET_HIDE_REQUESTED)}
            type="button"
          >
            <X className="size-4" />
          </button>
        </div>
      </header>

      <section className="grid flex-1 content-center gap-1 py-2">
        {frame.items.length > 0 ? (
          frame.items.map((item) => <MetricRow item={item} key={item.metric} />)
        ) : (
          <p className="px-1 text-center text-sm text-zinc-400">
            No tray metrics available
          </p>
        )}
      </section>
    </main>
  );
};

const MetricRow = ({ item }: { item: TrayWidgetFlyoutItem }) => {
  const Icon = metricIcon[item.metric];

  return (
    <div className="grid min-h-8 grid-cols-[28px_1fr_auto] items-center gap-2 rounded px-1.5">
      <div className="flex size-7 items-center justify-center rounded bg-zinc-900 text-zinc-300">
        <Icon className="size-4" />
      </div>
      <span className="truncate text-[13px] text-zinc-400">
        {metricLabel[item.metric]}
      </span>
      <strong
        className={`font-semibold text-base tabular-nums leading-none ${stateTextClass[item.state]}`}
      >
        {item.value}
      </strong>
    </div>
  );
};

const metricLabel: Record<TrayMetric, string> = {
  cpu: "CPU",
  gpu: "GPU",
  temp: "Temp",
};

const metricIcon = {
  cpu: Cpu,
  gpu: Gpu,
  temp: Thermometer,
};

const stateTextClass: Record<TrayMetricState, string> = {
  normal: "text-zinc-50",
  warning: "text-amber-400",
  critical: "text-red-400",
};

const stateDotClass: Record<TrayMetricState, string> = {
  normal: "bg-zinc-400",
  warning: "bg-amber-400",
  critical: "bg-red-500",
};

const stateRank: Record<TrayMetricState, number> = {
  normal: 0,
  warning: 1,
  critical: 2,
};

const getWorstState = (items: TrayWidgetFlyoutItem[]): TrayMetricState => {
  return items.reduce<TrayMetricState>(
    (worst, item) =>
      stateRank[item.state] > stateRank[worst] ? item.state : worst,
    "normal",
  );
};
