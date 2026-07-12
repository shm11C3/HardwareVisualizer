export const performanceLayoutPresets = [
  "compact",
  "monitor",
  "detailed",
  "custom",
] as const;

export type PerformanceLayoutPreset = (typeof performanceLayoutPresets)[number];

export const performancePanelIds = [
  "currentValues",
  "usageGraphs",
  "processTable",
] as const;

export type PerformancePanelId = (typeof performancePanelIds)[number];

export type PerformanceCustomLayout = {
  order: PerformancePanelId[];
  visible: PerformancePanelId[];
};

export const DEFAULT_PERFORMANCE_PRESET: PerformanceLayoutPreset = "detailed";

export const DEFAULT_PERFORMANCE_CUSTOM_LAYOUT: PerformanceCustomLayout = {
  order: [...performancePanelIds],
  visible: [...performancePanelIds],
};

export const normalizePerformancePreset = (
  value: unknown,
): PerformanceLayoutPreset =>
  performanceLayoutPresets.includes(value as PerformanceLayoutPreset)
    ? (value as PerformanceLayoutPreset)
    : DEFAULT_PERFORMANCE_PRESET;

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

export const normalizePerformanceCustomLayout = (
  value: unknown,
): PerformanceCustomLayout => {
  const candidate =
    value != null && typeof value === "object"
      ? (value as Partial<PerformanceCustomLayout>)
      : {};
  const knownOrder = uniqueKnownPanels(candidate.order);
  const missingPanels = performancePanelIds.filter(
    (panel) => !knownOrder.includes(panel),
  );
  const visiblePanels = uniqueKnownPanels(candidate.visible);
  const normalizedVisiblePanels = [
    ...visiblePanels,
    ...missingPanels.filter((panel) => !visiblePanels.includes(panel)),
  ];

  return {
    order: [...knownOrder, ...missingPanels],
    visible:
      normalizedVisiblePanels.length > 0
        ? normalizedVisiblePanels
        : [DEFAULT_PERFORMANCE_CUSTOM_LAYOUT.visible[0]],
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
