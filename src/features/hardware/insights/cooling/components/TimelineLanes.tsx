import { useTranslation } from "react-i18next";
import {
  Area,
  Bar,
  CartesianGrid,
  ComposedChart,
  Line,
  ReferenceArea,
  ReferenceLine,
  XAxis,
  YAxis,
} from "recharts";
import {
  type ChartConfig,
  ChartContainer,
  ChartTooltip,
} from "@/components/ui/chart";
import type { TemperatureUnit } from "@/rspc/bindings";
import type {
  BaselineBand,
  ThermalTimelineRow,
} from "../utils/thermalTimeline";

/**
 * What the lower lane shows, which differs by period because the two data
 * sources answer different questions:
 * - `usage`: the archive's bucket-average CPU usage (24h/7d/30d).
 * - `composition`: how the day was split across the four load bands, from
 *   the daily rollup's per-band sample minutes (90d/1y). The rollup has no
 *   intra-day usage series, so a daily "average load" line cannot be drawn
 *   from it without inventing one.
 */
export type LoadLaneMode = "usage" | "composition";

/** Every lane shares this id so Recharts keeps their cursors in step. */
const TIMELINE_SYNC_ID = "cooling-thermal-timeline";

/** Identical on both charts so the two plot areas line up horizontally. */
const LANE_MARGIN = { top: 8, right: 8, bottom: 0, left: 0 } as const;
const AXIS_WIDTH = 44;

const LOAD_LANE_DOMAIN: [number, number] = [0, 100];

const chartConfig = {
  temperatureRange: {
    label: "range",
    color: "hsl(var(--chart-1))",
  },
  temperatureAvg: {
    label: "avg",
    color: "hsl(var(--chart-1))",
  },
  idleTemperature: {
    label: "idle",
    color: "hsl(var(--chart-2))",
  },
  cpuUsage: {
    label: "cpu",
    color: "hsl(var(--chart-4))",
  },
  // `--chart-1` stays reserved for the temperature series, so the load
  // bands take the remaining tokens rather than reusing the lane color that
  // already means "temperature" one lane above.
  loadIdle: {
    label: "idle",
    color: "hsl(var(--chart-2))",
  },
  loadLow: {
    label: "low",
    color: "hsl(var(--chart-3))",
  },
  loadMid: {
    label: "mid",
    color: "hsl(var(--chart-4))",
  },
  loadHigh: {
    label: "high",
    color: "hsl(var(--chart-5))",
  },
  // `--chart-1` means temperature and `--chart-4` already means CPU load
  // in the lane directly above, so power takes the remaining token rather
  // than borrowing a meaning the reader has already learned.
  powerRange: {
    label: "range",
    color: "hsl(var(--chart-3))",
  },
  powerAvg: {
    label: "avg",
    color: "hsl(var(--chart-3))",
  },
} satisfies ChartConfig;

type SeriesKey = keyof typeof chartConfig;

const seriesColor = (key: SeriesKey) => `var(--color-${key})`;

const LOAD_BAND_SERIES = [
  { key: "loadIdle", band: "idle" },
  { key: "loadLow", band: "low" },
  { key: "loadMid", band: "mid" },
  { key: "loadHigh", band: "high" },
] as const satisfies readonly { key: SeriesKey; band: string }[];

const LegendSwatch = ({
  label,
  color,
  variant = "line",
}: {
  label: string;
  color: string;
  variant?: "line" | "band" | "bar" | "dashed";
}) => (
  <span className="flex items-center gap-1.5 text-muted-foreground text-xs">
    {variant === "band" ? (
      <span
        className="h-2.5 w-3.5 rounded-[2px] opacity-30"
        style={{ backgroundColor: color }}
      />
    ) : variant === "bar" ? (
      <span
        className="h-2.5 w-2.5 rounded-[2px]"
        style={{ backgroundColor: color }}
      />
    ) : variant === "dashed" ? (
      <span
        className="h-0 w-3.5 border-t-2 border-dashed"
        style={{ borderColor: color }}
      />
    ) : (
      <span
        className="h-0.5 w-3.5 rounded-full"
        style={{ backgroundColor: color }}
      />
    )}
    {label}
  </span>
);

type TooltipRenderProps = {
  active?: boolean;
  payload?: { payload?: ThermalTimelineRow }[];
};

/**
 * The one shared tooltip. Both lanes are driven by the same rows, so the
 * temperature lane renders the whole story for the hovered period and the
 * load lane only draws the synchronized cursor.
 */
