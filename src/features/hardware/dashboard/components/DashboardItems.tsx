import { platform } from "@tauri-apps/plugin-os";
import { useAtom } from "jotai";
import { RefreshCw } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { tv } from "tailwind-variants";
import {
  StorageBarChart,
  type StorageBarChartData,
} from "@/components/charts/Bar";
import { DoughnutChart } from "@/components/charts/DoughnutChart";
import { InfoTable } from "@/components/shared/InfoTable";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { minOpacity } from "@/consts/style";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import {
  cpuTempAtom,
  cpuUsageHistoryAtom,
  gpuDedicatedMemoryKbMapAtom,
  gpuTempAtom,
  gpuUsageSourceAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
  processorsUsageHistoryAtom,
  selectedGpuIdAtom,
  sensorTempsAtom,
} from "@/features/hardware/store/chart";
import type { NameValues } from "@/features/hardware/types/hardwareDataType";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { useTauriStore } from "@/hooks/useTauriStore";
import { useWindowSize } from "@/hooks/useWindowSize";
import { formatBytes } from "@/lib/formatter";
import { cn } from "@/lib/utils";
import type {
  LiveStorageHealth,
  StorageHealthRecord,
  StorageInfo,
} from "@/rspc/bindings";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";
import { useProcessInfo } from "../../hooks/useProcessInfo";
import {
  buildStorageHealthSummary,
  formatStorageHealthMetricValue,
  type StorageHealthDeviceViewModel,
  type StorageHealthMetric,
  type StorageHealthSummaryViewModel,
} from "../utils/storageHealthSummary";
import { MiniLineChart } from "./MiniLineChart";
import { StorageHealthStatusIcon } from "./StorageHealthStatusIcon";

export const CPUInfo = () => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const [cpuUsageHistory] = useAtom(cpuUsageHistoryAtom);
  const [cpuTemp] = useAtom(cpuTempAtom);
  const [sensorTemps] = useAtom(sensorTempsAtom);
  const { hardwareInfo } = useHardwareInfoAtom();
  const processes = useProcessInfo();
  const [processorsUsageHistory] = useAtom(processorsUsageHistoryAtom);

  const cpuTemperature = cpuTemp[0]?.value;
  const temperatureUnit = settings.temperatureUnit === "C" ? "°C" : "°F";

  return (
    <>
      <div className="flex h-[100px] justify-around xl:h-[200px]">
        <DoughnutChart
          chartValue={cpuUsageHistory[cpuUsageHistory.length - 1]}
          dataType={"usage"}
        />
        {/** Temperature is only available on supported platforms (Windows thermal zones) */}
        {cpuTemperature != null ? (
          <DoughnutChart chartValue={cpuTemperature} dataType={"temp"} />
        ) : (
          <MiniLineChart hardwareType="cpu" usage={cpuUsageHistory} />
        )}
      </div>

      {hardwareInfo.cpu ? (
        <InfoTable
          data={{
            [t("shared.name")]: hardwareInfo.cpu.name,
            [t("shared.vendor")]: hardwareInfo.cpu.vendor,
            [t("shared.coreCount")]: hardwareInfo.cpu.coreCount,
            [t("shared.threadCount")]: processorsUsageHistory[0]?.length || 0,
            [t("shared.defaultClockSpeed")]:
              `${hardwareInfo.cpu.clock} ${hardwareInfo.cpu.clockUnit}`,
            [t("shared.processCount")]: processes.length,
          }}
        />
      ) : (
        <Skeleton className="h-[188px] w-full rounded-md" />
      )}

      {sensorTemps.length > 0 && (
        <div className="mt-2 ml-2">
          <h4 className="font-bold text-sm md:text-md">
            {t("shared.thermalSensors")}
          </h4>
          <InfoTable
            data={Object.fromEntries(
              sensorTemps.map((sensor) => [
                sensor.name,
                `${sensor.value} ${temperatureUnit}`,
              ]),
            )}
          />
        </div>
      )}
    </>
  );
};

