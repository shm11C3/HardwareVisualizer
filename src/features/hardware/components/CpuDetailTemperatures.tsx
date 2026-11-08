import { memo } from "react";
import { useTranslation } from "react-i18next";
import { LoadFailure } from "@/components/LoadFailure";
import { useCpuDetailTemperatures } from "@/features/hardware/hooks/useCpuDetailTemperatures";

export const CpuDetailTemperatures = memo(() => {
  const { t } = useTranslation();
  const { sample, hasError, retry } = useCpuDetailTemperatures();

  if (hasError) {
    return (
      <section className="p-4">
        <LoadFailure message={t("shared.loadFailed")} onRetry={retry} />
      </section>
    );
  }

  if (!sample) {
    return null;
  }

  const groups = [
    {
      title: t("pages.performance.coreTemperatures"),
      // The representative comes from the Windows topology. The usage array
      // has its own order and is never joined here.
      readings: sample.cores.map((reading) => ({
        key: `${reading.processorGroup}:${reading.logicalProcessor}`,
        label: t("pages.performance.coreTemperatureProcessor", {
          processor: reading.logicalProcessor,
        }),
        temperature: reading.temperature,
      })),
    },
    {
      title: t("pages.performance.ccdTemperatures"),
      // A fixed CCD slot has no documented mapping to an OS core identifier.
      readings: sample.ccds.map((reading) => ({
        key: `${reading.packageIndex}:${reading.ccdSlot}`,
        label: t("pages.performance.ccdTemperatureSlot", {
          slot: reading.ccdSlot,
        }),
        temperature: reading.temperature,
      })),
    },
  ];

  return groups
    .filter(({ readings }) => readings.length > 0)
    .map(({ title, readings }) => (
      <section key={title} className="space-y-2 p-4">
        <h3 className="text-sm font-semibold">{title}</h3>
        <dl className="grid grid-cols-[repeat(auto-fill,minmax(min(9rem,100%),1fr))] gap-2">
          {readings.map(({ key, label, temperature }) => (
            <div
              key={key}
              className="flex flex-wrap items-baseline justify-between gap-x-2 rounded-md bg-muted px-2 py-1 text-xs"
            >
              <dt className="text-muted-foreground">{label}</dt>
              <dd className="font-mono tabular-nums">
                {temperature === null
                  ? t("shared.notAvailable")
                  : `${Math.round(temperature)} °${sample.temperatureUnit}`}
              </dd>
            </div>
          ))}
        </dl>
      </section>
    ));
});
