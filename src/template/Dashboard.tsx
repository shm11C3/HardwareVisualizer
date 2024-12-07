import {
  cpuUsageHistoryAtom,
  gpuFanSpeedAtom,
  gpuTempAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
} from "@/atom/chart";
import { useHardwareInfoAtom } from "@/atom/useHardwareInfoAtom";
import {
  StorageBarChart,
  type StorageBarChartData,
} from "@/components/charts/Bar";
import DoughnutChart from "@/components/charts/DoughnutChart";
import ProcessesTable from "@/components/charts/ProcessTable";
import type { StorageInfo } from "@/rspc/bindings";
import type { NameValues } from "@/types/hardwareDataType";
import { useAtom } from "jotai";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";

const InfoTable = ({ data }: { data: { [key: string]: string | number } }) => {
  return (
    <div className="p-4 border rounded-md shadow-md bg-zinc-300 dark:bg-gray-800 dark:text-white">
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

const DataArea = ({ children }: { children: React.ReactNode }) => {
  return (
    <div className="p-4">
      <div className="border rounded-2xl border-zinc-400 dark:border-zinc-600">
        <div className="p-4">{children}</div>
      </div>
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
          chartData={cpuUsageHistory[cpuUsageHistory.length - 1]}
          dataType={"usage"}
          hardType="cpu"
          showTitle={true}
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
  const [gpuFan] = useAtom(gpuFanSpeedAtom);
  const { hardwareInfo } = useHardwareInfoAtom();

  const getTargetInfo = (data: NameValues) => {
    return data.find(
      (x) => hardwareInfo.gpus && x.name === hardwareInfo.gpus[0].name,
    )?.value;
  };

  const targetTemperature = getTargetInfo(gpuTemp);
  const targetFanSpeed = getTargetInfo(gpuFan);

  return (
    hardwareInfo.gpus && (
      <>
        <div className="flex justify-around">
          <DoughnutChart
            chartData={graphicUsageHistory[graphicUsageHistory.length - 1]}
            dataType={"usage"}
            hardType="gpu"
            showTitle={true}
          />
          {targetTemperature && (
            <DoughnutChart
              chartData={targetTemperature}
              dataType={"temp"}
              hardType="gpu"
              showTitle={false}
            />
          )}
          {targetFanSpeed && (
            <DoughnutChart
              chartData={targetFanSpeed}
              dataType={"clock"}
              hardType="gpu"
              showTitle={false}
            />
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
          chartData={memoryUsageHistory[memoryUsageHistory.length - 1]}
          dataType={"usage"}
          hardType="memory"
          showTitle={true}
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
    <div>
      <StorageBarChart chartData={chartData} unit={sortedStorage[0].sizeUnit} />
    </div>
  );
};

const Dashboard = () => {
  const { hardwareInfo } = useHardwareInfoAtom();
  const { init } = useHardwareInfoAtom();

  // biome-ignore lint/correctness/useExhaustiveDependencies: <explanation>
  useEffect(() => {
    init();
  }, []);

  const hardwareInfoList: { key: string; component: JSX.Element }[] = [
    hardwareInfo.cpu && { key: "cpuInfo", component: <CPUInfo /> },
    hardwareInfo.gpus && { key: "gpuInfo", component: <GPUInfo /> },
    hardwareInfo.memory && { key: "memoryInfo", component: <MemoryInfo /> },
    {
      key: "processesTable",
      component: <ProcessesTable defaultItemLength={6} />,
    },
    hardwareInfo.storage && hardwareInfo.storage.length > 0
      ? {
          key: "storageInfo",
          component: <StorageDataInfo />,
        }
      : undefined,
  ].filter((x) => x != null);

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
      {hardwareInfoList.map(({ key, component }) => (
        <DataArea key={key}>{component}</DataArea>
      ))}
    </div>
  );
};

export default Dashboard;
