import { ClipboardTextIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { useExportToClipboard } from "../hooks/useExportToClipboard";

export const ExportHardwareInfo = ({
  includeRuntimeStats = true,
}: {
  includeRuntimeStats?: boolean;
}) => {
  const { exportToClipboard } = useExportToClipboard({ includeRuntimeStats });
  const { t } = useTranslation();

  return (
    <div className="mr-4 flex justify-end gap-3">
      <button
        onClick={exportToClipboard}
        className="rounded-lg bg-zinc-200 p-2 hover:bg-zinc-300 dark:bg-slate-800 dark:hover:bg-slate-700"
        type="button"
        aria-label={t("pages.dashboard.copyHardwareReport")}
      >
        <ClipboardTextIcon size={32} />
      </button>
    </div>
  );
};
