import {
  cpuUsageHistoryAtom,
  gpuFanSpeedAtom,
  gpuTempAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
} from "@/atom/chart";
import { useHardwareInfoAtom } from "@/atom/useHardwareInfoAtom";
import DoughnutChart from "@/components/charts/DoughnutChart";
import ProcessesTable from "@/components/charts/ProcessTable";
import type { NameValues } from "@/types/hardwareDataType";
import { useAtom } from "jotai";

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
            Name: hardwareInfo.cpu.name,
            Vendor: hardwareInfo.cpu.vendor,
            "Core Count": hardwareInfo.cpu.coreCount,
            "Default Clock Speed": `${hardwareInfo.cpu.clock} ${hardwareInfo.cpu.clockUnit}`,
          }}
        />
      </>
    )
  );
};

const GPUInfo = () => {
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

        <InfoTable
          data={{
            Name: hardwareInfo.gpus[0].name,
            Vendor: hardwareInfo.gpus[0].vendorName,
            "Memory Size": hardwareInfo.gpus[0].memorySize,
            "Memory Size Dedicated": hardwareInfo.gpus[0].memorySizeDedicated,
          }}
        />
      </>
    )
  );
};

const MemoryInfo = () => {
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
            "Memory Type": hardwareInfo.memory.memoryType,
            "Total Memory": hardwareInfo.memory.size,
            "Memory Count": `${hardwareInfo.memory.memoryCount}/${hardwareInfo.memory.totalSlots}`,
            "Memory Clock": `${hardwareInfo.memory.clock} ${hardwareInfo.memory.clockUnit}`,
          }}
        />
      </>
    )
  );
};

const Dashboard = () => {
  return (
    <>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 ">
        <DataArea>
          <CPUInfo />
        </DataArea>
        <DataArea>
          <GPUInfo />
        </DataArea>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 ">
        <DataArea>
          <MemoryInfo />
        </DataArea>
        <DataArea>
          <ProcessesTable defaultItemLength={6} />
        </DataArea>
      </div>
    </>
  );
};

export default Dashboard;
