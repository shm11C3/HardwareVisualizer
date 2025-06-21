import { ProcessesTable } from "@/features/hardware/dashboard/components/ProcessTable";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { cn } from "@/lib/utils";
import {
  CpuIcon,
  GraphicsCardIcon,
  HardDrivesIcon,
  MemoryIcon,
  NetworkIcon,
} from "@phosphor-icons/react";
import { type JSX, useEffect, useMemo,  } from "react";
import { useTranslation } from "react-i18next";
import { CPUInfo, GPUInfo, MemoryInfo, NetworkInfo, StorageDataInfo } from "./components/DashboardItems";


type DataTypeKey = "cpu" | "memory" | "storage" | "gpu" | "network" | "process";

export const Dashboard = () => {
  const { init, hardwareInfo } = useHardwareInfoAtom();
  const { settings } = useSettingsAtom();
  const { t } = useTranslation();

  // biome-ignore lint/correctness/useExhaustiveDependencies: <explanation>
  useEffect(() => {
    init();
  }, []);

  // 表示対象のリストをフィルタしたうえで左右交互に振り分ける
  const [hardwareInfoListLeft, hardwareInfoListRight] = useMemo(() => {
    const fullList = [
      {
        key: "cpu",
        icon: (
          <CpuIcon size={24} color={`rgb(${settings.lineGraphColor.cpu})`} />
        ),
        component: <CPUInfo />,
      },
      (hardwareInfo.gpus == null || hardwareInfo.gpus.length > 0) && {
        key: "gpu",
        icon: (
          <GraphicsCardIcon
            size={24}
            color={`rgb(${settings.lineGraphColor.gpu})`}
          />
        ),
        component: <GPUInfo />,
      },
      {
        key: "memory",
        icon: (
          <MemoryIcon
            size={24}
            color={`rgb(${settings.lineGraphColor.memory})`}
          />
        ),
        component: <MemoryInfo />,
      },
      {
        key: "process",
        component: <ProcessesTable />,
      },
      {
        key: "storage",
        icon: <HardDrivesIcon size={24} color="var(--color-storage)" />,
        component: <StorageDataInfo />,
      },
      {
        key: "network",
        icon: <NetworkIcon size={24} color="oklch(74.6% 0.16 232.661)" />,
        component: <NetworkInfo />,
      },
    ].filter(
      (
        x,
      ): x is { key: DataTypeKey; icon: JSX.Element; component: JSX.Element } =>
        Boolean(x),
    );

    return fullList.reduce<[typeof fullList, typeof fullList]>(
      ([left, right], item, index) => {
        if (index % 2 === 0) {
          left.push(item);
        } else {
          right.push(item);
        }
        return [left, right];
      },
      [[], []],
    );
  }, [hardwareInfo, settings.lineGraphColor]);

  const dataAreaKey2Title: Partial<Record<DataTypeKey, string>> = {
    cpu: "CPU",
    memory: "RAM",
    storage: t("shared.storage"),
    gpu: "GPU",
    network: t("shared.network"),
  };

  return (
    <div className="flex flex-wrap gap-4">
      <div className="flex flex-1 flex-col gap-4">
        {hardwareInfoListLeft.map(({ key, icon, component }) =>
          key !== "process" ? (
            <DataArea
              key={`left-${key}`}
              title={dataAreaKey2Title[key]}
              icon={icon}
            >
              {component}
            </DataArea>
          ) : (
            <div key={`left-${key}`} className="p-4">
              {component}
            </div>
          ),
        )}
      </div>
      <div className="flex flex-1 flex-col gap-4">
        {hardwareInfoListRight.map(({ key, icon, component }) =>
          key !== "process" ? (
            <DataArea
              key={`right-${key}`}
              title={dataAreaKey2Title[key]}
              icon={icon}
            >
              {component}
            </DataArea>
          ) : (
            <div key={`right-${key}`} className="p-4">
              {component}
            </div>
          ),
        )}
      </div>
    </div>
  );
};


const DataArea = ({
  children,
  title,
  icon,
  border = false,
  className = "rounded-2xl bg-zinc-300/50 dark:bg-slate-950/50",
}: {
  children: React.ReactNode;
  title?: string;
  icon?: JSX.Element;
  border?: boolean;
  className?: string;
}) => {
  return (
    <div className="p-4">
      {
        <div
          className={cn(
            border && "border border-zinc-400 dark:border-zinc-600",
            className,
          )}
        >
          <div className="flex items-center pt-4 pb-2 pl-4">
            {icon && <div className="mr-2 mb-0.5">{icon}</div>}
            {title && (
              <h3 className="align-middle font-bold text-xl">{title}</h3>
            )}
          </div>
          <div className="px-4 pb-4">{children}</div>
        </div>
      }
    </div>
  );
};