export const GPUInfo = () => {
  const { t } = useTranslation();
  const [graphicUsageHistory] = useAtom(graphicUsageHistoryAtom);
  const [gpuTemp] = useAtom(gpuTempAtom);
  const [gpuUsageSource] = useAtom(gpuUsageSourceAtom);
  const [selectedGpuId, setSelectedGpuId] = useAtom(selectedGpuIdAtom);
  const { hardwareInfo } = useHardwareInfoAtom();
  const { isBreak } = useWindowSize();
  const [showGpuUsageSource] = useTauriStore("showGpuUsageSource", false);
  const [gpuDedicatedMemoryKbMap] = useAtom(gpuDedicatedMemoryKbMapAtom);
  const os = useMemo(() => platform(), []);

  const gpus = hardwareInfo.gpus ?? [];
  const targetGpu = gpus.find((g) => g.id === selectedGpuId) ?? gpus[0] ?? null;
  const hasMultipleGpus = gpus.length > 1;

  const getTargetInfo = (data: NameValues) => {
    if (!targetGpu || data.length === 0) return undefined;
    // Prefer an exact name match for the currently selected GPU.
    const matched = data.find((x) => x.name === targetGpu.name);
    if (matched) return matched.value;
    // If there is exactly one GPU and one metric entry, allow a safe fallback.
    if (gpus.length === 1 && data.length === 1) {
      return data[0]?.value;
    }
    // Otherwise, avoid showing potentially incorrect metrics.
    return undefined;
  };

  const targetTemperature = getTargetInfo(gpuTemp);

  return (
    <>
      {hasMultipleGpus && targetGpu && (
        <TooltipProvider>
          <div
            role="tablist"
            aria-label={t("pages.dashboard.gpuSelector.label")}
            className="mb-3 flex justify-end gap-1"
          >
            {gpus.map((gpu, i) => {
              const isSelected = gpu.id === targetGpu.id;
              return (
                <Tooltip key={gpu.id}>
                  <TooltipTrigger asChild>
                    <button
                      type="button"
                      role="tab"
                      aria-selected={isSelected}
                      aria-label={gpu.name}
                      onClick={() => setSelectedGpuId(gpu.id)}
                      className={cn(
                        "min-w-7 rounded-md border px-2 py-0.5 font-mono text-xs transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50",
                        isSelected
                          ? "border-primary bg-primary text-primary-foreground"
                          : "border-border bg-transparent text-muted-foreground hover:bg-muted",
                      )}
                    >
                      #{i + 1}
                    </button>
                  </TooltipTrigger>
                  <TooltipContent>{gpu.name}</TooltipContent>
                </Tooltip>
              );
            })}
          </div>
        </TooltipProvider>
      )}
      <div className="relative">
        <div
          className={cn(
            "flex justify-around",
            !isBreak("md") && targetTemperature
              ? "h-[150px] lg:h-[100px] xl:h-[200px]"
              : "h-[100px] xl:h-[200px]",
          )}
        >
          <DoughnutChart
            chartValue={graphicUsageHistory[graphicUsageHistory.length - 1]}
            dataType={"usage"}
          />
          {targetTemperature && (
            <DoughnutChart
              chartValue={targetTemperature}
              dataType={"temp"}
              className={!isBreak("md") ? "mt-12" : ""}
            />
          )}
        </div>
        {showGpuUsageSource && gpuUsageSource && (
          <span className="absolute top-0 right-0 rounded-sm bg-muted/80 px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
            {gpuUsageSource}
          </span>
        )}
      </div>

      {hardwareInfo.gpus ? (
        hardwareInfo.gpus.map((gpu, index, arr) => (
          <div
            className={index !== 0 ? "py-3" : arr.length > 1 ? "pb-3" : ""}
            key={gpu.id}
          >
            {hasMultipleGpus && (
              <div className="mb-1 flex items-center px-4">
                <span
                  className={cn(
                    "rounded-md px-2 py-0.5 font-mono text-xs",
                    gpu.id === targetGpu?.id
                      ? "bg-primary text-primary-foreground"
                      : "bg-muted text-muted-foreground",
                  )}
                >
                  #{index + 1}
                </span>
              </div>
            )}
            {(() => {
              const dedicatedMemoryKb = gpuDedicatedMemoryKbMap[gpu.id] ?? null;
              const hasMemorySize = gpu.memorySize !== "N/A";
              const hasMemoryUsage = dedicatedMemoryKb != null;
              const formattedMemoryUsage = hasMemoryUsage
                ? (() => {
                    const [value, unit] = formatBytes(dedicatedMemoryKb * 1024);
                    return `${value} ${unit}`;
                  })()
                : null;
              const memorySizeDisplay = hasMemorySize
                ? gpu.memorySize
                : hasMemoryUsage
                  ? (formattedMemoryUsage ?? "N/A")
                  : "N/A";
              const memorySizeLabel = hasMemorySize
                ? t("shared.memorySize")
                : hasMemoryUsage
                  ? t("shared.memorySizeSharedUsage")
                  : t("shared.memorySize");

              const showCoreCount =
                gpu.memorySizeDedicated === "N/A" && os === "macos";
              const dedicatedMemoryDisplay = showCoreCount
                ? (gpu.coreCount ?? "N/A")
                : gpu.memorySizeDedicated;
              const dedicatedMemoryLabel = showCoreCount
                ? t("shared.coreCount")
                : t("shared.memorySizeDedicated");

              return (
                <InfoTable
                  data={{
                    [t("shared.name")]: gpu.name,
                    [t("shared.vendor")]: gpu.vendorName,
                    [memorySizeLabel]: memorySizeDisplay,
                    [dedicatedMemoryLabel]: dedicatedMemoryDisplay,
                  }}
                />
              );
            })()}
          </div>
        ))
      ) : (
        <Skeleton className="h-[188px] w-full rounded-md" />
      )}
    </>
  );
};

