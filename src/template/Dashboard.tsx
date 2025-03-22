import {
  cpuUsageHistoryAtom,
  gpuTempAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
} from "@/atom/chart";
import { useHardwareInfoAtom } from "@/atom/useHardwareInfoAtom";
import { useSettingsAtom } from "@/atom/useSettingsAtom";
import {
  StorageBarChart,
  type StorageBarChartData,
} from "@/components/charts/Bar";
import { DoughnutChart } from "@/components/charts/DoughnutChart";
import { ProcessesTable } from "@/components/charts/ProcessTable";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { minOpacity } from "@/consts";
import type { StorageInfo } from "@/rspc/bindings";
import type { NameValues } from "@/types/hardwareDataType";
import { useAtom } from "jotai";
import { type JSX, useEffect } from "react";
import { useTranslation } from "react-i18next";

const InfoTable = ({ data }: { data: { [key: string]: string | number } }) => {
  const { settings } = useSettingsAtom();

  return (
    <div
      className="px-4 pt-2 pb-4 border rounded-md shadow-md bg-zinc-300 dark:bg-gray-800 dark:text-white"
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
      <table className="w-full text-left">
        <tbody>
          {Object.keys(data).map((key) => (
            <tr key={key} className="border-b border-gray-700">
              <th className="pr-4 py-2 dark:text-gray-400">{key}</th>
              <td className="py-2">{data[key]}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

const DataArea = ({
  children,
  title,
  border = true,
}: { children: React.ReactNode; title?: string; border?: boolean }) => {
  return (
    <div className="p-4">
      {border ? (
        <div className="border rounded-2xl border-zinc-400 dark:border-zinc-600">
          {title && (
            <h3 className="pt-4 pl-4 pb-2 text-xl font-bold">{title}</h3>
          )}
          <div className="px-4 pb-4">{children}</div>
        </div>
      ) : (
        <>{children}</>
      )}
    </div>
  );
};

const CPUInfo = () => {
  const { t } = useTranslation();
  const [cpuUsageHistory] = useAtom(cpuUsageHistoryAtom);
  const { hardwareInfo } = useHardwareInfoAtom();

  return (
    hardwareInfo.cpu && (
      <>
        <DoughnutChart
          chartValue={cpuUsageHistory[cpuUsageHistory.length - 1]}
          dataType={"usage"}
        />
        <InfoTable
          data={{
            [t("shared.name")]: hardwareInfo.cpu.name,
            [t("shared.vendor")]: hardwareInfo.cpu.vendor,
            [t("shared.coreCount")]: hardwareInfo.cpu.coreCount,
            [t("shared.defaultClockSpeed")]:
              `${hardwareInfo.cpu.clock} ${hardwareInfo.cpu.clockUnit}`,
          }}
        />
      </>
    )
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
    hardwareInfo.gpus && (
      <>
        <div className="flex justify-around h-[200px]">
          <DoughnutChart
            chartValue={graphicUsageHistory[graphicUsageHistory.length - 1]}
            dataType={"usage"}
          />
          {targetTemperature && (
            <DoughnutChart chartValue={targetTemperature} dataType={"temp"} />
          )}
        </div>

        {hardwareInfo.gpus.map((gpu, index) => (
          <div className="py-2" key={`${gpu.name}${index}`}>
            <InfoTable
              data={{
                [t("shared.name")]: gpu.name,
                [t("shared.vendor")]: gpu.vendorName,
                [t("shared.memorySize")]: gpu.memorySize,
                [t("shared.memorySizeDedicated")]: gpu.memorySizeDedicated,
              }}
            />
          </div>
        ))}
      </>
    )
  );
};

const MemoryInfo = () => {
  const { t } = useTranslation();
  const [memoryUsageHistory] = useAtom(memoryUsageHistoryAtom);
  const { hardwareInfo } = useHardwareInfoAtom();

  return (
    hardwareInfo.memory && (
      <>
        <DoughnutChart
          chartValue={memoryUsageHistory[memoryUsageHistory.length - 1]}
          dataType={"usage"}
        />
        <InfoTable
          data={{
            [t("shared.memoryType")]: hardwareInfo.memory.memoryType,
            [t("shared.totalMemory")]: hardwareInfo.memory.size,
            [t("shared.memoryCount")]:
              `${hardwareInfo.memory.memoryCount}/${hardwareInfo.memory.totalSlots}`,
            [t("shared.memoryClockSpeed")]:
              `${hardwareInfo.memory.clock} ${hardwareInfo.memory.clockUnit}`,
          }}
        />
      </>
    )
  );
};

const StorageDataInfo = () => {
  const { t } = useTranslation();
  const { hardwareInfo } = useHardwareInfoAtom();

  // TODO ストレージの総量・総使用量をグラフ化する

  // ドライブ名でソート
  const sortedStorage = hardwareInfo.storage.sort((a, b) =>
    a.name.localeCompare(b.name),
  );

  const chartData: StorageBarChartData[] = sortedStorage.reduce(
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
  );

  return (
    <div className="pt-2">
      <div className="flex flex-col 2xl:flex-row">
        <div className="w-full 2xl:w-1/2">
          {sortedStorage.map((storage) => {
            return (
              <div key={storage.name} className="mt-4">
                <InfoTable
                  data={{
                    [t("shared.driveName")]: storage.name,
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
          })}
        </div>
        <div className="w-full 2xl:w-1/2 mt-8 2xl:mt-0">
          <StorageBarChart
            chartData={chartData}
            unit={sortedStorage[0].sizeUnit}
          />
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
            className="mt-4 mb-2 px-4 pt-2 pb-2 border rounded-md shadow-md bg-zinc-300 dark:bg-gray-800 dark:text-white"
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
                  <div className="w-full flex items-center justify-between cursor-pointer">
                    <p>{network.description ?? "No description"}</p>
                    {/**  この部分にネットワーク使用量を表示 */}
                    <p className="text-sm text-gray-500 dark:text-gray-400 mr-2 w-24 text-left ">
                      {network.ipv4[0] ?? "No IP Address"}
                    </p>
                  </div>
                </AccordionTrigger>
                <AccordionContent>
                  <table className="w-full text-left text-base">
                    <tbody>
                      <tr className="border-b border-gray-700">
                        <th className="pr-4 py-2 dark:text-gray-400">
                          {t("shared.macAddress")}
                        </th>
                        <td className="py-2">
                          {network.macAddress ?? "No MAC Address"}
                        </td>
                      </tr>
                      <tr className="border-b border-gray-700">
                        <th className="pr-4 py-2 dark:text-gray-400">
                          {t("shared.ipv4")}
                        </th>
                        <td className="py-2">
                          {network.ipv4.map((ip) => (
                            <p key={ip}>{ip}</p>
                          ))}
                        </td>
                      </tr>
                      <tr className="border-b border-gray-700">
                        <th className="pr-4 py-2 dark:text-gray-400">
                          {t("shared.ipv4")} {t("shared.subnetMask")}
                        </th>
                        <td className="py-2">
                          {network.ipSubnet.map((subnet) => (
                            <p key={subnet}>{subnet}</p>
                          ))}
                        </td>
                      </tr>
                      <tr className="border-b border-gray-700">
                        <th className="pr-4 py-2 dark:text-gray-400">
                          {t("shared.ipv4")} {t("shared.gateway")}
                        </th>
                        <td className="py-2">
                          {network.defaultIpv4Gateway.map((gateway) => (
                            <p key={gateway}>{gateway}</p>
                          ))}
                        </td>
                      </tr>
                      <tr className="border-b border-gray-700">
                        <th className="pr-4 py-2 dark:text-gray-400">
                          {t("shared.ipv6")}
                        </th>
                        <td className="py-2">
                          {network.ipv6.map((ip) => (
                            <p key={ip}>{ip}</p>
                          ))}
                        </td>
                      </tr>
                      <tr className="border-b border-gray-700">
                        <th className="pr-4 py-2 dark:text-gray-400">
                          {t("shared.linkLocal")} {t("shared.ipv6")}{" "}
                          {t("shared.address")}
                        </th>
                        <td className="py-2">
                          {network.linkLocalIpv6.map((ip) => (
                            <p key={ip}>{ip}</p>
                          ))}
                        </td>
                      </tr>
                      <tr className="border-b border-gray-700">
                        <th className="pr-4 py-2 dark:text-gray-400">
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
  const { hardwareInfo } = useHardwareInfoAtom();
  const { init } = useHardwareInfoAtom();
  const { t } = useTranslation();

  // biome-ignore lint/correctness/useExhaustiveDependencies: <explanation>
  useEffect(() => {
    init();
  }, []);

  const hardwareInfoListLeft: { key: DataTypeKey; component: JSX.Element }[] = [
    hardwareInfo.cpu && { key: "cpu", component: <CPUInfo /> },
    hardwareInfo.memory && { key: "memory", component: <MemoryInfo /> },
    hardwareInfo.storage.length > 0 && {
      key: "storage",
      component: <StorageDataInfo />,
    },
  ].filter((x): x is { key: DataTypeKey; component: JSX.Element } => x != null);

  const hardwareInfoListRight: { key: DataTypeKey; component: JSX.Element }[] =
    [
      hardwareInfo.gpus && { key: "gpu", component: <GPUInfo /> },
      {
        key: "process",
        component: <ProcessesTable />,
      },
      {
        key: "network",
        component: <NetworkInfo />,
      },
    ].filter(
      (x): x is { key: DataTypeKey; component: JSX.Element } => x != null,
    );

  const dataAreaKey2Title: Partial<Record<DataTypeKey, string>> = {
    cpu: "CPU",
    memory: "RAM",
    storage: t("shared.storage"),
    gpu: "GPU",
    network: t("shared.network"),
  };

  return (
    <div className="flex flex-wrap gap-4">
      <div className="flex-1 flex flex-col gap-4">
        {hardwareInfoListLeft.map(({ key, component }) => (
          <DataArea key={`left-${key}`} title={dataAreaKey2Title[key]}>
            {component}
          </DataArea>
        ))}
      </div>
      <div className="flex-1 flex flex-col gap-4">
        {hardwareInfoListRight.map(({ key, component }) => (
          <DataArea
            key={`right-${key}`}
            title={dataAreaKey2Title[key]}
            border={key !== "process"}
          >
            {component}
          </DataArea>
        ))}
      </div>
    </div>
  );
};

export default Dashboard;
