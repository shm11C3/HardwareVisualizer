import { useTauriDialog } from "@/hooks/useTauriDialog";
import { type ProcessInfo, commands } from "@/rspc/bindings";
import { CaretDown } from "@phosphor-icons/react";
import { atom, useAtom, useSetAtom } from "jotai";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const processesAtom = atom<ProcessInfo[]>([]);

const ProcessesTable = ({
  defaultItemLength,
}: { defaultItemLength: number }) => {
  const { t } = useTranslation();
  const { error } = useTauriDialog();
  const [processes] = useAtom(processesAtom);
  const setAtom = useSetAtom(processesAtom);
  const [showAllItem, setShowAllItem] = useState<boolean>(false);
  const [sortConfig, setSortConfig] = useState<{
    key: keyof ProcessInfo;
    direction: "ascending" | "descending";
  } | null>(null);

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
  }, [setAtom, error]);

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
    <div className="p-4 border rounded-md shadow-md bg-zinc-300 dark:bg-gray-800 dark:text-whit">
      <h4 className="text-xl font-bold mb-2">{t("shared.process")}</h4>
      <div className="overflow-x-auto">
        <table className="w-full text-left">
          <thead>
            <tr className="border-b border-gray-700">
              <th
                className="pr-4 py-2 dark:text-gray-400 cursor-pointer"
                onClick={() => requestSort("pid")}
                onKeyDown={() => requestSort("pid")}
              >
                PID
              </th>
              <th
                className="pr-4 py-2 dark:text-gray-400 cursor-pointer"
                onClick={() => requestSort("name")}
                onKeyDown={() => requestSort("name")}
              >
                {t("shared.name")}
              </th>
              <th
                className="pr-4 py-2 dark:text-gray-400 cursor-pointer"
                onClick={() => requestSort("cpuUsage")}
                onKeyDown={() => requestSort("cpuUsage")}
              >
                {t("shared.cpuUsage")}
              </th>
              <th
                className="pr-4 py-2 dark:text-gray-400 cursor-pointer"
                onClick={() => requestSort("memoryUsage")}
                onKeyDown={() => requestSort("memoryUsage")}
              >
                {t("shared.memoryUsage")}
              </th>
            </tr>
          </thead>
          <tbody>
            {sortedProcesses
              .slice(0, showAllItem ? processes.length : defaultItemLength)
              .map((process) => (
                <tr key={process.pid} className="border-b border-gray-700">
                  <td className="py-2">{process.pid}</td>
                  <td className="py-2">{process.name}</td>
                  <td className="py-2">{process.cpuUsage}%</td>
                  <td className="py-2">{process.memoryUsage} MB</td>
                </tr>
              ))}
          </tbody>
        </table>
        {!showAllItem && (
          <button
            type="button"
            onClick={() => setShowAllItem(true)}
            className="w-full flex justify-center items-center py-2 dark:text-gray-400 hover:text-zinc-600 dark:hover:text-white focus:outline-none mt--4"
          >
            <CaretDown size={32} />
          </button>
        )}
      </div>
    </div>
  );
};

export default ProcessesTable;
