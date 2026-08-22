export const performanceViews = ["panels", "compact", "monitor"] as const;

export type PerformanceView = (typeof performanceViews)[number];

export const performancePanelIds = [
  "usageGraphs",
  "processTable",
  "perCore",
  "motherboardSensors",
  "power",
] as const;

export type PerformancePanelId = (typeof performancePanelIds)[number];

/**
 * Requested panel column count. Two columns are a maximum, not a guarantee:
 * the grid falls back to one column when the window is too narrow to hold
 * two readable panels side by side.
 */
export const performancePanelColumnOptions = [1, 2] as const;

export type PerformancePanelColumns =
  (typeof performancePanelColumnOptions)[number];

export const DEFAULT_PERFORMANCE_PANEL_COLUMNS: PerformancePanelColumns = 1;

export const normalizePerformancePanelColumns = (
  value: unknown,
): PerformancePanelColumns =>
  performancePanelColumnOptions.includes(value as PerformancePanelColumns)
    ? (value as PerformancePanelColumns)
    : DEFAULT_PERFORMANCE_PANEL_COLUMNS;

export type PerformanceCustomLayout = {
  order: PerformancePanelId[];
  visible: PerformancePanelId[];
};

export const DEFAULT_PERFORMANCE_VIEW: PerformanceView = "panels";

/**
 * Panels every user starts with. perCore and motherboardSensors stay hidden
 * until explicitly enabled so the default screen only carries the panels most
 * sessions actually watch.
 */
const DEFAULT_VISIBLE_PANELS: readonly PerformancePanelId[] = [
  "usageGraphs",
  "processTable",
  "power",
];

export const DEFAULT_PERFORMANCE_CUSTOM_LAYOUT: PerformanceCustomLayout = {
  order: [...performancePanelIds],
  visible: [...DEFAULT_VISIBLE_PANELS],
};

/**
 * The retired Detailed and Custom Performance Layout Presets both map onto the
 * single customizable panel view; their stored values must keep resolving.
 */
const LEGACY_VIEW_ALIASES: Record<string, PerformanceView> = {
  detailed: "panels",
  custom: "panels",
};

export const normalizePerformanceView = (value: unknown): PerformanceView => {
  if (performanceViews.includes(value as PerformanceView)) {
    return value as PerformanceView;
  }

  if (typeof value === "string" && value in LEGACY_VIEW_ALIASES) {
    return LEGACY_VIEW_ALIASES[value];
  }

  return DEFAULT_PERFORMANCE_VIEW;
};

const uniqueKnownPanels = (value: unknown): PerformancePanelId[] => {
  if (!Array.isArray(value)) {
    return [];
  }

  return value.reduce<PerformancePanelId[]>((panels, candidate) => {
    if (
      performancePanelIds.includes(candidate as PerformancePanelId) &&
      !panels.includes(candidate as PerformancePanelId)
    ) {
      panels.push(candidate as PerformancePanelId);
    }
    return panels;
  }, []);
};

/**
 * Normalize a stored layout: unknown panels are dropped, panels the stored
 * layout has never seen are appended to the order. Existing visibility is
 * preserved exactly; defaults apply only when the stored visibility is absent
 * or malformed. An empty visible set is valid because the Instrument Strip
 * stays mounted regardless of panel visibility.
 */
export const normalizePerformanceCustomLayout = (
  value: unknown,
): PerformanceCustomLayout => {
  if (value == null || typeof value !== "object") {
    return {
      order: [...DEFAULT_PERFORMANCE_CUSTOM_LAYOUT.order],
      visible: [...DEFAULT_PERFORMANCE_CUSTOM_LAYOUT.visible],
    };
  }

  const candidate = value as Partial<PerformanceCustomLayout>;
  const knownOrder = uniqueKnownPanels(candidate.order);
  const missingPanels = performancePanelIds.filter(
    (panel) => !knownOrder.includes(panel),
  );
  const visible = Array.isArray(candidate.visible)
    ? uniqueKnownPanels(candidate.visible)
    : [...DEFAULT_VISIBLE_PANELS];

  return {
    order: [...knownOrder, ...missingPanels],
    visible,
  };
};

export const performanceCustomLayoutsEqual = (
  left: unknown,
  right: PerformanceCustomLayout,
) => {
  if (left == null || typeof left !== "object") {
    return false;
  }

  const candidate = left as Partial<PerformanceCustomLayout>;
  return (
    Array.isArray(candidate.order) &&
    Array.isArray(candidate.visible) &&
    candidate.order.length === right.order.length &&
    candidate.visible.length === right.visible.length &&
    candidate.order.every((panel, index) => panel === right.order[index]) &&
    candidate.visible.every((panel, index) => panel === right.visible[index])
  );
};