const TimelineTooltipContent = ({
  active,
  payload,
  unitSuffix,
  baseline,
  loadMode,
}: TooltipRenderProps & {
  unitSuffix: string;
  baseline: BaselineBand | null;
  loadMode: LoadLaneMode;
}) => {
  const { t } = useTranslation();

  if (!active || !payload?.length) {
    return null;
  }

  const row = payload[0]?.payload;
  if (!row) {
    return null;
  }

  const temperature = (value: number | null) =>
    value == null ? null : `${value.toFixed(1)}${unitSuffix}`;
  const percent = (value: number | null) =>
    value == null ? null : `${value.toFixed(0)}%`;
  const watts = (value: number | null) =>
    value == null ? null : `${value.toFixed(1)} W`;

  const baselineDelta =
    baseline == null || row.idleTemperature == null
      ? null
      : row.idleTemperature - baseline.value;

  const hasAnyValue =
    row.temperatureAvg != null ||
    row.temperatureRange != null ||
    row.idleTemperature != null ||
    row.cpuUsage != null ||
    row.loadIdle != null ||
    row.powerAvg != null;

  const entries: { label: string; value: string }[] = [];
  const push = (label: string, value: string | null) => {
    if (value != null) {
      entries.push({ label, value });
    }
  };

  push(
    t("pages.insights.cooling.timeline.tooltip.average"),
    temperature(row.temperatureAvg),
  );
  push(
    t("pages.insights.cooling.timeline.tooltip.range"),
    row.temperatureRange == null
      ? null
      : `${row.temperatureRange[0].toFixed(1)} - ${row.temperatureRange[1].toFixed(1)}${unitSuffix}`,
  );
  push(
    t("pages.insights.cooling.timeline.tooltip.idle"),
    temperature(row.idleTemperature),
  );
  push(
    t("pages.insights.cooling.timeline.tooltip.baselineDelta"),
    baselineDelta == null
      ? null
      : `${baselineDelta >= 0 ? "+" : ""}${baselineDelta.toFixed(1)}${unitSuffix}`,
  );

  if (loadMode === "usage") {
    push(
      t("pages.insights.cooling.timeline.tooltip.cpuUsage"),
      percent(row.cpuUsage),
    );
  } else {
    for (const series of LOAD_BAND_SERIES) {
      push(
        t(`pages.insights.cooling.loadBands.${series.band}`),
        percent(row[series.key]),
      );
    }
  }

  push(t("pages.insights.cooling.timeline.tooltip.power"), watts(row.powerAvg));
  push(
    t("pages.insights.cooling.timeline.tooltip.powerRange"),
    row.powerRange == null
      ? null
      : `${row.powerRange[0].toFixed(1)} - ${row.powerRange[1].toFixed(1)} W`,
  );

  return (
    <div className="grid min-w-[10rem] gap-1 rounded-lg border border-neutral-200/50 bg-white px-2.5 py-1.5 text-xs shadow-xl dark:border-neutral-800/50 dark:bg-neutral-950">
      <div className="font-medium">{row.label}</div>
      {hasAnyValue ? (
        entries.map((entry) => (
          <div
            key={entry.label}
            className="flex items-center justify-between gap-3"
          >
            <span className="text-neutral-500 dark:text-neutral-400">
              {entry.label}
            </span>
            <span className="font-medium font-mono tabular-nums">
              {entry.value}
            </span>
          </div>
        ))
      ) : (
        <span className="text-neutral-500 dark:text-neutral-400">
          {t("pages.insights.cooling.timeline.tooltip.noRecording")}
        </span>
      )}
    </div>
  );
};

/**
 * The synchronized thermal timeline: a tall temperature lane over a short
 * CPU-load lane on one shared category axis.
 *
 * Both charts read the same `rows`, carry the same `syncId`, and reserve the
 * same axis width, so the cursor, the x positions, and the gaps line up. No
 * series uses `connectNulls`, so an unrecorded period is a gap in both lanes
 * instead of a line drawn straight through it.
 */
