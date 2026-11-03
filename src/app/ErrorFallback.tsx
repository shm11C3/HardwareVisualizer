import type { FallbackProps } from "react-error-boundary";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";

function ErrorFallback({ error, resetErrorBoundary }: FallbackProps) {
  const { t } = useTranslation();

  return (
    <div className="bg-background text-foreground flex min-h-screen items-center justify-center">
      <div className="border-border bg-card max-w-xl space-y-4 rounded-xl border p-6 shadow-lg">
        <h2 className="text-xl font-semibold">{t("errorBoundary.title")}</h2>
        <p className="text-muted-foreground text-sm">
          {t("errorBoundary.description")}
        </p>
        <div className="flex flex-wrap gap-3">
          <Button type="button" onClick={resetErrorBoundary}>
            {t("errorBoundary.resetButton")}
          </Button>
          <Button variant="outline" asChild>
            <a
              href="https://github.com/shm11C3/HardwareVisualizer/issues/new?assignees=&labels=bug&projects=&template=bug_report.md&title=%5BBUG%5D"
              target="_blank"
              rel="noopener noreferrer"
            >
              {t("errorBoundary.reportButton")}
            </a>
          </Button>
        </div>
        <p className="text-muted-foreground text-xs">
          {t("errorBoundary.settingsJsonHint")}
        </p>
        <details className="text-muted-foreground text-xs">
          <summary className="text-foreground cursor-pointer font-medium">
            {t("errorBoundary.settingsJsonLocations.title")}
          </summary>
          <ul className="mt-2 list-disc space-y-1 pl-4">
            <li>{t("errorBoundary.settingsJsonLocations.windows")}</li>
            <li>{t("errorBoundary.settingsJsonLocations.macos")}</li>
            <li>{t("errorBoundary.settingsJsonLocations.linux")}</li>
          </ul>
        </details>
        <div className="bg-muted text-muted-foreground rounded-md p-3 text-xs">
          <p className="text-foreground mb-2 font-medium">
            {t("errorBoundary.errorDetails")}
          </p>
          <pre className="whitespace-pre-wrap">
            {error instanceof Error ? error.message : String(error)}
          </pre>
        </div>
      </div>
    </div>
  );
}
export default ErrorFallback;
