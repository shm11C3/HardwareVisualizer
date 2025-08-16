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
  gpuTempAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
  processorsUsageHistoryAtom,
} from "@/features/hardware/store/chart";
import type { NameValues } from "@/features/hardware/types/hardwareDataType";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
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
      <div className="flex h-[200px] justify-around">
        <DoughnutChart
          chartValue={cpuUsageHistory[cpuUsageHistory.length - 1]}
          dataType={"usage"}
        />
        {/**  TODO ここで温度を取得し取得できれば `MiniLineChart` ではなく温度を表示させる  */}
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
  const { hardwareInfo } = useHardwareInfoAtom();

  const getTargetInfo = (data: NameValues) => {
    return data.find(
      (x) => hardwareInfo.gpus && x.name === hardwareInfo.gpus[0].name,
    )?.value;
  };

  const targetTemperature = getTargetInfo(gpuTemp);

  return (
    <>
      <div className="flex h-[200px] justify-around">
        <DoughnutChart
          chartValue={graphicUsageHistory[graphicUsageHistory.length - 1]}
          dataType={"usage"}
        />
        {targetTemperature && (
          <DoughnutChart chartValue={targetTemperature} dataType={"temp"} />
        )}
      </div>

      {hardwareInfo.gpus ? (
        hardwareInfo.gpus.map((gpu, index, arr) => (
          <div
            className={index !== 0 ? "py-3" : arr.length > 1 ? "pb-3" : ""}
            key={`${gpu.name}${index}`}
          >
            <InfoTable
              data={{
                [t("shared.name")]: gpu.name,
                [t("shared.vendor")]: gpu.vendorName,
                [t("shared.memorySize")]: gpu.memorySize,
                [t("shared.memorySizeDedicated")]: gpu.memorySizeDedicated,
              }}
            />
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
      <div className="flex h-[200px] justify-around">
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
        {/**  TODO ここで温度を取得し取得できれば `MiniLineChart` ではなく温度を表示させる  */}
        <MiniLineChart hardwareType="memory" usage={memoryUsageHistory} />
      </div>

      {hardwareInfo.memory ? (
        <div className="space-y-2">
          <InfoTable
            data={
              // Linuxの場合は pkexec でしか詳細な情報が取得できないため、
              // 初期状態では memory.size と読み込みボタンを表示する
              hardwareInfo.memory.isDetailed
                ? {
                    [t("shared.memoryType")]: hardwareInfo.memory.memoryType,
                    [t("shared.totalMemory")]: hardwareInfo.memory.size,
                    [t("shared.memoryCount")]:
                      `${hardwareInfo.memory.memoryCount}/${hardwareInfo.memory.totalSlots}`,
                    [t("shared.memoryClockSpeed")]:
                      `${hardwareInfo.memory.clock} ${hardwareInfo.memory.clockUnit}`,
                  }
                : {
                    [t("shared.memoryType")]: hardwareInfo.memory.memoryType,
                    [t("shared.totalMemory")]: hardwareInfo.memory.size,
                  }
            }
          />
          <div className="flex justify-end">
            {!hardwareInfo.memory.isDetailed && <FetchDetailButton />}
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

  // TODO ストレージの総量・総使用量をグラフ化する

  // ドライブ名でソート
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
                  <h4 className="font-bold text-md">
                    {storage.name}
                    <span className="ml-2 font-normal text-gray-500 text-sm dark:text-gray-400">
                      {" "}
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
                    <p>{network.description ?? "No description"}</p>
                    {/**  この部分にネットワーク使用量を表示 */}
                    <p className="mr-2 w-24 text-left text-gray-500 text-sm dark:text-gray-400">
                      {network.ipv4[0] ?? "No IP Address"}
                    </p>
                  </div>
                </AccordionTrigger>
                <AccordionContent>
                  <table className="w-full text-left text-base">
                    <tbody>
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
                              <p key={ip}>{ip}</p>
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