const TemperatureLaneChart = ({
  rows,
  domain,
  baseline,
  temperatureUnit,
  loadMode,
}: {
  rows: ThermalTimelineRow[];
  domain: [number, number];
  baseline: BaselineBand | null;
  temperatureUnit: TemperatureUnit;
  loadMode: LoadLaneMode;
}) => {
  const unitSuffix = temperatureUnit === "C" ? "°C" : "°F";
  const hasIdleSeries = rows.some((row) => row.idleTemperature != null);

  return (
    <ChartContainer
      className="aspect-auto h-50 w-full"
      config={chartConfig}
      data-testid="cooling-temperature-lane"
    >
      <ComposedChart data={rows} syncId={TIMELINE_SYNC_ID} margin={LANE_MARGIN}>
        <CartesianGrid horizontal vertical={false} />
        <XAxis dataKey="label" hide />
        <YAxis
          domain={domain}
          width={AXIS_WIDTH}
          tickLine={false}
          axisLine={false}
          tickCount={6}
          allowDecimals={false}
          unit={unitSuffix}
        />
        {baseline != null && (
          <ReferenceArea
            y1={baseline.lower}
            y2={baseline.upper}
            fill="var(--muted-foreground)"
            fillOpacity={0.12}
            ifOverflow="hidden"
          />
        )}
        {baseline != null && (
          <ReferenceLine
            y={baseline.value}
            stroke="var(--muted-foreground)"
            strokeDasharray="4 4"
            ifOverflow="hidden"
          />
        )}
        <ChartTooltip
          filterNull={false}
          content={
            <TimelineTooltipContent
              unitSuffix={unitSuffix}
              baseline={baseline}
              loadMode={loadMode}
            />
          }
        />
        <Area
          dataKey="temperatureRange"
          stroke={seriesColor("temperatureRange")}
          strokeOpacity={0.4}
          strokeWidth={1}
          fill={seriesColor("temperatureRange")}
          fillOpacity={0.18}
          isAnimationActive={false}
          activeDot={false}
        />
        <Line
          dataKey="temperatureAvg"
          type="monotone"
          stroke={seriesColor("temperatureAvg")}
          strokeWidth={2}
          dot={false}
          isAnimationActive={false}
        />
        {hasIdleSeries && (
          <Line
            dataKey="idleTemperature"
            type="monotone"
            stroke={seriesColor("idleTemperature")}
            strokeWidth={1.5}
            dot={false}
            isAnimationActive={false}
          />
        )}
      </ComposedChart>
    </ChartContainer>
  );
};

/**
 * The third lane: CPU package power draw, in watts.
 *
 * Deliberately shorter than the load lane. It answers "how much electrical
 * input produced the temperature above", which is context for the
 * temperature lane rather than a reading to scrub in its own right - and a
 * third full-height lane would push the load-band comparison below the
 * fold.
 *
 * Only mounted when the period actually recorded power (see
 * `hasPowerReadings`): rendering an empty axis on a machine with no CPU
 * power source would read as a measured flat zero.
 */
const PowerLaneChart = ({
  rows,
  domain,
}: {
  rows: ThermalTimelineRow[];
  domain: [number, number];
}) => {
  const hasRangeSeries = rows.some((row) => row.powerRange != null);

  return (
    <ChartContainer
      className="aspect-auto h-24 w-full"
      config={chartConfig}
      data-testid="cooling-power-lane"
    >
      <ComposedChart data={rows} syncId={TIMELINE_SYNC_ID} margin={LANE_MARGIN}>
        {/* The shared time axis is labeled on whichever lane is last, so
            it reads as the stack's axis rather than a divider between
            two lanes. */}
        <XAxis
          dataKey="label"
          tickLine={false}
          axisLine={false}
          minTickGap={32}
          height={18}
        />
        <YAxis
          domain={domain}
          width={AXIS_WIDTH}
          tickLine={false}
          axisLine={false}
          tickCount={3}
          allowDecimals={false}
          unit="W"
        />
        {/* Cursor only - the shared tooltip is rendered by the temperature lane. */}
        <ChartTooltip filterNull={false} content={() => null} />
        {hasRangeSeries && (
          <Area
            dataKey="powerRange"
            stroke={seriesColor("powerRange")}
            strokeOpacity={0.4}
            strokeWidth={1}
            fill={seriesColor("powerRange")}
            fillOpacity={0.18}
            isAnimationActive={false}
            activeDot={false}
          />
        )}
        <Line
          dataKey="powerAvg"
          type="monotone"
          stroke={seriesColor("powerAvg")}
          strokeWidth={1.5}
          dot={false}
          isAnimationActive={false}
        />
      </ComposedChart>
    </ChartContainer>
  );
};

