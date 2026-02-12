import { AlertCircle, RotateCcw } from "lucide-react";
import type { FallbackProps } from "react-error-boundary";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";

/**
 * Compact error fallback rendered inside individual dashboard widgets.
 * Displays the error message with a retry button so that a single widget
 * failure does not take down the entire dashboard.
 */
export const WidgetErrorFallback = ({
  error,
  resetErrorBoundary,
}: FallbackProps) => {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col items-center justify-center gap-3 rounded-md border border-destructive/30 bg-destructive/5 p-6 text-center">
      <AlertCircle className="size-8 text-destructive" />
      <p className="font-medium text-destructive text-sm">
        {t("pages.dashboard.widgetError.title")}
      </p>
      <p className="max-w-xs text-muted-foreground text-xs">
        {error instanceof Error ? error.message : String(error)}
      </p>
      <Button
        variant="outline"
        size="sm"
        onClick={resetErrorBoundary}
        className="gap-1.5"
      >
        <RotateCcw className="size-3.5" />
        {t("pages.dashboard.widgetError.retry")}
      </Button>
    </div>
  );
};
