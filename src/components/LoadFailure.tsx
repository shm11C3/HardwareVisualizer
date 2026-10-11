import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

/**
 * Failure state for a panel whose read failed. It renders where the data
 * would have appeared, so it is never confused with an empty result.
 * `message` must be translated copy; never pass a Rust error string.
 */
export const LoadFailure = ({
  message,
  onRetry,
  className,
}: {
  message: string;
  onRetry?: (() => void) | undefined;
  className?: string | undefined;
}) => {
  const { t } = useTranslation();

  return (
    <div
      role="alert"
      data-testid="load-failure"
      className={cn(
        "flex h-full w-full flex-col items-center justify-center gap-2 text-center",
        className,
      )}
    >
      <span className="text-destructive text-sm">{message}</span>
      {onRetry ? (
        <Button type="button" variant="outline" size="sm" onClick={onRetry}>
          {t("shared.retry")}
        </Button>
      ) : null}
    </div>
  );
};
