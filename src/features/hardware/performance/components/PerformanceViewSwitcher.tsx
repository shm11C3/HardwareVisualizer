import {
  ArrowsInSimpleIcon,
  ChartLineUpIcon,
  SquaresFourIcon,
} from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  type PerformanceView,
  performanceViews,
} from "../types/performanceLayout";

const viewIcons = {
  panels: <SquaresFourIcon />,
  compact: <ArrowsInSimpleIcon />,
  monitor: <ChartLineUpIcon />,
} satisfies Record<PerformanceView, React.ReactNode>;

export const PerformanceViewSwitcher = ({
  view,
  onViewChange,
}: {
  view: PerformanceView;
  onViewChange: (view: PerformanceView) => void;
}) => {
  const { t } = useTranslation();

  return (
    <Tabs
      value={view}
      onValueChange={(value) => onViewChange(value as PerformanceView)}
      className="min-w-0"
    >
      <TabsList
        className="h-auto max-w-full justify-start overflow-x-auto"
        aria-label={t("pages.performance.viewSwitcher")}
      >
        {performanceViews.map((candidate) => (
          <TabsTrigger
            key={candidate}
            value={candidate}
            className="min-h-9 px-3"
          >
            {viewIcons[candidate]}
            {t(`pages.performance.views.${candidate}`)}
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  );
};
