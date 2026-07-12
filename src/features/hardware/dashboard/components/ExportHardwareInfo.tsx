import { ClipboardTextIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { useExportToClipboard } from "../hooks/useExportToClipboard";

export const ExportHardwareInfo = ({
  showLabel = false,
}: {
  showLabel?: boolean;
}) => {
  const { t } = useTranslation();
  const { exportToClipboard } = useExportToClipboard();

  return (
    <div className="mr-4 flex justify-end gap-3">
      <button
        onClick={exportToClipboard}
        className={cn(
          "flex items-center gap-2 rounded-lg bg-zinc-200 p-2 hover:bg-zinc-300 dark:bg-slate-800 dark:hover:bg-slate-700",
          showLabel && "px-4",
        )}
        type="button"
        aria-label={t("pages.hardware.system.copyHardwareReport")}
      >
        <ClipboardTextIcon size={32} />
        {showLabel && (
          <span>{t("pages.hardware.system.copyHardwareReport")}</span>
        )}
      </button>
    </div>
  );
};
