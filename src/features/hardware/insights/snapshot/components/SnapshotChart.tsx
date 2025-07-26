import { Warning, Database } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { SingleLineChart } from "@/components/charts/LineChart";
import type { ChartConfig } from "@/components/ui/chart";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import type { ClientSettings } from "@/rspc/bindings";

interface SnapshotChartProps {
  labels: string[];
  chartData: (number | null)[];
  selectedDataType: "cpu" | "memory";
  chartConfig: ChartConfig;
  settings: ClientSettings;
  loading?: boolean;
  error?: Error | null;
  onRetry?: () => void;
}

export const SnapshotChart = ({
  labels,
  chartData,
  selectedDataType,
  chartConfig,
  settings,
  loading = false,
  error = null,
  onRetry,
}: SnapshotChartProps) => {
  const { t } = useTranslation();

  if (loading) {
    return (
      <div className="flex items-center justify-center h-[400px]">
        <div className="text-center space-y-4 w-full">
          <Skeleton className="h-[300px] w-full" />
          <Skeleton className="h-4 w-48 mx-auto" />
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-[400px]">
        <div className="text-center space-y-2">
          <Warning className="h-12 w-12 text-destructive mx-auto" />
          <p className="text-destructive font-medium">Failed to load data</p>
          <p className="text-sm text-muted-foreground">{error.message}</p>
          {onRetry && (
            <Button onClick={onRetry} variant="outline" size="sm">
              Try Again
            </Button>
          )}
        </div>
      </div>
    );
  }

  if (!chartData.some((d) => d !== null)) {
    return (
      <div className="flex items-center justify-center h-[400px]">
        <div className="text-center space-y-2">
          <Database className="h-12 w-12 text-muted-foreground mx-auto" />
          <p className="font-medium">No data available</p>
          <p className="text-sm text-muted-foreground">
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