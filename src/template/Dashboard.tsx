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
      <div className="flex">
        <div className="w-1/2">
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
        <div className="w-1/2 mt-4">
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
  const { networkInfo, initNetwork } = useHardwareInfoAtom();

  // biome-ignore lint/correctness/useExhaustiveDependencies: <explanation>
  useEffect(() => {
    initNetwork();
  }, []);

  return (
    <>
      {networkInfo.map((network) => {
        return (
          <div key={network.description} className="mt-4">
            <InfoTable
              data={{
                description: network.description ?? "",
                mac_address: network.macAddress ?? "",
                ipv4: network.ipV4.join(", "),
                ipv6: network.ipV6.join(", "),
                linkLocalIpV6: network.linkLocalIpV6.join(", "),
                ip_subnet: network.ipSubnet.join(", "),
                gatewayV4: network.defaultIpv4Gateway.join(", "),
                gatewayV6: network.defaultIpv6Gateway.join(", "),
              }}
            />
          </div>
        );
      })}
    </>
  );
};

const Dashboard = () => {
  const { hardwareInfo } = useHardwareInfoAtom();
  const { init } = useHardwareInfoAtom();
  const { t } = useTranslation();

  // biome-ignore lint/correctness/useExhaustiveDependencies: <explanation>
  useEffect(() => {
    init();
  }, []);

  const hardwareInfoListLeft: { key: string; component: JSX.Element }[] = [
    hardwareInfo.cpu && { key: "cpuInfo", component: <CPUInfo /> },
    hardwareInfo.memory && { key: "memoryInfo", component: <MemoryInfo /> },
    hardwareInfo.storage && hardwareInfo.storage.length > 0
      ? {
          key: "storageInfo",
          component: <StorageDataInfo />,
        }
      : undefined,
  ].filter((x) => x != null);

  const hardwareInfoListRight: { key: string; component: JSX.Element }[] = [
    hardwareInfo.gpus && { key: "gpuInfo", component: <GPUInfo /> },
    {
      key: "processesTable",
      component: <ProcessesTable />,
    },
    {
      key: "networkInfo",
      component: <NetworkInfo />,
    },
  ].filter((x) => x != null);

  const dataAreaKey2Title: Record<string, string> = {
    cpuInfo: "CPU",
    memoryInfo: "RAM",
    storageInfo: t("shared.storage"),
    gpuInfo: "GPU",
  };

  return (
    <div className="flex flex-wrap gap-4">
      <div className="flex-1 flex flex-col gap-4">
        {hardwareInfoListLeft.map(({ key, component }) => (
          <DataArea key={key} title={dataAreaKey2Title[key]}>
            {component}
          </DataArea>
        ))}
      </div>
      <div className="flex-1 flex flex-col gap-4">
        {hardwareInfoListRight.map(({ key, component }) => (
          <DataArea
            key={key}
            title={dataAreaKey2Title[key]}
            border={key !== "processesTable"}
          >
            {component}
          </DataArea>
        ))}
      </div>
    </div>
  );
};

export default Dashboard;
