import { useAtomValue } from "jotai";
import { useTranslation } from "react-i18next";
import { powerDrawAtom } from "@/features/hardware/store/chart";

export const PowerPanel = () => {
  const { t } = useTranslation();
  const power = useAtomValue(powerDrawAtom);
  const readings = [
    ["cpu", power.cpuWatts],
    ["gpu", power.gpuWatts],
    ["ane", power.aneWatts],
    ["package", power.packageWatts],
  ] as const;

  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(12rem,1fr))] gap-x-8 gap-y-1 p-4 pt-2">
      {readings.map(([component, watts]) => (
        <div
          key={component}
          className="flex items-baseline justify-between gap-4 border-border/60 border-b py-1.5 text-sm"
        >
          <span className="text-muted-foreground">
            {t(`pages.performance.power.${component}`)}
          </span>
          <span className="font-mono tabular-nums">
            {watts != null ? `${watts.toFixed(1)} W` : "—"}
          </span>
        </div>
      ))}
    </div>
  );
};
