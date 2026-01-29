import type { FallbackProps } from "react-error-boundary";

/**
 * Minimal fallback for fatal initialization errors.
 * Does not depend on Tauri Store, i18n, or other app state.
 */
export function RootErrorFallback({ error }: FallbackProps) {
  const message = error instanceof Error ? error.message : String(error);

  return (
    <div className="flex min-h-screen items-center justify-center bg-background text-foreground">
      <div className="max-w-xl space-y-4 rounded-xl border border-border bg-card p-6 shadow-lg">
        <h2 className="font-semibold text-xl">Something went wrong</h2>
        <p className="text-muted-foreground text-sm">
          The app failed to start. Please reload the app.
        </p>
        <div className="flex flex-wrap gap-3">
          <button
            type="button"
            className="inline-flex h-10 items-center justify-center rounded-md bg-[var(--primary)] px-4 py-2 font-medium text-[var(--primary-foreground)] text-sm"
            onClick={() => window.location.reload()}
          >
            Reload
          </button>
          <a
            className="inline-flex h-10 items-center justify-center rounded-md border border-[var(--border)] bg-[var(--background)] px-4 py-2 font-medium text-sm"
            href="https://github.com/shm11C3/HardwareVisualizer/issues/new?assignees=&labels=bug&projects=&template=bug_report.md&title=%5BBUG%5D"
            target="_blank"
            rel="noopener noreferrer"
          >
            Report this issue
          </a>
        </div>
        <p className="text-muted-foreground text-xs">
          If this happens repeatedly, delete settings.json and restart the app.
        </p>
        <details className="text-muted-foreground text-xs">
          <summary className="cursor-pointer font-medium text-foreground">
            settings.json locations
          </summary>
          <ul className="mt-2 list-disc space-y-1 pl-4">
            <li>Windows: AppData\Roaming\HardwareVisualizer\settings.json</li>
            <li>
              macOS: ~/Library/Application
              Support/HardwareVisualizer/settings.json
            </li>
            <li>Linux: ~/.config/HardwareVisualizer/settings.json</li>
          </ul>
        </details>
        <div className="rounded-md bg-muted p-3 text-muted-foreground text-xs">
          <p className="mb-2 font-medium text-foreground">Error details</p>
          <pre className="whitespace-pre-wrap">{message}</pre>
        </div>
      </div>
    </div>
  );
}
