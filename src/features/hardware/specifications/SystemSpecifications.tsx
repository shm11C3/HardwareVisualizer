import {
  ComputerTowerIcon,
  CpuIcon,
  DesktopIcon,
  GraphicsCardIcon,
  HardDrivesIcon,
  MemoryIcon,
  NetworkIcon,
} from "@phosphor-icons/react";
import { arch, platform, version } from "@tauri-apps/plugin-os";
import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Skeleton } from "@/components/ui/skeleton";
import {
  FetchDetailButton,
  NetworkInfo,
  StorageDataInfo,
} from "@/features/hardware/dashboard/components/DashboardItems";
import { ExportHardwareInfo } from "@/features/hardware/dashboard/components/ExportHardwareInfo";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { SpecList } from "./components/SpecList";
import { SpecBadge, SpecSection } from "./components/SpecSection";

const platformLabels: Record<string, string> = {
  windows: "Windows",
  macos: "macOS",
  linux: "Linux",
};

/**
 * System Specifications sheet: static hardware and platform facts as one flat
 * list. Live readings (usage, temperatures, fan speeds) belong to the
 * Performance Tab; the sheet keeps only facts plus the daily-record Storage
 * Health summary that ships inside the storage block.
 */
export const SystemSpecifications = () => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const { hardwareInfo, init } = useHardwareInfoAtom();
  const os = useMemo(() => platform(), []);

  // biome-ignore lint/correctness/useExhaustiveDependencies: one-time static-fact fetch
  useEffect(() => {
    void init();
  }, []);

  const gpuSections = hardwareInfo.gpus?.map((gpu, index, gpus) => {
    const showCoreCount = gpu.memorySizeDedicated === "N/A" && os === "macos";
    return (
      <div key={gpu.id} className={index !== 0 ? "mt-3" : undefined}>
        {gpus.length > 1 && (
          <div className="mb-1">
            <SpecBadge>#{index + 1}</SpecBadge>
          </div>
        )}
        <SpecList
          rows={[
            { label: t("shared.name"), value: gpu.name },
            { label: t("shared.vendor"), value: gpu.vendorName },
            { label: t("shared.memorySize"), value: gpu.memorySize },
            showCoreCount
              ? { label: t("shared.coreCount"), value: gpu.coreCount ?? "N/A" }
              : {
                  label: t("shared.memorySizeDedicated"),
                  value: gpu.memorySizeDedicated,
                },
          ]}
        />
      </div>
    );
  });

  return (
    <div data-testid="system-specifications-sheet">
      <ExportHardwareInfo includeRuntimeStats={false} />

      <div className="divide-y divide-border px-1">
        <SpecSection
          icon={
            <CpuIcon size={22} color={`rgb(${settings.lineGraphColor.cpu})`} />
          }
          title="CPU"
        >
          {hardwareInfo.cpu ? (
            <SpecList
              rows={[
                { label: t("shared.name"), value: hardwareInfo.cpu.name },
                { label: t("shared.vendor"), value: hardwareInfo.cpu.vendor },
                {
                  label: t("shared.coreCount"),
                  value: hardwareInfo.cpu.coreCount,
                },
                {
                  label: t("shared.defaultClockSpeed"),
                  value: `${hardwareInfo.cpu.clock} ${hardwareInfo.cpu.clockUnit}`,
                },
              ]}
            />
          ) : (
            <Skeleton className="h-20 w-full rounded-md" />
          )}
        </SpecSection>

        {(hardwareInfo.gpus == null || hardwareInfo.gpus.length > 0) && (
          <SpecSection
            icon={
              <GraphicsCardIcon
                size={22}
                color={`rgb(${settings.lineGraphColor.gpu})`}
              />
            }
            title="GPU"
            badge={
              hardwareInfo.gpus != null && hardwareInfo.gpus.length > 1 ? (
                <SpecBadge>
                  {t("pages.dashboard.systemSpecifications.deviceCount", {
                    count: hardwareInfo.gpus.length,
                  })}
                </SpecBadge>
              ) : undefined
            }
          >
            {gpuSections ?? <Skeleton className="h-20 w-full rounded-md" />}
          </SpecSection>
        )}

        <SpecSection
          icon={
            <MemoryIcon
              size={22}
              color={`rgb(${settings.lineGraphColor.memory})`}
            />
          }
          title="RAM"
        >
          {hardwareInfo.memory ? (
            <div className="space-y-2">
              <SpecList
                rows={[
                  {
                    label: t("shared.memoryType"),
                    value: hardwareInfo.memory.memoryType,
                  },
                  {
                    label: t("shared.totalMemory"),
                    value: hardwareInfo.memory.size,
                  },
                  ...(hardwareInfo.memory.isDetailed &&
                  hardwareInfo.memory.totalSlots > 0
                    ? [
                        {
                          label: t("shared.memoryCount"),
                          value: `${hardwareInfo.memory.memoryCount}/${hardwareInfo.memory.totalSlots}`,
                        },
                      ]
                    : []),
                  ...(hardwareInfo.memory.isDetailed &&
                  hardwareInfo.memory.clock > 0
                    ? [
                        {
                          label: t("shared.memoryClockSpeed"),
                          value: `${hardwareInfo.memory.clock} ${hardwareInfo.memory.clockUnit}`,
                        },
                      ]
                    : []),
                ]}
              />
              {!hardwareInfo.memory.isDetailed && os !== "macos" && (
                <div className="flex justify-end">
                  <FetchDetailButton />
                </div>
              )}
            </div>
          ) : (
            <Skeleton className="h-20 w-full rounded-md" />
          )}
        </SpecSection>

        <SpecSection
          icon={<HardDrivesIcon size={22} color="var(--color-storage)" />}
          title={t("shared.storage")}
        >
          <StorageDataInfo />
        </SpecSection>

        {hardwareInfo.motherboard != null && (
          <SpecSection
            icon={<DesktopIcon size={22} color="oklch(70% 0.14 150)" />}
            title={t("shared.motherboard")}
          >
            <SpecList
              rows={[
                {
                  label: t("shared.manufacturer"),
                  value: hardwareInfo.motherboard.manufacturer,
                },
                {
                  label: t("shared.product"),
                  value: hardwareInfo.motherboard.product,
                },
                ...(hardwareInfo.motherboard.version
                  ? [
                      {
                        label: t("shared.version"),
                        value: hardwareInfo.motherboard.version,
                      },
                    ]
                  : []),
                {
                  label: t("shared.serialNumber"),
                  value: hardwareInfo.motherboard.serialNumber,
                },
                {
                  label: t("shared.biosVendor"),
                  value: hardwareInfo.motherboard.biosVendor,
                },
                {
                  label: t("shared.biosVersion"),
                  value: hardwareInfo.motherboard.biosVersion,
                },
                ...(hardwareInfo.motherboard.biosReleaseDate
                  ? [
                      {
                        label: t("shared.biosReleaseDate"),
                        value: hardwareInfo.motherboard.biosReleaseDate,
                      },
                    ]
                  : []),
              ]}
            />
          </SpecSection>
        )}

        <SpecSection
          icon={<ComputerTowerIcon size={22} color="oklch(75% 0.09 260)" />}
          title={t("pages.dashboard.systemSpecifications.platform")}
        >
          <SpecList
            rows={[
              {
                label: t(
                  "pages.dashboard.systemSpecifications.operatingSystem",
                ),
                value: platformLabels[os] ?? os,
              },
              {
                label: t("shared.version"),
                value: version(),
              },
              {
                label: t("pages.dashboard.systemSpecifications.architecture"),
                value: arch(),
              },
            ]}
          />
        </SpecSection>

        <SpecSection
          icon={<NetworkIcon size={22} color="oklch(74.6% 0.16 232.661)" />}
          title={t("shared.network")}
        >
          <NetworkInfo showUnavailableState />
        </SpecSection>
      </div>
    </div>
  );
};