export const MemoryInfo = () => {
  const { t } = useTranslation();
  const [memoryUsageHistory] = useAtom(memoryUsageHistoryAtom);
  const { hardwareInfo } = useHardwareInfoAtom();
  const os = platform();

  const {
    memoryCurrentUsage,
    memoryCurrentUsageUnit,
  }:
    | {
        memoryCurrentUsage: number;
        memoryCurrentUsageUnit: "GB" | "MB";
      }
    | {
        memoryCurrentUsage: null;
        memoryCurrentUsageUnit: null;
      } = useMemo(() => {
    const current = memoryUsageHistory[memoryUsageHistory.length - 1];
    const [total, unit] = hardwareInfo.memory?.size.split(" ") || [null, null];

    if (total === null || unit === null || current == null) {
      return {
        memoryCurrentUsage: null,
        memoryCurrentUsageUnit: null,
      };
    }

    const currentUsage = (current / 100) * Number.parseFloat(total);
    const currentUsageUnit = unit === "GB" ? "GB" : "MB";
    return {
      memoryCurrentUsage: Number(currentUsage.toFixed(0)),
      memoryCurrentUsageUnit: currentUsageUnit,
    };
  }, [memoryUsageHistory, hardwareInfo.memory]);

  return (
    <>
      <div className="flex h-[100px] justify-around xl:h-[200px]">
        {memoryCurrentUsage ? (
          <DoughnutChart
            chartValue={memoryCurrentUsage}
            usagePercentage={
              memoryUsageHistory[memoryUsageHistory.length - 1] ?? 0
            }
            dataType={"memoryUsageValue"}
            unit={memoryCurrentUsageUnit}
          />
        ) : (
          <DoughnutChart
            chartValue={memoryUsageHistory[memoryUsageHistory.length - 1]}
            dataType={"usage"}
          />
        )}
        {/**  TODO If temperature can be retrieved here, display temperature instead of `MiniLineChart`  */}
        <MiniLineChart hardwareType="memory" usage={memoryUsageHistory} />
      </div>

      {hardwareInfo.memory ? (
        <div className="space-y-2">
          <InfoTable
            data={
              // On Linux, detailed information can only be obtained with pkexec,
              // so initially display memory.size and load button
              hardwareInfo.memory.isDetailed
                ? {
                    [t("shared.memoryType")]: hardwareInfo.memory.memoryType,
                    [t("shared.totalMemory")]: hardwareInfo.memory.size,
                    ...(hardwareInfo.memory.totalSlots > 0
                      ? {
                          [t("shared.memoryCount")]:
                            `${hardwareInfo.memory.memoryCount}/${hardwareInfo.memory.totalSlots}`,
                        }
                      : {}),
                    ...(hardwareInfo.memory.clock > 0
                      ? {
                          [t("shared.memoryClockSpeed")]:
                            `${hardwareInfo.memory.clock} ${hardwareInfo.memory.clockUnit}`,
                        }
                      : {}),
                  }
                : {
                    [t("shared.memoryType")]: hardwareInfo.memory.memoryType,
                    [t("shared.totalMemory")]: hardwareInfo.memory.size,
                  }
            }
          />
          <div className="flex justify-end">
            {!hardwareInfo.memory.isDetailed && os !== "macos" && (
              <FetchDetailButton />
            )}
          </div>
        </div>
      ) : (
        <Skeleton className="h-[188px] w-full rounded-md" />
      )}
    </>
  );
};

