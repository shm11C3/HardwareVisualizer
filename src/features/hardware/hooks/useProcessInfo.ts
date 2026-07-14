import { atom, useAtomValue, useSetAtom } from "jotai";
import { useEffect } from "react";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { commands, type ProcessInfo } from "@/rspc/bindings";

const processesAtom = atom<ProcessInfo[]>([]);
const disabledProcessesAtom = atom<ProcessInfo[]>([]);

export const useProcessInfo = ({
  enabled = true,
}: {
  enabled?: boolean;
} = {}) => {
  const { error } = useTauriDialog();
  const processes = useAtomValue(
    enabled ? processesAtom : disabledProcessesAtom,
  );
  const setAtom = useSetAtom(processesAtom);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `error` and `setAtom` are stable functions
  useEffect(() => {
    if (!enabled) {
      return;
    }

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
  }, [enabled]);

  return processes;
};
