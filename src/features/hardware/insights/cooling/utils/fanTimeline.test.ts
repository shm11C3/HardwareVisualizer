import { describe, expect, it } from "vitest";
import type { CoolingFanTrendSeries, FanArchiveSeries } from "@/rspc/bindings";
import {
  buildFanLaneRows,
  claimsFanUnsupported,
  computeFanDomain,
  type FanLaneRow,
  fanDataKey,
  resolveFanSeries,
  resolveRoutedFanCapability,
  toArchiveFanSeries,
  toDailyFanSeries,
} from "./fanTimeline";
import type {
  ArchiveTimelineSeries,
  ThermalTimelineRow,
} from "./thermalTimeline";

const EMPTY_ROW = {
  temperatureAvg: null,
  temperatureMin: null,
  temperatureMax: null,
  temperatureRange: null,
  idleTemperature: null,
  cpuUsage: null,
  loadIdle: null,
  loadLow: null,
  loadMid: null,
  loadHigh: null,
  powerAvg: null,
  powerMin: null,
  powerMax: null,
  powerRange: null,
} as const satisfies Omit<ThermalTimelineRow, "key" | "label">;

const row = (key: string, label = key): ThermalTimelineRow => ({
  ...EMPTY_ROW,
  key,
  label,
});

const laneRow = (key: string, values: Record<string, number | null>) =>
  ({ key, label: key, values }) satisfies FanLaneRow;

describe("resolveFanSeries", () => {
  it("assigns positional keys ordered by source", () => {
    expect(resolveFanSeries(["Fan 2", "Fan 1"])).toEqual([
      { source: "Fan 1", key: "fan0" },
      { source: "Fan 2", key: "fan1" },
    ]);
  });

  it("keeps each fan's key stable regardless of the input order", () => {
    // The lane colors series by position, so a reordered input must not
    // silently recolor every fan between refreshes.
    expect(resolveFanSeries(["Fan 1", "Fan 2"])).toEqual(
      resolveFanSeries(["Fan 2", "Fan 1"]),
    );
  });

  it("has no series for a machine with no readable fan", () => {
    expect(resolveFanSeries([])).toEqual([]);
  });

  it("addresses a series through the values object rather than the source name", () => {
    // A source name containing a dot would otherwise make Recharts resolve
    // the dataKey as a nested path that does not exist.
    expect(fanDataKey({ source: "SYS.FAN1", key: "fan0" })).toBe("values.fan0");
  });
});

describe("buildFanLaneRows", () => {
  const series = resolveFanSeries(["Fan 1", "Fan 2"]);

  it("aligns every fan onto the timeline's own rows", () => {
    const rows = buildFanLaneRows(
      [row("a"), row("b")],
      [
        {
          source: "Fan 1",
          valueByRowKey: new Map([
            ["a", 900],
            ["b", 1100],
          ]),
        },
        { source: "Fan 2", valueByRowKey: new Map([["a", 1500]]) },
      ],
      series,
    );

    expect(rows).toEqual([
      laneRow("a", { fan0: 900, fan1: 1500 }),
      laneRow("b", { fan0: 1100, fan1: null }),
    ]);
  });

  it("keeps the same length and labels as the lanes above it", () => {
    // The synchronized cursor is index-based, so a fan lane with its own
    // shorter axis would put the cursor on a different period.
    const timelineRows = [row("a", "Mon"), row("b", "Tue"), row("c", "Wed")];

    const rows = buildFanLaneRows(timelineRows, [], series);

    expect(rows.map((entry) => entry.key)).toEqual(["a", "b", "c"]);
    expect(rows.map((entry) => entry.label)).toEqual(["Mon", "Tue", "Wed"]);
  });

  it("leaves an unrecorded period null rather than zero-filling it", () => {
    const rows = buildFanLaneRows(
      [row("a"), row("b"), row("c")],
      [
        {
          source: "Fan 1",
          valueByRowKey: new Map([
            ["a", 900],
            ["c", 950],
          ]),
        },
      ],
      resolveFanSeries(["Fan 1"]),
    );

    expect(rows.map((entry) => entry.values["fan0"])).toEqual([900, null, 950]);
  });

  it("keeps an Inactive Fan Reading as the real zero it is", () => {
    // 0 RPM means the fan is not reporting rotation - a measurement, not a
    // gap. Collapsing it into null would erase a stopped fan entirely.
    const rows = buildFanLaneRows(
      [row("a")],
      [{ source: "Fan 1", valueByRowKey: new Map([["a", 0]]) }],
      resolveFanSeries(["Fan 1"]),
    );

    expect(rows[0].values["fan0"]).toBe(0);
  });
});

