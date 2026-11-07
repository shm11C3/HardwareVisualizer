import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { FileTextIcon } from "lucide-react";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ActionFailure } from "@/components/ActionFailure";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { openURL } from "@/lib/openUrl";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

const postingUrl =
  "https://github.com/shm11C3/HardwareVisualizer/issues/new?template=hardware_compatibility_report.yml";

export const HardwareCompatibilityReport = ({
  icon = false,
}: {
  icon?: boolean;
}) => {
  const { t } = useTranslation();
  const trigger = useRef<HTMLButtonElement>(null);
  const [report, setReport] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<"generate" | "save" | "post" | null>(
    null,
  );

  const generate = async () => {
    setBusy(true);
    setFailure(null);
    try {
      const result = await commands.getHardwareCompatibilityReport();
      if (isError(result)) throw new Error(result.error);
      setReport(result.data);
    } catch {
      setFailure("generate");
    } finally {
      setBusy(false);
    }
  };

  const exportReport = async (action: "save" | "post") => {
    if (report === null) return;
    setBusy(true);
    setFailure(null);
    try {
      if (action === "save") {
        const result = await commands.saveHardwareCompatibilityReport(report);
        if (isError(result)) throw new Error(result.error);
      } else {
        await writeText(report);
        await openURL(postingUrl);
      }
    } catch {
      setFailure(action);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-1">
      <Button
        ref={trigger}
        type="button"
        variant="secondary"
        size={icon ? "icon" : "default"}
        disabled={busy}
        aria-haspopup="dialog"
        aria-expanded={report !== null}
        aria-label={t("hardwareCompatibilityReport.title")}
        title={t("hardwareCompatibilityReport.title")}
        onClick={() => void generate()}
      >
        <FileTextIcon aria-hidden="true" />
        {!icon && t("hardwareCompatibilityReport.title")}
      </Button>
      {failure === "generate" && (
        <ActionFailure
          message={t("hardwareCompatibilityReport.errors.generate")}
        />
      )}
      {report !== null && (
        <Dialog
          open
          onOpenChange={(open) => {
            if (!open && !busy) setReport(null);
          }}
        >
          <DialogContent
            className="max-h-[calc(100dvh-2rem)] w-[calc(100%_-_2rem)] overflow-y-auto text-foreground sm:max-w-2xl"
            onCloseAutoFocus={(event) => {
              event.preventDefault();
              trigger.current?.focus();
            }}
          >
            <DialogHeader>
              <DialogTitle>
                {t("hardwareCompatibilityReport.title")}
              </DialogTitle>
              <DialogDescription>
                {t("hardwareCompatibilityReport.description")}
              </DialogDescription>
            </DialogHeader>
            <textarea
              className="max-h-[40dvh] w-full resize-none rounded-md border border-border bg-background p-3 font-mono text-xs"
              rows={14}
              readOnly
              aria-label={t("hardwareCompatibilityReport.preview")}
              value={report}
            />
            <div className="flex flex-wrap gap-3">
              <div className="space-y-1">
                <Button
                  disabled={busy}
                  variant="outline"
                  onClick={() => void exportReport("save")}
                >
                  {t("hardwareCompatibilityReport.save")}
                </Button>
                {failure === "save" && (
                  <ActionFailure
                    message={t("hardwareCompatibilityReport.errors.save")}
                  />
                )}
              </div>
              <div className="space-y-1">
                <Button
                  disabled={busy}
                  onClick={() => void exportReport("post")}
                >
                  {t("hardwareCompatibilityReport.post")}
                </Button>
                {failure === "post" && (
                  <ActionFailure
                    message={t("hardwareCompatibilityReport.errors.post")}
                  />
                )}
              </div>
            </div>
          </DialogContent>
        </Dialog>
      )}
    </div>
  );
};