export const FetchDetailButton = () => {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const { fetchMemoryInfoDetail } = useHardwareInfoAtom();

  const handleLoadDetail = async () => {
    setLoading(true);
    await fetchMemoryInfoDetail();
    setLoading(false);
  };

  return (
    <Button onClick={handleLoadDetail} disabled={loading}>
      {t("shared.loadDetailedInfo")}
    </Button>
  );
};

const storageDataInfoGridVariants = tv({
  base: "grid grid-cols-1 gap-4",
  variants: {
    isWindows: {
      true: "2xl:grid-cols-2",
      false: "3xl:grid-cols-2",
    },
  },
});

export const StorageDataInfo = () => {
  const { t } = useTranslation();
  const { error } = useTauriDialog();
  const { settings } = useSettingsAtom();
  const { hardwareInfo } = useHardwareInfoAtom();
  const os = useMemo(() => platform(), []);
  const storageHealthErrorShownRef = useRef(false);
  const liveStorageHealthErrorShownRef = useRef(false);
  const storageHealthRecordsVersionRef = useRef(0);
  const storageHealthEnabled = settings.storageHealth.enabled ?? true;
  const [storageHealthRecords, setStorageHealthRecords] = useState<
    StorageHealthRecord[]
  >([]);
  const [liveStorageHealth, setLiveStorageHealth] = useState<
    LiveStorageHealth[]
  >([]);
  const [storageHealthRefreshing, setStorageHealthRefreshing] = useState(false);
  const [storageHealthRefreshError, setStorageHealthRefreshError] = useState<
    string | null
  >(null);

  // Sort by drive name
  const sortedStorage = hardwareInfo.storage.sort((a, b) =>
    a.name.localeCompare(b.name),
  );

  const chartData: StorageBarChartData[] = useMemo(() => {
    return sortedStorage
      ? sortedStorage.reduce(
          (acc: StorageBarChartData[], storage: StorageInfo) => {
            const used = storage.size - storage.free;
            const free = storage.free;
            acc.push({
              label: storage.name,
              used,
              free,
            });
            return acc;
          },
          [],
        )
      : [];
  }, [sortedStorage]);

  const storageHealthSummary =
    useMemo<StorageHealthSummaryViewModel | null>(() => {
      if (!storageHealthEnabled) return null;

      return buildStorageHealthSummary(storageHealthRecords, new Date(), {
        liveSignals: liveStorageHealth,
      });
    }, [storageHealthEnabled, storageHealthRecords, liveStorageHealth]);

  useEffect(() => {
    if (!storageHealthEnabled) {
      storageHealthErrorShownRef.current = false;
      setStorageHealthRecords([]);
      setStorageHealthRefreshError(null);
      setStorageHealthRefreshing(false);
      return;
    }

    let isMounted = true;

    const loadStorageHealthDevices = async () => {
      const recordsVersionAtRequest = storageHealthRecordsVersionRef.current;
      const result = await commands.getStorageHealthLatestRecords();
      if (!isMounted) return;
      if (recordsVersionAtRequest !== storageHealthRecordsVersionRef.current) {
        return;
      }

      if (isError(result)) {
        console.error(
          "Failed to fetch storage health dashboard records",
          result.error,
        );
        setStorageHealthRecords([]);
        if (!storageHealthErrorShownRef.current) {
          storageHealthErrorShownRef.current = true;
          void error(
            `${t("pages.dashboard.storageHealth.errors.fetchLatest")}\n${result.error}`,
          );
        }
        return;
      }

      storageHealthErrorShownRef.current = false;
      setStorageHealthRecords(result.data);
    };

    loadStorageHealthDevices();
    const intervalId = window.setInterval(loadStorageHealthDevices, 60_000);

    return () => {
      isMounted = false;
      window.clearInterval(intervalId);
    };
  }, [storageHealthEnabled, error, t]);

  useEffect(() => {
    if (!storageHealthEnabled) {
      liveStorageHealthErrorShownRef.current = false;
      setLiveStorageHealth([]);
      return;
    }

    let isMounted = true;

    const loadLiveStorageHealth = async () => {
      const result = await commands.getLiveStorageHealth();
      if (!isMounted) return;

      if (isError(result)) {
        console.error("Failed to fetch live storage health", result.error);
        setLiveStorageHealth([]);
        if (!liveStorageHealthErrorShownRef.current) {
          liveStorageHealthErrorShownRef.current = true;
          void error(
            `${t("pages.dashboard.storageHealth.errors.fetchLive")}\n${result.error}`,
          );
        }
        return;
      }

      liveStorageHealthErrorShownRef.current = false;
      setLiveStorageHealth(result.data);
    };

    loadLiveStorageHealth();
    const intervalId = window.setInterval(loadLiveStorageHealth, 10_000);

    return () => {
      isMounted = false;
      window.clearInterval(intervalId);
    };
  }, [storageHealthEnabled, error, t]);

  const refreshStorageDevices = async () => {
    if (!storageHealthEnabled || storageHealthRefreshing) return;

    setStorageHealthRefreshing(true);
    setStorageHealthRefreshError(null);

    const result = await commands.refreshStorageDevices();

    if (isError(result)) {
      console.error("Failed to refresh storage devices", result.error);
      const message = `${t("pages.dashboard.storageHealth.errors.refresh")}\n${result.error}`;
      setStorageHealthRefreshError(message);
      void error(message);
      setStorageHealthRefreshing(false);
      return;
    }

    storageHealthRecordsVersionRef.current += 1;
    setStorageHealthRecords(result.data);
    setStorageHealthRefreshing(false);
  };

  return (
    <div className="pt-2">
      <StorageHealthOverview
        summary={storageHealthSummary}
        onRefresh={storageHealthEnabled ? refreshStorageDevices : undefined}
        refreshError={storageHealthRefreshError}
        refreshing={storageHealthRefreshing}
      />
      <div
        className={storageDataInfoGridVariants({ isWindows: os === "windows" })}
      >
        <div>
          {sortedStorage.length > 0 ? (
            sortedStorage.map((storage) => {
              return (
                <div key={storage.name} className="mt-4 ml-2">
                  <h4 className="font-bold text-sm md:text-md">
                    {storage.name}
                    <span className="ml-2 font-normal text-gray-500 text-xs md:text-sm dark:text-gray-400">
                      ({storage.size} {storage.sizeUnit})
                    </span>
                  </h4>
                  <InfoTable
                    data={{
                      [t("shared.driveFileSystem")]: storage.fileSystem,
                      [t("shared.driveType")]: {
                        hdd: "HDD",
                        ssd: "SSD",
                        other: t("shared.other"),
                      }[storage.storageType],
                    }}
                  />
                </div>
              );
            })
          ) : (
            <Skeleton className="h-[188px] rounded-md" />
          )}
        </div>
        <div className="mt-8">
          {sortedStorage.length > 0 ? (
            <StorageBarChart
              chartData={chartData}
              unit={sortedStorage[0].sizeUnit}
            />
          ) : (
            <>
              <Skeleton className="ml-6 h-[88px] rounded-md" />
              <Skeleton className="mt-3 ml-6 h-[88px] rounded-md" />
            </>
          )}
        </div>
      </div>
    </div>
  );
};

