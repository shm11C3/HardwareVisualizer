import {
  ComputerTowerIcon,
  GaugeIcon,
  SquaresFourIcon,
} from "@phosphor-icons/react";
import { lazy, Suspense } from "react";
import { useTranslation } from "react-i18next";
import { BurnInShift } from "@/components/shared/BurnInShift";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useTauriStore } from "@/hooks/useTauriStore";
import { cn } from "@/lib/utils";
import type { DashboardItemType } from "./types/dashboardItem";

const Performance = lazy(() =>
  import("../performance/Performance").then((module) => ({
    default: module.Performance,
  })),
);

const Dashboard = lazy(() =>
  import("./Dashboard").then((module) => ({ default: module.Dashboard })),
);

export type GroupedDashboardTab = "performance" | "systemSpecifications";

export const DEFAULT_GROUPED_DASHBOARD_TAB =
  "performance" satisfies GroupedDashboardTab;

const GROUPED_DASHBOARD_TABS: readonly GroupedDashboardTab[] = [
  "performance",
  "systemSpecifications",
];

const SYSTEM_SPECIFICATIONS_EXCLUDED_ITEMS: readonly DashboardItemType[] = [
  "process",
];

export const normalizeGroupedDashboardTab = (
  value: string,
): GroupedDashboardTab =>
  GROUPED_DASHBOARD_TABS.includes(value as GroupedDashboardTab)
    ? (value as GroupedDashboardTab)
    : DEFAULT_GROUPED_DASHBOARD_TAB;

export const GroupedDashboard = ({
  isFullScreen = false,
  showTitle = true,
}: {
  isFullScreen?: boolean;
  showTitle?: boolean;
}) => {
  const { t } = useTranslation();
  const [storedTab, setStoredTab, isPending] = useTauriStore<string>(
    "groupedDashboardTab",
    DEFAULT_GROUPED_DASHBOARD_TAB,
  );

  if (isPending) {
    return null;
  }

  const activeTab = normalizeGroupedDashboardTab(storedTab);

  return (
    <BurnInShift enabled>
      <main
        className={cn(
          "min-h-screen w-full pt-12 pr-4 pb-8",
          isFullScreen ? "pl-4" : "pl-16",
        )}
        data-testid="grouped-dashboard"
      >
        <Tabs
          value={activeTab}
          onValueChange={(value) =>
            void setStoredTab(normalizeGroupedDashboardTab(value))
          }
        >
          <div className="2xl:mx-auto 2xl:w-3/4 2xl:px-4">
            {showTitle && (
              <div className="flex items-center gap-2 py-3">
                <SquaresFourIcon size={32} />
                <h2 className="font-bold text-3xl text-foreground">
                  {t("pages.dashboard.name")}
                </h2>
              </div>
            )}
            <TabsList
              className="sticky top-2 z-50"
              aria-label={t("pages.dashboard.viewTabs")}
            >
              <TabsTrigger value="performance">
                <GaugeIcon />
                {t("pages.dashboard.tabs.performance")}
              </TabsTrigger>
              <TabsTrigger value="systemSpecifications">
                <ComputerTowerIcon />
                {t("pages.dashboard.tabs.systemSpecifications")}
              </TabsTrigger>
            </TabsList>
          </div>

          <TabsContent value="performance">
            {activeTab === "performance" ? (
              <Suspense fallback={<DashboardTabSkeleton />}>
                <Performance
                  embedded
                  isFullScreen={isFullScreen}
                  showTitle={false}
                />
              </Suspense>
            ) : null}
          </TabsContent>
          <TabsContent
            value="systemSpecifications"
            className="pt-4 2xl:mx-auto 2xl:w-3/4 2xl:px-4"
          >
            {activeTab === "systemSpecifications" ? (
              <Suspense fallback={<DashboardTabSkeleton />}>
                <Dashboard
                  excludedItems={SYSTEM_SPECIFICATIONS_EXCLUDED_ITEMS}
                  responsive
                  specificationsMode
                />
              </Suspense>
            ) : null}
          </TabsContent>
        </Tabs>
      </main>
    </BurnInShift>
  );
};

const DashboardTabSkeleton = () => (
  <div
    className="grid grid-cols-1 gap-4 pt-2 md:grid-cols-2"
    data-testid="grouped-dashboard-tab-loading"
  >
    <Skeleton className="h-40 rounded-xl" />
    <Skeleton className="h-40 rounded-xl" />
  </div>
);
