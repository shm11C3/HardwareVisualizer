import { useSettingsAtom } from "@/atom/useSettingsAtom";
import { displayDataType, displayHardType } from "@/consts/chart";
import type { ChartDataType, HardwareDataType } from "@/types/hardwareDataType";
import { Lightning, Speedometer, Thermometer } from "@phosphor-icons/react";
import {
  ArcElement,
  Chart as ChartJS,
  type ChartOptions,
  Legend,
  Tooltip,
} from "chart.js";
import { Doughnut } from "react-chartjs-2";

ChartJS.register(ArcElement, Tooltip, Legend);

const DoughnutChart = ({
  chartData,
  hardType,
  dataType,
  showTitle,
}: {
  chartData: number;
  hardType: ChartDataType;
  dataType: HardwareDataType;
  showTitle: boolean;
}) => {
  const { settings } = useSettingsAtom();

  const data = {
    datasets: [
      {
        data: [chartData, 100 - chartData],
        backgroundColor: {
          light: ["#374151", "#f3f4f6"],
          dark: ["#888", "#222"],
        }[settings.theme],
        borderWidth: 0,
      },
    ],
  };

  const options: ChartOptions<"doughnut"> = {
    cutout: "85%",
    plugins: {
      tooltip: { enabled: false },
    },
  };

  const dataTypeIcons: Record<HardwareDataType, JSX.Element> = {
    usage: <Lightning className="mr-1" size={18} weight="duotone" />,
    temp: <Thermometer className="mr-1" size={18} weight="duotone" />,
    clock: <Speedometer className="mr-1" size={18} weight="duotone" />,
  };

  const dataTypeUnits: Record<HardwareDataType, string> = {
    usage: "%",
    temp: "℃",
    clock: "MHz",
  };

  return (
    <div className="p-2 w-36 relative">
      <h3 className="text-lg font-bold">
        {
          showTitle
            ? displayHardType[hardType]
            : "　" /** [TODO] タイトルはコンポーネント外のほうが使いやすそう */
        }
      </h3>
      <Doughnut data={data} options={options} />
      <div className="absolute inset-0 flex flex-col items-center justify-center">
        <span className="text-slate-800 dark:text-white text-xl font-semibold">
          {chartData}
          {dataTypeUnits[dataType]}
        </span>
      </div>
      <span className="flex justify-center mt-4 text-gray-800 dark:text-gray-400">
        {dataTypeIcons[dataType]}
        {displayDataType[dataType]}
      </span>
    </div>
  );
};

export default DoughnutChart;
