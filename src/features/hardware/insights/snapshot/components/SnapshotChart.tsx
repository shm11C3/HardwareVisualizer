import { DatabaseIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { SingleLineChart } from "@/components/charts/LineChart";
import { LoadFailure } from "@/components/LoadFailure";
import type { ChartConfig } from "@/components/ui/chart";
import { Skeleton } from "@/components/ui/skeleton";
import type { ClientSettings } from "@/rspc/bindings";

interface SnapshotChartProps {
  labels: string[];
  chartData: (number | null)[];
  selectedDataType: "cpu" | "memory";
  chartConfig: ChartConfig;
  settings: ClientSettings;
  loading?: boolean;
  /** The read failed. Distinct from an empty result. */
  hasError?: boolean;
  onRetry?: () => void;
}

export const SnapshotChart = ({
  labels,
  chartData,
  selectedDataType,
  chartConfig,
  settings,
  loading = false,
  hasError = false,
  onRetry,
}: SnapshotChartProps) => {
  const { t } = useTranslation();

  if (loading) {
    return (
      <div className="flex h-[400px] items-center justify-center">
        <div className="w-full space-y-4 text-center">
          <Skeleton className="h-[300px] w-full" />
          <Skeleton className="mx-auto h-4 w-48" />
        </div>
      </div>
    );
  }

  if (hasError) {
    return (
      <LoadFailure
        className="h-[400px]"
        message={t("shared.loadFailed")}
        onRetry={onRetry}
      />
    );
  }

  if (!chartData.some((d) => d !== null)) {
    return (
      <div className="flex h-[400px] items-center justify-center">
        <div className="space-y-2 text-center">
          <DatabaseIcon className="mx-auto h-12 w-12 text-muted-foreground" />
          <p className="font-medium">No data available</p>
          <p className="text-muted-foreground text-sm">
            Try adjusting your time range or filters
          </p>
        </div>
      </div>
    );
  }

  return (
    <SingleLineChart
      labels={labels}
      chartData={chartData}
      dataType={selectedDataType}
      chartConfig={chartConfig}
      border={false}
      size="lg"
      lineGraphMix={false}
      lineGraphShowScale={true}
      lineGraphShowTooltip={true}
      lineGraphType={settings.lineGraphType}
      lineGraphShowLegend={false}
      dataKey={`${t("shared.usage")} (%)`}
    />
  );
};
