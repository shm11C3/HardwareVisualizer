import { useTranslation } from "react-i18next";
import { useLiveScalar } from "@/features/hardware/hooks/useLiveScalar";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";

export const PowerPanel = () => {
  const { t } = useTranslation();
  const cpuWatts = useLiveScalar({ kind: "power", key: "cpuWatts" });
  const gpuWatts = useLiveScalar({ kind: "power", key: "gpuWatts" });
  const aneWatts = useLiveScalar({ kind: "power", key: "aneWatts" });
  const packageWatts = useLiveScalar({ kind: "power", key: "packageWatts" });
  const { settings } = useSettingsAtom();
  const allReadings: readonly [
    "cpu" | "gpu" | "ane" | "package",
    number | null,
  ][] = [
    ["cpu", cpuWatts],
    ["gpu", gpuWatts],
    ["ane", aneWatts],
    ["package", packageWatts],
  ];
  const readings = allReadings.filter(([component]) =>
    settings.powerDisplayTargets.includes(component),
  );

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
