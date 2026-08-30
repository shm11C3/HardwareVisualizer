import { useTranslation } from "react-i18next";
import {
  type ExplorerMinimapSegment,
  explorerWindowColors,
} from "../utils/loadTemperatureExplorer";

/**
 * A read-only strip showing where the two compared periods sit on a shared
 * timeline, and how far apart they are.
 *
 * Deliberately not a draggable brush: the Explorer compares a *fixed*
 * reference (the established baseline window, which the user cannot move
 * without invalidating every other Cooling Insight reading) against a
 * trailing window chosen from presets. There is nothing to drag, so this
 * only has to answer "which two periods am I looking at". Drawn as plain
 * positioned elements, the same self-drawn approach `CoverageStrip` uses.
 */
export const ExplorerWindowMinimap = ({
  segments,
}: {
  segments: ExplorerMinimapSegment[];
}) => {
  const { t } = useTranslation();

  if (segments.length === 0) {
    return null;
  }

  return (
    <div className="space-y-1" data-testid="cooling-explorer-minimap">
      <p className="text-muted-foreground text-xs">
        {t("pages.insights.cooling.explorer.minimap.title")}
      </p>
      <div className="relative h-3 w-full rounded-full bg-muted">
        {segments.map((segment) => (
          <span
            key={segment.kind}
            className="absolute inset-y-0 rounded-full"
            style={{
              left: `${segment.offsetPercent}%`,
              width: `${segment.widthPercent}%`,
              // Same two colors the scatter uses, so a window reads as
              // the same color everywhere in the panel.
              backgroundColor: explorerWindowColors[segment.kind],
            }}
            title={`${t(
              `pages.insights.cooling.explorer.legend.${segment.kind}`,
            )}: ${segment.startDate}–${segment.endDate}`}
          />
        ))}
      </div>
      <dl className="flex flex-wrap gap-x-4 gap-y-0.5 text-muted-foreground text-xs">
        {segments.map((segment) => (
          <div key={segment.kind} className="flex items-center gap-1.5">
            <span
              aria-hidden
              className="h-2 w-2 rounded-full"
              style={{ backgroundColor: explorerWindowColors[segment.kind] }}
            />
            <dt>
              {t(`pages.insights.cooling.explorer.legend.${segment.kind}`)}
            </dt>
            <dd className="font-mono tabular-nums">
              {segment.startDate}–{segment.endDate}
            </dd>
          </div>
        ))}
      </dl>
    </div>
  );
};
