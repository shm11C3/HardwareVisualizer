import {
  CheckCircleIcon,
  QuestionIcon,
  WarningCircleIcon,
  XCircleIcon,
} from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import type { StorageHealthStatus } from "@/rspc/bindings";

export const StorageHealthStatusIcon = ({
  status,
  size = 16,
}: {
  status: StorageHealthStatus;
  size?: number;
}) => {
  const { t } = useTranslation();
  const label = t(`pages.dashboard.storageHealth.status.${status}`);

  switch (status) {
    case "good":
      return (
        <span aria-label={label} role="img" title={label}>
          <CheckCircleIcon
            aria-hidden
            className="text-emerald-500"
            size={size}
            weight="fill"
          />
        </span>
      );
    case "warning":
      return (
        <span aria-label={label} role="img" title={label}>
          <WarningCircleIcon
            aria-hidden
            className="text-amber-500"
            size={size}
            weight="fill"
          />
        </span>
      );
    case "critical":
      return (
        <span aria-label={label} role="img" title={label}>
          <XCircleIcon
            aria-hidden
            className="text-red-500"
            size={size}
            weight="fill"
          />
        </span>
      );
    default:
      return (
        <span aria-label={label} role="img" title={label}>
          <QuestionIcon
            aria-hidden
            className="text-zinc-400"
            size={size}
            weight="fill"
          />
        </span>
      );
  }
};
