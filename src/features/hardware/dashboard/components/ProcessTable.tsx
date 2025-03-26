import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { minOpacity } from "@/consts";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { type ProcessInfo, commands } from "@/rspc/bindings";
import { ArrowsOut, CaretDown, CaretUp } from "@phosphor-icons/react";
import { DialogDescription } from "@radix-ui/react-dialog";
import { VisuallyHidden } from "@radix-ui/react-visually-hidden";
import { atom, useAtom, useSetAtom } from "jotai";
import { type JSX, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { twMerge } from "tailwind-merge";
import { ScrollArea, ScrollBar } from "../../../../components/ui/scroll-area";

const processesAtom = atom<ProcessInfo[]>([]);

export const ProcessesTable = () => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const { error } = useTauriDialog();
  const [processes] = useAtom(processesAtom);
  const setAtom = useSetAtom(processesAtom);
  const [sortConfig, setSortConfig] = useState<{
    key: keyof ProcessInfo;
    direction: "ascending" | "descending";
  } | null>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: <explanation>
  useEffect(() => {
    const fetchProcesses = async () => {
      try {
        const processesData = await commands.getProcessList();
        setAtom(processesData);
      } catch (err) {
        error(err as string);
        console.error("Failed to fetch processes:", err);
      }
    };

    fetchProcesses();

    const interval = setInterval(fetchProcesses, 3000);

    return () => clearInterval(interval);
  }, []);

  const sortedProcesses = [...processes];
  if (sortConfig !== null) {
    sortedProcesses.sort((a, b) => {
      const aValue = a[sortConfig.key];
      const bValue = b[sortConfig.key];

      // 数値の場合はそのまま比較
      if (typeof aValue === "number" && typeof bValue === "number") {
        return sortConfig.direction === "ascending"
          ? aValue - bValue
          : bValue - aValue;
      }

      // 数値を文字列として保持している場合は数値に変換
      if (typeof aValue === "string" && typeof bValue === "string") {
        const aNumber = Number.parseFloat(aValue);
        const bNumber = Number.parseFloat(bValue);

        // 小数が文字列として比較されてしまうため、NaNでない場合は数値として比較
        if (!Number.isNaN(aNumber) && !Number.isNaN(bNumber)) {
          return sortConfig.direction === "ascending"
            ? aNumber - bNumber
            : bNumber - aNumber;
        }

        // 数値でない場合は文字列として比較
        return sortConfig.direction === "ascending"
          ? aValue.localeCompare(bValue)
          : bValue.localeCompare(aValue);
      }

      // 型が異なる場合は順序を変更しない
      return 0;
    });
  }

  const requestSort = (key: keyof ProcessInfo) => {
    let direction: "ascending" | "descending" = "ascending";
    if (
      sortConfig &&
      sortConfig.key === key &&
      sortConfig.direction === "ascending"
    ) {
      direction = "descending";
    }
    setSortConfig({ key, direction });
  };

  return (
    <div
      className="p-4 border rounded-md shadow-md bg-zinc-300 dark:bg-gray-800 dark:text-whit"
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
      <Dialog>
        <div className="flex">
          <h4 className="text-xl font-bold mb-2">{t("shared.process")}</h4>
          <div className="ml-auto">
            <DialogTrigger
              type="button"
              className="w-full flex justify-center items-center  dark:text-gray-400 hover:text-zinc-600 dark:hover:text-white focus:outline-hidden cursor-pointer"
            >
              <ArrowsOut size={28} />
            </DialogTrigger>
          </div>
        </div>

        <div className="overflow-x-auto">
          <InfoTable
            className="h-64"
            processes={sortedProcesses}
            sortConfig={sortConfig}
            requestSort={requestSort}
          />
        </div>

        <DialogContent className="p-4 2xl:max-w-[800px] m-8 border rounded-md shadow-md bg-zinc-300 dark:bg-gray-800 dark:text-white">
          <DialogHeader>
            <DialogTitle>{t("shared.process")}</DialogTitle>
            <DialogDescription>
              <VisuallyHidden>Expand the process as a list</VisuallyHidden>
            </DialogDescription>
          </DialogHeader>
          <InfoTable
            className="max-h-[400px] xl:max-h-[600px] 2xl:max-h-[800px]"
            processes={sortedProcesses}
            sortConfig={sortConfig}
            requestSort={requestSort}
          />
        </DialogContent>
      </Dialog>
    </div>
  );
};

const InfoTable = ({
  processes,
  sortConfig,
  requestSort,
  className,
}: {
  processes: ProcessInfo[];
  requestSort: (key: keyof ProcessInfo) => void;
  sortConfig: {
    key: keyof ProcessInfo;
    direction: "ascending" | "descending";
  } | null;
  className: string;
}) => {
  const { t } = useTranslation();

  const sortIcon: Record<"ascending" | "descending", JSX.Element> = {
    ascending: <CaretUp className="ml-1" size={18} />,
    descending: <CaretDown className="ml-1" size={18} />,
  };

  return (
    <ScrollArea className={twMerge("w-full overflow-auto", className)}>
      <table className="w-full text-left">
        <thead className="sticky h-14 top-[-1px] bg-zinc-300 dark:bg-gray-800">
          <tr className="border-b border-gray-700">
            <th
              className="pr-4 py-2 dark:text-gray-400 cursor-pointer"
              onClick={() => requestSort("pid")}
              onKeyDown={() => requestSort("pid")}
            >
              <div className="flex items-center">
                <span>PID</span>
                {sortConfig &&
                  sortConfig.key === "pid" &&
                  sortIcon[sortConfig.direction]}
              </div>
            </th>
            <th
              className="pr-4 py-2 dark:text-gray-400 cursor-pointer"
              onClick={() => requestSort("name")}
              onKeyDown={() => requestSort("name")}
            >
              <div className="flex items-center">
                <span>{t("shared.name")}</span>
                {sortConfig &&
                  sortConfig.key === "name" &&
                  sortIcon[sortConfig.direction]}
              </div>
            </th>
            <th
              className="pr-4 py-2 dark:text-gray-400 cursor-pointer"
              onClick={() => requestSort("cpuUsage")}
              onKeyDown={() => requestSort("cpuUsage")}
            >
              <div className="flex items-center">
                <span>{t("shared.cpuUsage")}</span>
                {sortConfig &&
                  sortConfig.key === "cpuUsage" &&
                  sortIcon[sortConfig.direction]}
              </div>
            </th>
            <th
              className="pr-4 py-2 dark:text-gray-400 cursor-pointer"
              onClick={() => requestSort("memoryUsage")}
              onKeyDown={() => requestSort("memoryUsage")}
            >
              <div className="flex items-center">
                <span>{t("shared.memoryUsage")}</span>
                {sortConfig &&
                  sortConfig.key === "memoryUsage" &&
                  sortIcon[sortConfig.direction]}
              </div>
            </th>
          </tr>
        </thead>
        <tbody>
          {processes.map((process) => (
            <tr key={process.pid} className="border-b border-gray-700">
              <td className="py-2">{process.pid}</td>
              <td className="py-2">{process.name}</td>
              <td className="py-2">{process.cpuUsage}%</td>
              <td className="py-2">{process.memoryUsage} MB</td>
            </tr>
          ))}
        </tbody>
      </table>
      <ScrollBar className="ml-2" orientation="horizontal" />
    </ScrollArea>
  );
};
