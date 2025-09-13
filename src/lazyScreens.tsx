import React from "react";
import type { SelectedDisplayType } from "./types/ui";

// Lazy components (code-split per screen)
export const Dashboard = React.lazy(() =>
  import("./features/hardware/dashboard/Dashboard").then((m) => ({
    default: m.Dashboard,
  })),
);

export const ChartTemplate = React.lazy(() =>
  import("./features/hardware/usage/Usage").then((m) => ({
    default: m.ChartTemplate,
  })),
);

export const CpuUsages = React.lazy(() =>
  import("./features/hardware/usage/cpu/CpuUsage").then((m) => ({
    default: m.CpuUsages,
  })),
);

export const Insights = React.lazy(() =>
  import("./features/hardware/insights/Insights").then((m) => ({
    default: m.Insights,
  })),
);

export const Settings = React.lazy(() =>
  import("./features/settings/Settings").then((m) => ({
    default: m.Settings,
  })),
);

// Opportunistic prefetch on hover/focus
export const prefetchScreen = async (type: SelectedDisplayType) => {
  switch (type) {
    case "dashboard":
      await import("./features/hardware/dashboard/Dashboard");
      break;
    case "usage":
      await import("./features/hardware/usage/Usage");
      break;
    case "cpuDetail":
      await import("./features/hardware/usage/cpu/CpuUsage");
      break;
    case "insights":
      await import("./features/hardware/insights/Insights");
      break;
    case "settings":
      await import("./features/settings/Settings");
      break;
    default:
      break;
  }
};

