import { useState } from "react";
import { SelectPeriod, SelectRange } from "./components/SnapshotForm";
import type { SnapshotPeriod, UsageRange } from "./types/snapshotType";

export const Snapshot = () => {
  const [period, setPeriod] = useState<SnapshotPeriod>({
    start: new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString(),
    end: new Date().toISOString(),
  });
  const [cpuRange, setCpuRange] = useState<UsageRange>({
    type: "cpu",
    value: [0, 100],
  });
  const [memoryRange, setMemoryRange] = useState<UsageRange>({
    type: "memory",
    value: [0, 100],
  });
  return (
    <div>
      <div className="mt-2 flex items-center justify-between gap-5">
        <div className="flex w-full gap-4">
          <div className="flex-1">
            <SelectRange
              label="CPU Usage Range"
              range={cpuRange}
              setRange={setCpuRange}
            />
          </div>
          <div className="flex-1">
            <SelectRange
              label="Memory Usage Range"
              range={memoryRange}
              setRange={setMemoryRange}
            />
          </div>
        </div>

        <SelectPeriod period={period} setPeriod={setPeriod} />
      </div>

      {/** 日付やCPU使用率の範囲を設定するUI */}
      {/** 選択された範囲のCPU使用率・メモリ使用率 */}
      {/** 選択された範囲のプロセス */}
    </div>
  );
};
