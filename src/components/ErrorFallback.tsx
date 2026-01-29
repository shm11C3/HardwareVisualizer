import type { FallbackProps } from "react-error-boundary";
import { useTranslation } from "react-i18next";
import { Button } from "./ui/button";

function ErrorFallback({ error, resetErrorBoundary }: FallbackProps) {
  const { t } = useTranslation();

  return (
    <div className="flex min-h-screen items-center justify-center bg-background text-foreground">
      <div className="max-w-xl space-y-4 rounded-xl border border-border bg-card p-6 shadow-lg">
        <h2 className="font-semibold text-xl">{t("errorBoundary.title")}</h2>
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
          <summary className="cursor-pointer font-medium text-foreground">
            {t("errorBoundary.settingsJsonLocations.title")}
          </summary>
          <ul className="mt-2 list-disc space-y-1 pl-4">
            <li>{t("errorBoundary.settingsJsonLocations.windows")}</li>
            <li>{t("errorBoundary.settingsJsonLocations.macos")}</li>
            <li>{t("errorBoundary.settingsJsonLocations.linux")}</li>
          </ul>
        </details>
        <div className="rounded-md bg-muted p-3 text-muted-foreground text-xs">
          <p className="mb-2 font-medium text-foreground">
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
