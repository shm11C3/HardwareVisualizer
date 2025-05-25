import { useTauriDialog } from "@/hooks/useTauriDialog";
import { type ProcessInfo, commands } from "@/rspc/bindings";
import { atom, useAtom, useSetAtom } from "jotai";
import { useEffect } from "react";

const processesAtom = atom<ProcessInfo[]>([]);

export const useProcessInfo = () => {
  const { error } = useTauriDialog();
  const [processes] = useAtom(processesAtom);
  const setAtom = useSetAtom(processesAtom);

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

  return processes;
};
