import { memo } from "react";
import { useTranslation } from "react-i18next";
import { LoadFailure } from "@/components/LoadFailure";
import { useCpuCoreTemperatures } from "@/features/hardware/hooks/useCpuCoreTemperatures";

export const CpuCoreTemperatures = memo(() => {
  const { t } = useTranslation();
  const { sample, hasError, retry } = useCpuCoreTemperatures();

  if (!hasError && !sample?.readings.length) {
    return null;
  }

  return (
    <section className="space-y-2 p-4">
      <h3 className="text-sm font-semibold">
        {t("pages.performance.coreTemperatures")}
      </h3>
      {hasError ? (
        <LoadFailure message={t("shared.loadFailed")} onRetry={retry} />
      ) : (
        <dl className="grid grid-cols-[repeat(auto-fill,minmax(min(9rem,100%),1fr))] gap-2">
          {sample?.readings.map((reading) => (
            // The Windows topology supplies this representative. Processor
            // usage arrays have their own order and are never joined here.
            <div
              key={`${reading.processorGroup}:${reading.logicalProcessor}`}
              className="flex flex-wrap items-baseline justify-between gap-x-2 rounded-md bg-muted px-2 py-1 text-xs"
            >
              <dt className="text-muted-foreground">
                {t("pages.performance.coreTemperatureProcessor", {
                  processor: reading.logicalProcessor,
                })}
              </dt>
              <dd className="font-mono tabular-nums">
                {reading.temperature === null
                  ? t("shared.notAvailable")
                  : `${Math.round(reading.temperature)} °${sample.temperatureUnit}`}
              </dd>
            </div>
          ))}
        </dl>
      )}
    </section>
  );
});
