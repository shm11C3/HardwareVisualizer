import { type PrimitiveAtom, useSetAtom } from "jotai";
import { useEffect } from "react";
import {
  cpuFanSpeedAtom,
  cpuTempAtom,
  gpuFanSpeedAtom,
  gpuTempAtom,
} from "@/features/hardware/store/chart";
import type { ChartDataType } from "@/features/hardware/types/hardwareDataType";
import { commands, type NameValue, type Result } from "@/rspc/bindings";
import { isOk } from "@/types/result";

export const useHardwareUpdater = (
  hardType: Exclude<ChartDataType, "memory">,
  dataType: "temp" | "fan",
) => {
  type AtomActionMapping = {
    atom: PrimitiveAtom<NameValue[]>;
    action: () => Promise<Result<NameValue[], string>>;
  };

  const mapping: Record<
    Exclude<ChartDataType, "memory">,
    Record<"temp" | "fan", AtomActionMapping>
  > = {
    cpu: {
      temp: {
        atom: cpuTempAtom,
        action: () => {
          console.error("Not implemented");
          return Promise.resolve({ status: "error", error: "Not implemented" });
        },
      },
      fan: {
        atom: cpuFanSpeedAtom,
        action: () => {
          console.error("Not implemented");
          return Promise.resolve({ status: "error", error: "Not implemented" });
        },
      },
    },
    gpu: {
      fan: {
        atom: gpuFanSpeedAtom,
        action: commands.getNvidiaGpuCooler,
      },
      temp: {
        atom: gpuTempAtom,
        action: commands.getGpuTemperature,
      },
    },
  };

  const setData = useSetAtom(mapping[hardType][dataType].atom);
  const getData = mapping[hardType][dataType].action;

  useEffect(() => {
    const fetchData = async () => {
      const result = await getData();

      if (isOk(result)) {
        setData(result.data);
      }
    };

    fetchData();

    const intervalId = setInterval(async () => {
      fetchData();
    }, 10000);

    return () => clearInterval(intervalId);
  }, [setData, getData]);
};