describe("toArchiveFanSeries", () => {
  it("keys each fan by the archive bucket timestamp the rows use", () => {
    const series = toArchiveFanSeries([
      {
        source: "Fan 1",
        points: [
          { timestamp: 1000, value: 900 },
          { timestamp: 2000, value: null },
        ],
      },
    ]);

    expect(series[0].source).toBe("Fan 1");
    expect(series[0].valueByRowKey.get("1000")).toBe(900);
    expect(series[0].valueByRowKey.get("2000")).toBeNull();
  });
});

describe("toDailyFanSeries", () => {
  it("keys each fan by ISO date and carries the day's average", () => {
    const series = toDailyFanSeries([
      {
        source: "Fan 1",
        days: [
          {
            date: "2026-01-15",
            rpmAvg: 940,
            rpmMax: 1200,
            rpmMin: 800,
            sampleMinutes: 900,
          },
        ],
      },
    ]);

    expect(series[0].valueByRowKey.get("2026-01-15")).toBe(940);
  });

  it("leaves a day the fan never recorded out of the map", () => {
    const series = toDailyFanSeries([{ source: "Fan 1", days: [] }]);

    expect(series[0].valueByRowKey.size).toBe(0);
  });
});

describe("computeFanDomain", () => {
  it("anchors at zero so a stopped fan sits on the floor", () => {
    expect(computeFanDomain([laneRow("a", { fan0: 1000 })])).toEqual([0, 1100]);
  });

  it("pads by at least the minimum headroom for a slow fan", () => {
    expect(computeFanDomain([laneRow("a", { fan0: 300 })])).toEqual([0, 400]);
  });

  it("follows the fastest fan across every series", () => {
    expect(computeFanDomain([laneRow("a", { fan0: 600, fan1: 2000 })])).toEqual(
      [0, 2200],
    );
  });

  it("still renders a lane for a window of Inactive Fan Readings", () => {
    // A fan that reported 0 RPM all period is a real observation, not an
    // absent one: the lane must render rather than disappear.
    expect(computeFanDomain([laneRow("a", { fan0: 0 })])).toEqual([0, 100]);
  });

  it("is null when nothing in the window recorded a fan", () => {
    expect(computeFanDomain([laneRow("a", { fan0: null })])).toBeNull();
  });

  it("rounds the top of the axis to a readable hundred", () => {
    // 1770 + 10% is 1947; an axis tick of 1947 reads as a measurement of
    // its own rather than as a scale.
    expect(computeFanDomain([laneRow("a", { fan0: 1770 })])).toEqual([0, 2000]);
  });

  it("is null for an empty window", () => {
    expect(computeFanDomain([])).toBeNull();
  });
});

