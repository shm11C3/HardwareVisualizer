import {
  StorageBarChart,
  type StorageBarChartData,
} from "@/components/charts/Bar";
import { DoughnutChart } from "@/components/charts/DoughnutChart";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { minOpacity } from "@/consts/style";
import { ProcessesTable } from "@/features/hardware/dashboard/components/ProcessTable";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import {
  cpuUsageHistoryAtom,
  gpuTempAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
} from "@/features/hardware/store/chart";
import type { NameValues } from "@/features/hardware/types/hardwareDataType";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { cn } from "@/lib/utils";
import type { StorageInfo } from "@/rspc/bindings";
import {
  Cpu,
  GraphicsCard,
  HardDrives,
  Memory,
  Network,
} from "@phosphor-icons/react";
import { platform } from "@tauri-apps/plugin-os";
import { useAtom } from "jotai";
import { type JSX, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { tv } from "tailwind-variants";

const InfoTable = ({ data }: { data: { [key: string]: string | number } }) => {
  const { settings } = useSettingsAtom();

  return (
    <div
      className="grid grid-cols-2 gap-2 px-4 pt-2 pb-4 dark:text-white"
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
      {Object.keys(data).map((key) => (
        <div key={key}>
          <p className="text-slate-400 text-sm">{key}</p>
          <p>{data[key]}</p>
        </div>
      ))}
    </div>
  );
};

const DataArea = ({
  children,
  title,
  icon,
  border = false,
  className = "rounded-2xl bg-zinc-300/50 dark:bg-slate-950/50",
}: {
  children: React.ReactNode;
  title?: string;
  icon?: JSX.Element;
  border?: boolean;
  className?: string;
}) => {
  return (
    <div className="p-4">
      {
        <div
          className={cn(
            border && "border border-zinc-400 dark:border-zinc-600",
            className,
          )}
        >
          <div className="flex items-center pt-4 pb-2 pl-4">
            {icon && <div className="mr-2 mb-0.5">{icon}</div>}
            {title && (
              <h3 className="align-middle font-bold text-xl">{title}</h3>
            )}
          </div>
          <div className="px-4 pb-4">{children}</div>
        </div>
      }
    </div>
  );
};

const CPUInfo = () => {
  const { t } = useTranslation();
  const [cpuUsageHistory] = useAtom(cpuUsageHistoryAtom);
  const { hardwareInfo } = useHardwareInfoAtom();

  return (
    <>
      <DoughnutChart
        chartValue={cpuUsageHistory[cpuUsageHistory.length - 1]}
        dataType={"usage"}
      />
      {hardwareInfo.cpu ? (
        <InfoTable
          data={{
            [t("shared.name")]: hardwareInfo.cpu.name,
            [t("shared.vendor")]: hardwareInfo.cpu.vendor,
            [t("shared.coreCount")]: hardwareInfo.cpu.coreCount,
            [t("shared.defaultClockSpeed")]:
              `${hardwareInfo.cpu.clock} ${hardwareInfo.cpu.clockUnit}`,
          }}
        />
      ) : (
        <Skeleton className="h-[188px] w-full rounded-md" />
      )}
    </>
  );
};

const GPUInfo = () => {
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

const MemoryInfo = () => {
  const { t } = useTranslation();
  const [memoryUsageHistory] = useAtom(memoryUsageHistoryAtom);
  const { hardwareInfo } = useHardwareInfoAtom();

  return (
    <>
      <DoughnutChart
        chartValue={memoryUsageHistory[memoryUsageHistory.length - 1]}
        dataType={"usage"}
      />
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

const FetchDetailButton = () => {
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

const StorageDataInfo = () => {
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

const NetworkInfo = () => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const { networkInfo, initNetwork } = useHardwareInfoAtom();

  // biome-ignore lint/correctness/useExhaustiveDependencies: <explanation>
  useEffect(() => {
    initNetwork();
  }, []);

  return (
    <>
      {networkInfo.map((network) => {
        return (
          <div
            key={network.macAddress}
            className="mt-4 mb-2 rounded-md border bg-zinc-300 px-4 pt-2 pb-2 shadow-md dark:bg-gray-800 dark:text-white"
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
                    <p className="mr-2 w-24 text-left text-gray-500 text-sm dark:text-gray-400 ">
                      {network.ipv4[0] ?? "No IP Address"}
                    </p>
                  </div>
                </AccordionTrigger>
                <AccordionContent>
                  <table className="w-full text-left text-base">
                    <tbody>
                      <tr className="border-gray-700 border-b">
                        <th className="py-2 pr-4 dark:text-gray-400">
                          {t("shared.macAddress")}
                        </th>
                        <td className="py-2">
                          {network.macAddress ?? "No MAC Address"}
                        </td>
                      </tr>
                      <tr className="border-gray-700 border-b">
                        <th className="py-2 pr-4 dark:text-gray-400">
                          {t("shared.ipv4")}
                        </th>
                        <td className="py-2">
                          {network.ipv4.map((ip) => (
                            <p key={ip}>{ip}</p>
                          ))}
                        </td>
                      </tr>
                      <tr className="border-gray-700 border-b">
                        <th className="py-2 pr-4 dark:text-gray-400">
                          {t("shared.ipv4")} {t("shared.subnetMask")}
                        </th>
                        <td className="py-2">
                          {network.ipSubnet.map((subnet) => (
                            <p key={subnet}>{subnet}</p>
                          ))}
                        </td>
                      </tr>
                      <tr className="border-gray-700 border-b">
                        <th className="py-2 pr-4 dark:text-gray-400">
                          {t("shared.ipv4")} {t("shared.gateway")}
                        </th>
                        <td className="py-2">
                          {network.defaultIpv4Gateway.map((gateway) => (
                            <p key={gateway}>{gateway}</p>
                          ))}
                        </td>
                      </tr>
                      <tr className="border-gray-700 border-b">
                        <th className="py-2 pr-4 dark:text-gray-400">
                          {t("shared.ipv6")}
                        </th>
                        <td className="py-2">
                          {network.ipv6.map((ip) => (
                            <p key={ip}>{ip}</p>
                          ))}
                        </td>
                      </tr>
                      <tr className="border-gray-700 border-b">
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
                      <tr className="border-gray-700 border-b">
                        <th className="py-2 pr-4 dark:text-gray-400">
                          {t("shared.ipv6")} {t("shared.gateway")}
                        </th>
                        <td className="py-2">
                          {network.defaultIpv6Gateway.map((gateway) => (
                            <p key={gateway}>{gateway}</p>
                          ))}
                        </td>
                      </tr>
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

type DataTypeKey = "cpu" | "memory" | "storage" | "gpu" | "network" | "process";

const Dashboard = () => {
  const { init, hardwareInfo } = useHardwareInfoAtom();
  const { settings } = useSettingsAtom();
  const { t } = useTranslation();

  // biome-ignore lint/correctness/useExhaustiveDependencies: <explanation>
  useEffect(() => {
    init();
  }, []);

  // 表示対象のリストをフィルタしたうえで左右交互に振り分ける
  const [hardwareInfoListLeft, hardwareInfoListRight] = useMemo(() => {
    const fullList = [
      {
        key: "cpu",
        icon: <Cpu size={24} color={`rgb(${settings.lineGraphColor.cpu})`} />,
        component: <CPUInfo />,
      },
      (hardwareInfo.gpus == null || hardwareInfo.gpus.length > 0) && {
        key: "gpu",
        icon: (
          <GraphicsCard
            size={24}
            color={`rgb(${settings.lineGraphColor.gpu})`}
          />
        ),
        component: <GPUInfo />,
      },
      {
        key: "memory",
        icon: (
          <Memory size={24} color={`rgb(${settings.lineGraphColor.memory})`} />
        ),
        component: <MemoryInfo />,
      },
      {
        key: "process",
        component: <ProcessesTable />,
      },
      {
        key: "storage",
        icon: <HardDrives size={24} color="var(--color-storage)" />,
        component: <StorageDataInfo />,
      },
      {
        key: "network",
        icon: <Network size={24} color="oklch(74.6% 0.16 232.661)" />,
        component: <NetworkInfo />,
      },
    ].filter(
      (
        x,
      ): x is { key: DataTypeKey; icon: JSX.Element; component: JSX.Element } =>
        Boolean(x),
    );

    return fullList.reduce<[typeof fullList, typeof fullList]>(
      ([left, right], item, index) => {
        if (index % 2 === 0) {
          left.push(item);
        } else {
          right.push(item);
        }
        return [left, right];
      },
      [[], []],
    );
  }, [hardwareInfo, settings.lineGraphColor]);

  const dataAreaKey2Title: Partial<Record<DataTypeKey, string>> = {
    cpu: "CPU",
    memory: "RAM",
    storage: t("shared.storage"),
    gpu: "GPU",
    network: t("shared.network"),
  };

  return (
    <div className="flex flex-wrap gap-4">
      <div className="flex flex-1 flex-col gap-4">
        {hardwareInfoListLeft.map(({ key, icon, component }) =>
          key !== "process" ? (
            <DataArea
              key={`left-${key}`}
              title={dataAreaKey2Title[key]}
              icon={icon}
            >
              {component}
            </DataArea>
          ) : (
            <div key={`left-${key}`} className="p-4">
              {component}
            </div>
          ),
        )}
      </div>
      <div className="flex flex-1 flex-col gap-4">
        {hardwareInfoListRight.map(({ key, icon, component }) =>
          key !== "process" ? (
            <DataArea
              key={`right-${key}`}
              title={dataAreaKey2Title[key]}
              icon={icon}
            >
              {component}
            </DataArea>
          ) : (
            <div key={`right-${key}`} className="p-4">
              {component}
            </div>
          ),
        )}
      </div>
    </div>
  );
};

export default Dashboard;
