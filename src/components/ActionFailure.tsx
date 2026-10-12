import { cn } from "@/lib/utils";

/**
 * One translated line for a user action that failed, rendered directly under
 * the control that was used. The control has already rolled back; the line
 * only says why nothing changed. There is no retry button: the user uses the
 * control again. `message` must be translated copy; never pass a Rust error
 * string.
 */
export const ActionFailure = ({
  message,
  className,
}: {
  message: string;
  className?: string | undefined;
}) => (
  <p
    role="alert"
    data-testid="action-failure"
    className={cn("text-destructive text-sm", className)}
  >
    {message}
  </p>
);