export const TimelineLanes = ({
  rows,
  domain,
  powerDomain,
  baseline,
  loadMode,
  temperatureUnit,
}: {
  rows: ThermalTimelineRow[];
  /**
   * `null` when the period recorded no temperature at all: the
   * temperature lane then degrades to an honest notice while the load
   * lane below keeps rendering - archived CPU usage without a working
   * temperature sensor is still useful partial data (DP-02).
   */
  domain: [number, number] | null;
  /**
   * `null` when the period recorded no CPU package power - either the
   * machine has no such source, or none was archived yet. The power lane
   * is then not rendered at all rather than degrading to a notice: unlike
   * temperature it is not what this view is primarily about, so its
   * absence belongs in the pending-sensors note, not in the timeline.
   */
  powerDomain: [number, number] | null;
  baseline: BaselineBand | null;
  loadMode: LoadLaneMode;
  temperatureUnit: TemperatureUnit;
}) => {
  const { t } = useTranslation();
  const unitSuffix = temperatureUnit === "C" ? "°C" : "°F";
  const hasIdleSeries = rows.some((row) => row.idleTemperature != null);
  const showsPowerLane = powerDomain != null;

  return (
    <div className="space-y-2" data-testid="cooling-timeline-lanes">
      {domain == null ? (
        <p
          className="text-muted-foreground text-sm"
          data-testid="cooling-temperature-lane-unavailable"
        >
          {t("pages.insights.cooling.timeline.temperatureUnavailable")}
        </p>
      ) : (
        <>
          <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
            <span className="font-medium text-sm">
              {t("pages.insights.cooling.timeline.temperatureLane", {
                unit: unitSuffix,
              })}
            </span>
            <LegendSwatch
              label={t("pages.insights.cooling.timeline.legend.average")}
              color={seriesColor("temperatureAvg")}
            />
            <LegendSwatch
              label={t("pages.insights.cooling.timeline.legend.range")}
              color={seriesColor("temperatureRange")}
              variant="band"
            />
            {hasIdleSeries && (
              <LegendSwatch
                label={t("pages.insights.cooling.timeline.legend.idle")}
                color={seriesColor("idleTemperature")}
              />
            )}
            {baseline != null && (
              <LegendSwatch
                label={t("pages.insights.cooling.timeline.legend.baseline")}
                color="var(--muted-foreground)"
                variant="dashed"
              />
            )}
          </div>

          <TemperatureLaneChart
            rows={rows}
            domain={domain}
            baseline={baseline}
            temperatureUnit={temperatureUnit}
            loadMode={loadMode}
          />
        </>
      )}

      <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
        <span className="font-medium text-muted-foreground text-xs">
          {t(
            loadMode === "usage"
              ? "pages.insights.cooling.timeline.loadLaneUsage"
              : "pages.insights.cooling.timeline.loadLaneComposition",
          )}
        </span>
        {loadMode === "composition" &&
          LOAD_BAND_SERIES.map((series) => (
            <LegendSwatch
              key={series.key}
              label={t(`pages.insights.cooling.loadBands.${series.band}`)}
              color={seriesColor(series.key)}
              variant="bar"
            />
          ))}
      </div>

      <ChartContainer
        className="aspect-auto h-22 w-full"
        config={chartConfig}
        data-testid="cooling-load-lane"
      >
        <ComposedChart
          data={rows}
          syncId={TIMELINE_SYNC_ID}
          margin={LANE_MARGIN}
        >
          {showsPowerLane ? (
            <XAxis dataKey="label" hide />
          ) : (
            <XAxis
              dataKey="label"
              tickLine={false}
              axisLine={false}
              minTickGap={32}
              height={18}
            />
          )}
          <YAxis
            domain={LOAD_LANE_DOMAIN}
            width={AXIS_WIDTH}
            tick={false}
            tickLine={false}
            axisLine={false}
          />
          {/* Cursor only - the shared tooltip is rendered by the lane above. */}
          <ChartTooltip filterNull={false} content={() => null} />
          {loadMode === "usage" ? (
            <Area
              dataKey="cpuUsage"
              type="monotone"
              stroke={seriesColor("cpuUsage")}
              strokeWidth={1.5}
              fill={seriesColor("cpuUsage")}
              fillOpacity={0.25}
              isAnimationActive={false}
              activeDot={false}
            />
          ) : (
            LOAD_BAND_SERIES.map((series) => (
              <Bar
                key={series.key}
                dataKey={series.key}
                stackId="load"
                fill={seriesColor(series.key)}
                isAnimationActive={false}
              />
            ))
          )}
        </ComposedChart>
      </ChartContainer>

      {showsPowerLane && (
        <>
          <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
            <span className="font-medium text-muted-foreground text-xs">
              {t("pages.insights.cooling.timeline.powerLane")}
            </span>
            <LegendSwatch
              label={t("pages.insights.cooling.timeline.legend.average")}
              color={seriesColor("powerAvg")}
            />
            <LegendSwatch
              label={t("pages.insights.cooling.timeline.legend.range")}
              color={seriesColor("powerRange")}
              variant="band"
            />
          </div>
          <PowerLaneChart rows={rows} domain={powerDomain} />
        </>
      )}
    </div>
  );
};
