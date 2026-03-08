import { platform } from "@tauri-apps/plugin-os";
import { useAtom } from "jotai";
import { useEffect, useMemo, useState } from "react";
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
import { minOpacity } from "@/consts/style";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import {
  cpuUsageHistoryAtom,
  gpuDedicatedMemoryKbAtom,
  gpuTempAtom,
  gpuUsageSourceAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
  processorsUsageHistoryAtom,
} from "@/features/hardware/store/chart";
import type { NameValues } from "@/features/hardware/types/hardwareDataType";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { useTauriStore } from "@/hooks/useTauriStore";
import { useWindowSize } from "@/hooks/useWindowSize";
import { formatBytes } from "@/lib/formatter";
import { cn } from "@/lib/utils";
import type { StorageInfo } from "@/rspc/bindings";
import { useProcessInfo } from "../../hooks/useProcessInfo";
import { MiniLineChart } from "./MiniLineChart";

export const CPUInfo = () => {
  const { t } = useTranslation();
  const [cpuUsageHistory] = useAtom(cpuUsageHistoryAtom);
  const { hardwareInfo } = useHardwareInfoAtom();
  const processes = useProcessInfo();
  const [processorsUsageHistory] = useAtom(processorsUsageHistoryAtom);

  return (
    <>
      <div className="flex h-[100px] justify-around xl:h-[200px]">
        <DoughnutChart
          chartValue={cpuUsageHistory[cpuUsageHistory.length - 1]}
          dataType={"usage"}
        />
        {/**  TODO If temperature can be retrieved here, display temperature instead of `MiniLineChart`  */}
        <MiniLineChart hardwareType="cpu" usage={cpuUsageHistory} />
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
    </>
  );
};

export const GPUInfo = () => {
  const { t } = useTranslation();
  const [graphicUsageHistory] = useAtom(graphicUsageHistoryAtom);
  const [gpuTemp] = useAtom(gpuTempAtom);
  const [gpuUsageSource] = useAtom(gpuUsageSourceAtom);
  const { hardwareInfo } = useHardwareInfoAtom();
  const { isBreak } = useWindowSize();
  const [showGpuUsageSource] = useTauriStore("showGpuUsageSource", false);
  const [gpuDedicatedMemoryKb] = useAtom(gpuDedicatedMemoryKbAtom);
  const os = useMemo(() => platform(), []);

  const getTargetInfo = (data: NameValues) => {
    return data.find(
      (x) => hardwareInfo.gpus && x.name === hardwareInfo.gpus[0].name,
    )?.value;
  };

  const targetTemperature = getTargetInfo(gpuTemp);

  return (
    <>
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
            key={`${gpu.name}${index}`}
          >
            {(() => {
              const hasMemorySize = gpu.memorySize !== "N/A";
              const hasMemoryUsage = gpuDedicatedMemoryKb != null;
              const formattedMemoryUsage = hasMemoryUsage
                ? (() => {
                    const [value, unit] = formatBytes(
                      gpuDedicatedMemoryKb * 1024,
                    );
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

    if (total === null || unit === null) {
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
            usagePercentage={memoryUsageHistory[memoryUsageHistory.length - 1]}
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
  const { hardwareInfo } = useHardwareInfoAtom();
  const os = useMemo(() => platform(), []);

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

  return (
    <div className="pt-2">
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
