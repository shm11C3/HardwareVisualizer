import { ClipboardTextIcon, DownloadSimpleIcon } from "@phosphor-icons/react";
import { useExportToClipboard } from "../hooks/useExportToClipboard";

export const ExportHardwareInfo = () => {
  const { exportToClipboard } = useExportToClipboard();

  return (
    <div className="mr-4 flex justify-end gap-3">
      <button
        onClick={exportToClipboard}
        className="rounded-lg bg-zinc-200 p-2 hover:bg-zinc-300 dark:bg-slate-800 dark:hover:bg-slate-700"
        type="button"
      >
        <ClipboardTextIcon size={32} />
      </button>

      <button
        onClick={() => console.log("TODO")}
        className="rounded-lg bg-zinc-200 p-2 hover:bg-zinc-300 dark:bg-slate-800 dark:hover:bg-slate-700"
        type="button"
      >
        <DownloadSimpleIcon size={32} />
      </button>
    </div>
  );
};
