import type { FallbackProps } from "react-error-boundary";

function ErrorFallback({ error }: FallbackProps) {
  return (
    <div>
      <h2>An unexpected error has occurred.</h2>
      <pre>{error instanceof Error ? error.message : String(error)}</pre>
    </div>
  );
}
export default ErrorFallback;