const storageHealthMetricLabelKeys = {
  temperatureCelsius: "pages.dashboard.storageHealth.metrics.temperature",
  percentageUsed: "pages.dashboard.storageHealth.metrics.wear",
  availableSparePercent: "pages.dashboard.storageHealth.metrics.spare",
  powerOnHours: "pages.dashboard.storageHealth.metrics.powerOn",
  reallocatedSectorCount:
    "pages.dashboard.storageHealth.metrics.reallocatedSectors",
  currentPendingSectorCount:
    "pages.dashboard.storageHealth.metrics.pendingSectors",
  offlineUncorrectableCount:
    "pages.dashboard.storageHealth.metrics.uncorrectableSectors",
  mediaErrors: "pages.dashboard.storageHealth.metrics.mediaErrors",
  errorLogEntries: "pages.dashboard.storageHealth.metrics.errorLogEntries",
  unsafeShutdownCount: "pages.dashboard.storageHealth.metrics.unsafeShutdowns",
} as const satisfies Record<StorageHealthMetric["type"], string>;

const formatStorageHealthTimestamp = (value: string) => {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
};

const StorageHealthOverview = ({
  onRefresh,
  refreshError,
  refreshing = false,
  summary,
}: {
  onRefresh?: (() => void) | undefined;
  refreshError?: string | null;
  refreshing?: boolean;
  summary: StorageHealthSummaryViewModel | null;
}) => {
  const { t } = useTranslation();

  if (!summary || (summary.driveCount === 0 && !onRefresh)) return null;

  const focusDeviceLabel =
    summary.focusDevice?.model?.trim() ||
    summary.focusDevice?.displayName ||
    null;

  return (
    <div
      className={cn(
        "mb-3 space-y-2 border-border/60 border-b pb-3",
        summary.isStale && "opacity-70",
      )}
    >
      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <StorageHealthStatusIcon status={summary.status} size={17} />
          <span className="shrink-0 font-medium text-sm">
            {t("pages.dashboard.storageHealth.title")}
          </span>
          {focusDeviceLabel && (
            <span
              className="truncate text-muted-foreground text-xs"
              title={focusDeviceLabel}
            >
              {focusDeviceLabel}
            </span>
          )}
          {summary.isStale && (
            <span className="rounded-sm bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
              {t("pages.dashboard.storageHealth.stale")}
            </span>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-1">
          {summary.latestDate && (
            <span className="text-[10px] text-muted-foreground">
              {t("pages.dashboard.storageHealth.lastRecorded", {
                date: summary.latestDate,
              })}
            </span>
          )}
          {onRefresh && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  aria-label={t("pages.dashboard.storageHealth.refresh")}
                  className="size-7 rounded-sm p-0"
                  disabled={refreshing}
                  onClick={onRefresh}
                  type="button"
                  variant="ghost"
                >
                  <RefreshCw
                    className={cn("size-3.5", refreshing && "animate-spin")}
                  />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                {t("pages.dashboard.storageHealth.refresh")}
              </TooltipContent>
            </Tooltip>
          )}
        </div>
      </div>

      {refreshError && (
        <p className="truncate text-destructive text-xs" title={refreshError}>
          {refreshError}
        </p>
      )}

      {summary.metrics.length > 0 && (
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          {summary.metrics.map((metric) => (
            <div
              key={metric.type}
              className="min-w-0 rounded-sm bg-muted/40 px-2 py-1"
            >
              <div className="truncate text-[10px] text-muted-foreground">
                {t(storageHealthMetricLabelKeys[metric.type])}
              </div>
              <StorageHealthMetricValue metric={metric} />
            </div>
          ))}
        </div>
      )}

      {summary.reasons.length > 0 && (
        <ul className="space-y-0.5 text-amber-600 text-xs dark:text-amber-400">
          {summary.reasons.map((reason) => (
            <li key={reason} className="truncate" title={reason}>
              {reason}
            </li>
          ))}
        </ul>
      )}

      {summary.devices.length > 1 && (
        <StorageDeviceHealthOverview devices={summary.devices} />
      )}
    </div>
  );
};

const StorageHealthMetricValue = ({
  metric,
}: {
  metric: StorageHealthMetric;
}) => {
  const { t } = useTranslation();
  const value = formatStorageHealthMetricValue(metric);

  if (metric.type !== "temperatureCelsius") {
    return <div className="truncate font-mono text-xs">{value}</div>;
  }

  const tooltip = t(
    `pages.dashboard.storageHealth.temperatureSources.${metric.source}`,
    {
      datetime: formatStorageHealthTimestamp(metric.collectedAt),
    },
  );

  return (
    <Tooltip>
      <TooltipTrigger className="block max-w-full truncate border-0 bg-transparent p-0 text-left font-mono text-inherit text-xs outline-none focus-visible:ring-1 focus-visible:ring-ring">
        {value}
      </TooltipTrigger>
      <TooltipContent>{tooltip}</TooltipContent>
    </Tooltip>
  );
};

const StorageDeviceHealthOverview = ({
  devices,
}: {
  devices: StorageHealthDeviceViewModel[];
}) => {
  if (devices.length === 0) return null;

  return (
    <div className="grid max-h-20 grid-cols-1 gap-x-4 gap-y-1 overflow-y-auto sm:grid-cols-2">
      {devices.map((device) => (
        <div
          key={device.deviceId}
          className="grid min-h-6 grid-cols-[1rem_minmax(0,1fr)] items-center gap-2"
        >
          <StorageHealthStatusIcon status={device.status} size={15} />
          <span
            className="truncate text-muted-foreground text-xs"
            title={device.label}
          >
            {device.label}
          </span>
        </div>
      ))}
    </div>
  );
};

export const MotherboardDataInfo = () => {
  const { t } = useTranslation();
  const { hardwareInfo } = useHardwareInfoAtom();

  if (!hardwareInfo.motherboard) {
    return <Skeleton className="h-[188px] w-full rounded-md" />;
  }

  const mb = hardwareInfo.motherboard;

  return (
    <InfoTable
      data={{
        [t("shared.manufacturer")]: mb.manufacturer,
        [t("shared.product")]: mb.product,
        ...(mb.version ? { [t("shared.version")]: mb.version } : {}),
        [t("shared.serialNumber")]: mb.serialNumber,
        [t("shared.biosVendor")]: mb.biosVendor,
        [t("shared.biosVersion")]: mb.biosVersion,
        [t("shared.biosReleaseDate")]: mb.biosReleaseDate,
      }}
    />
  );
};

export const NetworkInfo = () => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const { networkInfo, initNetwork } = useHardwareInfoAtom();

  // biome-ignore lint/correctness/useExhaustiveDependencies: `initNetwork` is a stable function
  useEffect(() => {
    initNetwork();
  }, []);

  return (
    <>
      {networkInfo.map((network) => {
        return (
          <div
            key={network.macAddress}
            className="mt-4 mb-2 rounded-md bg-card px-4 pt-2 pb-2 text-foreground shadow-md"
            style={{
              opacity:
                settings.selectedBackgroundImg != null
                  ? Math.max(
                      (1 - settings.backgroundImgOpacity / 100) ** 2,
                      minOpacity,
                    )
                  : 1,
            }}
          >
            <Accordion type="single" collapsible className="w-full">
              <AccordionItem value="item-1" className="border-none">
                <AccordionTrigger>
                  <div className="flex w-full items-center justify-between">
                    <p className="text-xs md:text-sm xl:text-base">
                      {network.description ?? "No description"}
                    </p>
                    {/**  Display network usage in this section */}
                    <p className="mr-2 w-24 text-left text-gray-500 text-xs lg:text-sm dark:text-gray-400">
                      {network.ipv4[0] ?? "No IP Address"}
                    </p>
                  </div>
                </AccordionTrigger>
                <AccordionContent>
                  <table className="w-full text-left text-base">
                    <tbody className="text-sm xl:text-base">
                      <tr>
                        <th className="py-2 pr-4 dark:text-gray-400">
                          {t("shared.macAddress")}
                        </th>
                        <td className="py-2">
                          {network.macAddress ?? "No MAC Address"}
                        </td>
                      </tr>
                      <tr>
                        <th className="py-2 pr-4 dark:text-gray-400">
                          {t("shared.ipv4")}
                        </th>
                        <td className="py-2">
                          {network.ipv4.map((ip) => (
                            <p key={ip}>{ip}</p>
                          ))}
                        </td>
                      </tr>
                      <tr>
                        <th className="py-2 pr-4 dark:text-gray-400">
                          {t("shared.ipv4")} {t("shared.subnetMask")}
                        </th>
                        <td className="py-2">
                          {network.ipSubnet.map((subnet) => (
                            <p key={subnet}>{subnet}</p>
                          ))}
                        </td>
                      </tr>
                      <tr className="border-gray-700">
                        <th className="py-2 pr-4 dark:text-gray-400">
                          {t("shared.ipv4")} {t("shared.gateway")}
                        </th>
                        <td className="py-2">
                          {network.defaultIpv4Gateway.map((gateway) => (
                            <p key={gateway}>{gateway}</p>
                          ))}
                        </td>
                      </tr>
                      {network.ipv6.length > 0 && (
                        <tr className="border-gray-700">
                          <th className="py-2 pr-4 dark:text-gray-400">
                            {t("shared.ipv6")}
                          </th>
                          <td className="py-2">
                            {network.ipv6.map((ip) => (
                              <p className="text-xs xl:text-base" key={ip}>
                                {ip}
                              </p>
                            ))}
                          </td>
                        </tr>
                      )}
                      {network.linkLocalIpv6.length > 0 && (
                        <tr>
                          <th className="py-2 pr-4 dark:text-gray-400">
                            {t("shared.linkLocal")} {t("shared.ipv6")}{" "}
                            {t("shared.address")}
                          </th>
                          <td className="py-2">
                            {network.linkLocalIpv6.map((ip) => (
                              <p key={ip}>{ip}</p>
                            ))}
                          </td>
                        </tr>
                      )}
                      {network.defaultIpv6Gateway.length > 0 && (
                        <tr>
                          <th className="py-2 pr-4 dark:text-gray-400">
                            {t("shared.ipv6")} {t("shared.gateway")}
                          </th>
                          <td className="py-2">
                            {network.defaultIpv6Gateway.map((gateway) => (
                              <p key={gateway}>{gateway}</p>
                            ))}
                          </td>
                        </tr>
                      )}
                    </tbody>
                  </table>
                </AccordionContent>
              </AccordionItem>
            </Accordion>
          </div>
        );
      })}
    </>
  );
};
