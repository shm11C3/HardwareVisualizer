import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { TemperatureUnit } from "@/rspc/bindings";
import {
  type LoadBandDumbbellRow,
  type LoadBandWithheldReason,
  positionPercent,
} from "../utils/loadBandDumbbell";
import { formatSignedTemperatureDelta } from "../utils/temperatureUnit";
import { computeSignedTemperatureDomain } from "../utils/thermalTimeline";

const NOT_COMPARABLE_KEY =
  "pages.insights.cooling.loadBandComparison.notComparable";

/**
 * Zone (5)'s per-band baseline-vs-recent comparison: a lightweight
 * CSS-drawn dumbbell (a track, two dots, a connecting bar) per load band,
 * following the same self-drawn approach `CoverageStrip` uses for its strip
 * of rectangles rather than pulling in a charting library for four short
 * rows.
 */
export const LoadBandDumbbellChart = ({
  rows,
  temperatureUnit,
  testId = "cooling-load-band-dumbbell",
}: {
  rows: LoadBandDumbbellRow[];
  temperatureUnit: TemperatureUnit;
  /**
   * Overridden by the ambient-adjusted variant (#2046), which renders a
   * second chart of the same shape directly below the absolute one; two
   * elements sharing one test id would make either ambiguous to address.
   */
  testId?: string;
}) => {
  const { t } = useTranslation();
  const unitSuffix = temperatureUnit === "C" ? "°C" : "°F";

  // Signed, because the same chart draws the ambient-adjusted variant
  // (#2046) whose endpoints are thermal deltas. Core does not clamp a ΔT at
  // zero - a machine idling below the room it sits in is a real
  // observation - and clamping the domain here would pin every negative
  // reading to the left end of the track instead of placing it.
  const domain = useMemo(
    () =>
      computeSignedTemperatureDomain(
        rows.flatMap((row) =>
          row.comparable ? [row.baseline, row.recent] : [],
        ),
      ),
    [rows],
  );

  return (
    <div className="space-y-3" data-testid={testId}>
      <div className="flex items-center gap-4 text-muted-foreground text-xs">
        <span className="flex items-center gap-1.5">
          <span className="h-2.5 w-2.5 rounded-full border-2 border-background bg-muted-foreground" />
          {t("pages.insights.cooling.loadBandComparison.legend.baseline")}
        </span>
        <span className="flex items-center gap-1.5">
          <span className="h-2.5 w-2.5 rounded-full border-2 border-background bg-primary" />
          {t("pages.insights.cooling.loadBandComparison.legend.recent")}
        </span>
      </div>

      {rows.map((row) => (
        <div
          key={row.band}
          className="grid grid-cols-[3.5rem_1fr_4.5rem] items-center gap-2"
        >
          <span className="text-xs">
            {t(`pages.insights.cooling.loadBands.${row.band}`)}
          </span>
          {row.comparable && domain != null ? (
            <div className="relative h-2 rounded-full bg-muted">
              <div
                className="absolute inset-y-0 rounded-full bg-primary/30"
                style={{
                  left: `${Math.min(
                    positionPercent(row.baseline, domain),
                    positionPercent(row.recent, domain),
                  )}%`,
                  width: `${Math.abs(
                    positionPercent(row.recent, domain) -
                      positionPercent(row.baseline, domain),
                  )}%`,
                }}
              />
              <span
                className="absolute top-1/2 h-2.5 w-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-background bg-muted-foreground"
                style={{ left: `${positionPercent(row.baseline, domain)}%` }}
                title={`${t("pages.insights.cooling.loadBandComparison.legend.baseline")}: ${row.baseline.toFixed(1)}${unitSuffix}`}
              />
              <span
                className="absolute top-1/2 h-2.5 w-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-background bg-primary"
                style={{ left: `${positionPercent(row.recent, domain)}%` }}
                title={`${t("pages.insights.cooling.loadBandComparison.legend.recent")}: ${row.recent.toFixed(1)}${unitSuffix}`}
              />
            </div>
          ) : (
            <WithheldReason
              reason={row.comparable ? { kind: "missingValue" } : row.reason}
            />
          )}
          <span className="text-right font-mono text-xs tabular-nums">
            {row.comparable
              ? formatSignedTemperatureDelta(row.delta, unitSuffix)
              : "—"}
          </span>
        </div>
      ))}
    </div>
  );
};

/**
 * The copy for a withheld band: Core's reason, plus - for a thin window -
 * how many minutes each short side still needs, so the user can tell a
 * band that will fill in from one that will not. The baseline window is
 * fixed once pinned, so its shortfall is shown as "n / required" rather
 * than as minutes to go.
 */
const WithheldReason = ({ reason }: { reason: LoadBandWithheldReason }) => {
  const { t } = useTranslation();

  if (reason.kind === "tooFewSampleMinutes") {
    const { shortfall } = reason;
    const details = [
      shortfall.recent.remaining > 0
        ? t(`${NOT_COMPARABLE_KEY}.recentShortfall`, {
            minutes: shortfall.recent.remaining,
          })
        : null,
      shortfall.baseline.remaining > 0
        ? t(`${NOT_COMPARABLE_KEY}.baselineShortfall`, {
            minutes: shortfall.baseline.sampleMinutes,
            required: shortfall.required,
          })
        : null,
    ].filter((detail): detail is string => detail != null);

    return (
      <span className="text-muted-foreground text-xs">
        <span className="italic">
          {t(`${NOT_COMPARABLE_KEY}.tooFewSampleMinutes`)}
        </span>
        {details.length > 0 && (
          <span
            className="ml-1 tabular-nums"
            data-testid="cooling-load-band-shortfall"
          >
            ({details.join(" · ")})
          </span>
        )}
      </span>
    );
  }

  return (
    <span className="text-muted-foreground text-xs italic">
      {t(`${NOT_COMPARABLE_KEY}.${reason.kind}`)}
    </span>
  );
};
