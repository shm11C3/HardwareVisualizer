import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useAtom } from "jotai";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useHardwareInfoAtom } from "../../hooks/useHardwareInfoAtom";
import { useProcessInfo } from "../../hooks/useProcessInfo";
import { processorsUsageHistoryAtom } from "../../store/chart";

export const useExportToClipboard = () => {
  const { hardwareInfo, networkInfo } = useHardwareInfoAtom();
  const processes = useProcessInfo();
  const [processorsUsageHistory] = useAtom(processorsUsageHistoryAtom);
  const { t } = useTranslation();

  const clipboardContent = useMemo(() => {
    const cpuInfo = hardwareInfo.cpu
      ? [
          { key: t("shared.name"), value: hardwareInfo.cpu.name },
          { key: t("shared.vendor"), value: hardwareInfo.cpu.vendor },
          { key: t("shared.coreCount"), value: hardwareInfo.cpu.coreCount },
          {
            key: t("shared.threadCount"),
            value: processorsUsageHistory[0]?.length || 0,
          },
          {
            key: t("shared.defaultClockSpeed"),
            value: `${hardwareInfo.cpu.clock} ${hardwareInfo.cpu.clockUnit}`,
          },
          { key: t("shared.processCount"), value: processes.length },
        ]
          .map(({ key, value }) => `${key}: ${value}`)
          .join("\n")
      : "No CPU information available";

    const memoryInfo = hardwareInfo.memory
      ? [
          ...[
            {
              key: t("shared.memoryType"),
              value: hardwareInfo.memory.memoryType,
            },
            {
              key: t("shared.totalMemory"),
              value: hardwareInfo.memory.size,
            },
          ],
          ...(hardwareInfo.memory.isDetailed
            ? [
                {
                  key: t("shared.memoryCount"),
                  value: `${hardwareInfo.memory.memoryCount}/${hardwareInfo.memory.totalSlots}`,
                },
                {
                  key: t("shared.memoryClockSpeed"),
                  value: `${hardwareInfo.memory.clock} ${hardwareInfo.memory.clockUnit}`,
                },
              ]
            : []),
        ]
          .map(({ key, value }) => `${key}: ${value}`)
          .join("\n")
      : "No memory information available";

    const gpuInfo = hardwareInfo.gpus
      ? hardwareInfo.gpus
          .map((gpu) => {
            return [
              { key: t("shared.name"), value: gpu.name },
              { key: t("shared.vendor"), value: gpu.vendorName },
              {
                key: t("shared.memorySize"),
                value: gpu.memorySize,
              },
              {
                key: t("shared.memorySizeDedicated"),
                value: gpu.memorySizeDedicated,
              },
            ]
              .map(({ key, value }) => `${key}: ${value}`)
              .join("\n");
          })
          .join("\n\n")
      : "No GPU information available";

    const storageInfo = hardwareInfo.storage
      .map((storage) => {
        const type = { hdd: "HDD", ssd: "SSD", other: t("shared.other") }[
          storage.storageType
        ];
        return `${storage.name} (${storage.free} ${storage.freeUnit} / ${storage.size} ${storage.sizeUnit}), ${storage.fileSystem}, ${type}`;
      })
      .join("\n");

    const networkInfoText = networkInfo.map((nw) => {
      return [
        { key: t("shared.macAddress"), value: nw.macAddress },
        { key: t("shared.ipv4"), value: nw.ipv4.join(", ") },
        { key: t("shared.subnetMask"), value: nw.ipSubnet.join(", ") },
        {
          key: t("shared.gateway"),
          value: nw.defaultIpv4Gateway.join(", "),
        },
        { key: t("shared.ipv6"), value: nw.ipv6.join(", ") },
        ...(nw.linkLocalIpv6
          ? [
              {
                key: `${t("shared.linkLocal")} ${t("shared.ipv6")} ${t("shared.address")}`,
                value: nw.linkLocalIpv6.join(", "),
              },
            ]
          : []),
        {
          key: `${t("shared.ipv6")} ${t("shared.gateway")}`,
          value: nw.defaultIpv6Gateway.join(", "),
        },
      ]
        .map(({ key, value }) => `${key}: ${value}`)
        .join("\n");
    });

    return [cpuInfo, memoryInfo, gpuInfo, storageInfo, networkInfoText].join(
      "\n\n",
    );
  }, [hardwareInfo, processorsUsageHistory, processes, networkInfo, t]);

  const exportToClipboard = async () => {
    await writeText(clipboardContent);
  };

  return { exportToClipboard };
};
