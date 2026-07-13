import { arch, platform, version } from "@tauri-apps/plugin-os";
import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { InfoTable } from "@/components/shared/InfoTable";
import { Skeleton } from "@/components/ui/skeleton";
import {
  GPUInfo,
  MemoryInfo,
  MotherboardDataInfo,
  NetworkInfo,
  StorageDataInfo,
} from "@/features/hardware/dashboard/components/DashboardItems";
import { ExportHardwareInfo } from "@/features/hardware/dashboard/components/ExportHardwareInfo";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";

export type HardwareCategory = "gpu" | "memory" | "storage" | "system";

const CategoryPanel = ({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) => (
  <section className="rounded-2xl bg-card p-4 shadow-sm" aria-label={title}>
    <h2 className="mb-4 font-bold text-lg">{title}</h2>
    {children}
  </section>
);

const UnavailableState = ({ label }: { label: string }) => {
  const { t } = useTranslation();

  return (
    <div
      className="rounded-xl border border-border border-dashed bg-muted/30 px-5 py-8 text-center text-muted-foreground"
      role="status"
      data-testid="hardware-category-unavailable"
    >
      <p className="font-medium text-foreground">{t("shared.notAvailable")}</p>
      <p className="mt-1 text-sm">
        {t("pages.hardware.unavailable", { category: label })}
      </p>
    </div>
  );
};

const LoadingPanel = () => <Skeleton className="h-[240px] w-full rounded-xl" />;

const SimpleCategoryPanel = ({
  category,
  label,
  isLoading,
  hasData,
  maxWidthClassName,
  children,
}: {
  category: Exclude<HardwareCategory, "system">;
  label: string;
  isLoading: boolean;
  hasData: boolean;
  maxWidthClassName: string;
  children: ReactNode;
}) => (
  <div
    className={`mx-auto ${maxWidthClassName}`}
    data-testid={`hardware-category-${category}`}
  >
    <CategoryPanel title={label}>
      {isLoading ? (
        <LoadingPanel />
      ) : hasData ? (
        children
      ) : (
        <UnavailableState label={label} />
      )}
    </CategoryPanel>
  </div>
);

const SystemPlatformInfo = () => {
  const { t } = useTranslation();

  return (
    <InfoTable
      data={{
        [t("pages.hardware.system.operatingSystem")]: platform(),
        [t("pages.hardware.system.version")]: version(),
        [t("pages.hardware.system.architecture")]: arch(),
      }}
    />
  );
};

export const HardwareCategoryScreen = ({
  category,
}: {
  category: HardwareCategory;
}) => {
  const { t } = useTranslation();
  const { hardwareInfo, init } = useHardwareInfoAtom();
  const [isLoading, setIsLoading] = useState(true);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `init` is stable for the mounted category screen
  useEffect(() => {
    let isMounted = true;

    void init().finally(() => {
      if (isMounted) setIsLoading(false);
    });

    return () => {
      isMounted = false;
    };
  }, []);

  const labels: Record<HardwareCategory, string> = {
    gpu: t("navigation.hardware.gpu"),
    memory: t("navigation.hardware.memory"),
    storage: t("navigation.hardware.storage"),
    system: t("navigation.hardware.system"),
  };

  if (category === "gpu") {
    return (
      <SimpleCategoryPanel
        category="gpu"
        label={labels.gpu}
        isLoading={isLoading}
        hasData={Boolean(hardwareInfo.gpus && hardwareInfo.gpus.length > 0)}
        maxWidthClassName="max-w-5xl"
      >
        <GPUInfo />
      </SimpleCategoryPanel>
    );
  }

  if (category === "memory") {
    return (
      <SimpleCategoryPanel
        category="memory"
        label={labels.memory}
        isLoading={isLoading}
        hasData={hardwareInfo.memory != null}
        maxWidthClassName="max-w-5xl"
      >
        <MemoryInfo />
      </SimpleCategoryPanel>
    );
  }

  if (category === "storage") {
    return (
      <SimpleCategoryPanel
        category="storage"
        label={labels.storage}
        isLoading={isLoading}
        hasData={hardwareInfo.storage.length > 0}
        maxWidthClassName="max-w-6xl"
      >
        <StorageDataInfo />
      </SimpleCategoryPanel>
    );
  }

  const motherboardFallback = isLoading ? (
    <LoadingPanel />
  ) : (
    <UnavailableState label={t("shared.motherboard")} />
  );

  return (
    <div
      className="grid grid-cols-1 gap-4 xl:grid-cols-2"
      data-testid="hardware-category-system"
    >
      <CategoryPanel title={t("pages.hardware.system.platformInformation")}>
        <SystemPlatformInfo />
      </CategoryPanel>
      <CategoryPanel title={t("shared.motherboard")}>
        <MotherboardDataInfo unavailableContent={motherboardFallback} />
      </CategoryPanel>
      <CategoryPanel title={t("shared.network")}>
        <NetworkInfo
          unavailableContent={<UnavailableState label={t("shared.network")} />}
        />
      </CategoryPanel>
      <CategoryPanel title={t("pages.hardware.system.hardwareReport")}>
        <p className="mb-4 text-muted-foreground text-sm">
          {t("pages.hardware.system.hardwareReportDescription")}
        </p>
        <ExportHardwareInfo showLabel />
      </CategoryPanel>
    </div>
  );
};