describe("resolveRoutedFanCapability", () => {
  const NO_CPU_SERIES: ArchiveTimelineSeries = {
    temperatureAvg: [],
    temperatureMax: [],
    temperatureMin: [],
    cpuUsage: [],
    powerAvg: [],
    powerMax: [],
    powerMin: [],
  };
  /** A window that recorded temperature, so "no fan" is real evidence. */
  const RECORDED: ArchiveTimelineSeries = {
    ...NO_CPU_SERIES,
    temperatureAvg: [{ timestamp: 0, value: 50 }],
  };
  const ARCHIVE = { kind: "archive" } as const;
  const DAILY = { kind: "dailyTrend" } as const;
  const WITH_FAN: FanArchiveSeries[] = [
    { source: "Fan 1", points: [{ timestamp: 0, value: 900 }] },
  ];
  const loadedArchive = (
    fanSeries: FanArchiveSeries[],
    cpuSeries: ArchiveTimelineSeries,
  ) => ({
    fanSeries,
    cpuSeries,
    hasLoaded: true,
    hasError: false,
    fanHasError: false,
  });
  const NO_DAILY = {
    fanSeries: null,
    archiveHasReadings: false,
    recordedDays: null,
    hasError: false,
  };
  const dailyWithFan: CoolingFanTrendSeries[] = [
    {
      source: "Fan 1",
      days: [
        {
          date: "2026-01-15",
          rpmAvg: 900,
          rpmMax: 1000,
          rpmMin: 800,
          sampleMinutes: 900,
        },
      ],
    },
  ];

  it("is present when the archive window carries a fan reading", () => {
    expect(
      resolveRoutedFanCapability(
        ARCHIVE,
        loadedArchive(WITH_FAN, RECORDED),
        NO_DAILY,
      ),
    ).toBe("present");
  });

  it("is present for a window of Inactive Fan Readings", () => {
    // 0 RPM is a reading; a machine reporting it plainly has a fan sensor.
    expect(
      resolveRoutedFanCapability(
        ARCHIVE,
        loadedArchive(
          [{ source: "Fan 1", points: [{ timestamp: 0, value: 0 }] }],
          RECORDED,
        ),
        NO_DAILY,
      ),
    ).toBe("present");
  });

  it("is absent when a recorded window carried no fan reading at all", () => {
    expect(
      resolveRoutedFanCapability(
        ARCHIVE,
        loadedArchive([], RECORDED),
        NO_DAILY,
      ),
    ).toBe("absent");
  });

  it("is unknown while the archive fetch is still in flight", () => {
    // The regression to avoid: claiming "not supported yet" here tells a
    // user with working fan sensors that their machine has none.
    expect(
      resolveRoutedFanCapability(
        ARCHIVE,
        {
          fanSeries: [],
          cpuSeries: NO_CPU_SERIES,
          hasLoaded: false,
          hasError: false,
          fanHasError: false,
        },
        NO_DAILY,
      ),
    ).toBe("unknown");
  });

  it("is unknown when the archive fetch failed", () => {
    expect(
      resolveRoutedFanCapability(
        ARCHIVE,
        {
          fanSeries: [],
          cpuSeries: RECORDED,
          hasLoaded: true,
          hasError: true,
          fanHasError: false,
        },
        NO_DAILY,
      ),
    ).toBe("unknown");
  });

  it("is unknown when only the fan read failed", () => {
    // The lanes above rendered from their own results, so the window is
    // plainly readable - but a failed fan read is not evidence the machine
    // has no fan, and must not be reported as one.
    expect(
      resolveRoutedFanCapability(
        ARCHIVE,
        {
          fanSeries: [],
          cpuSeries: RECORDED,
          hasLoaded: true,
          hasError: false,
          fanHasError: true,
        },
        NO_DAILY,
      ),
    ).toBe("unknown");
  });

  it("is unknown for a window that recorded nothing at all", () => {
    expect(
      resolveRoutedFanCapability(
        ARCHIVE,
        loadedArchive([], NO_CPU_SERIES),
        NO_DAILY,
      ),
    ).toBe("unknown");
  });

  it("counts a power-only window as recorded evidence", () => {
    // A machine with a power sampler and no readable temperature sensor
    // still archived this window, so a missing fan is real evidence.
    // Without power in the "recorded anything" test it stayed unknown
    // forever and the note never named the fan.
    expect(
      resolveRoutedFanCapability(
        ARCHIVE,
        loadedArchive([], {
          ...NO_CPU_SERIES,
          powerAvg: [{ timestamp: 0, value: 18 }],
        }),
        NO_DAILY,
      ),
    ).toBe("absent");
  });

  it("ignores the daily fan trend while an archive route is selected", () => {
    // Otherwise a 24h window on a machine whose fan sensor stopped months
    // ago would claim this window's lane is available.
    expect(
      resolveRoutedFanCapability(ARCHIVE, loadedArchive([], RECORDED), {
        fanSeries: dailyWithFan,
        archiveHasReadings: true,
        recordedDays: 90,
        hasError: false,
      }),
    ).toBe("absent");
  });

  it("reads the daily fan trend on the daily routes", () => {
    expect(
      resolveRoutedFanCapability(DAILY, loadedArchive(WITH_FAN, RECORDED), {
        fanSeries: dailyWithFan,
        archiveHasReadings: true,
        recordedDays: 90,
        hasError: false,
      }),
    ).toBe("present");
  });

  it("is absent when a recorded daily window summarized no fan and none is archived", () => {
    expect(
      resolveRoutedFanCapability(DAILY, loadedArchive([], NO_CPU_SERIES), {
        fanSeries: [],
        archiveHasReadings: false,
        recordedDays: 90,
        hasError: false,
      }),
    ).toBe("absent");
  });

  it("stays unknown right after an upgrade, while the rollup catches up", () => {
    // The regression: the migration creates the fan tables empty beside a
    // `cooling_daily_summary` already full of history, and the rollup only
    // summarizes completed days. Reading the empty fan trend as evidence
    // told every upgrading user with working fans "Not supported yet" for
    // up to a day.
    expect(
      resolveRoutedFanCapability(DAILY, loadedArchive([], NO_CPU_SERIES), {
        fanSeries: [],
        archiveHasReadings: true,
        recordedDays: 90,
        hasError: false,
      }),
    ).toBe("unknown");
  });

  it("stays unknown on the first day of fan collection", () => {
    // Same shape as the upgrade case and the same reason: the fan is
    // readable now, but no completed day carries it yet.
    expect(
      resolveRoutedFanCapability(DAILY, loadedArchive(WITH_FAN, RECORDED), {
        fanSeries: [],
        archiveHasReadings: true,
        recordedDays: 1,
        hasError: false,
      }),
    ).toBe("unknown");
  });

  it("is unknown when the daily window recorded no day at all", () => {
    expect(
      resolveRoutedFanCapability(DAILY, loadedArchive([], NO_CPU_SERIES), {
        fanSeries: [],
        archiveHasReadings: false,
        recordedDays: 0,
        hasError: false,
      }),
    ).toBe("unknown");
  });

  it("is unknown while the daily fan fetch is still in flight", () => {
    expect(
      resolveRoutedFanCapability(DAILY, loadedArchive([], NO_CPU_SERIES), {
        fanSeries: null,
        archiveHasReadings: false,
        recordedDays: 90,
        hasError: false,
      }),
    ).toBe("unknown");
  });

  it("is unknown when the daily fan fetch failed", () => {
    expect(
      resolveRoutedFanCapability(DAILY, loadedArchive([], NO_CPU_SERIES), {
        fanSeries: [],
        archiveHasReadings: false,
        recordedDays: 90,
        hasError: true,
      }),
    ).toBe("unknown");
  });
});

describe("claimsFanUnsupported", () => {
  it("names the fan unsupported only on evidence", () => {
    expect(claimsFanUnsupported("absent")).toBe(true);
  });

  it("stays silent while the answer is unknown", () => {
    expect(claimsFanUnsupported("unknown")).toBe(false);
    expect(claimsFanUnsupported("present")).toBe(false);
  });
});
