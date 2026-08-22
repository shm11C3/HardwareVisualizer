import { useTranslation } from "react-i18next";
import type { GpuAdapter, LiveGpuId } from "@/features/hardware/gpuIdentity";
import { cn } from "@/lib/utils";

/**
 * The full name, except where two identical cards report the same one — then
 * the label, which carries the ordinal that tells them apart.
 */
const accessibleName = (adapter: GpuAdapter) =>
  adapter.isNameAmbiguous ? adapter.label : adapter.name;

/**
 * States which adapter the GPU instrument's readings belong to, and lets the
 * user pick another one when the machine has more than one.
 *
 * A single adapter needs no control — naming it is the whole job — so the
 * switcher only appears once there is a choice to make. The visible text is
 * the short label while the accessible name and tooltip keep the full name
 * the platform reported.
 */
export const GpuAdapterSelector = ({
  adapters,
  selectedId,
  onSelect,
  className,
}: {
  adapters: GpuAdapter[];
  selectedId: LiveGpuId | undefined;
  onSelect: (gpuId: LiveGpuId) => void;
  className?: string;
}) => {
  const { t } = useTranslation();

  if (adapters.length === 0) {
    return null;
  }

  if (adapters.length === 1) {
    const [adapter] = adapters;
    return (
      <p
        className={cn(
          "min-w-0 truncate text-muted-foreground text-xs",
          className,
        )}
        title={accessibleName(adapter)}
        data-testid="performance-gpu-adapter"
      >
        {/* The visible text is shortened to fit the card; the full name is
            still read out, since a title alone is not an accessible name. */}
        <span aria-hidden="true">{adapter.label}</span>
        <span className="sr-only">{accessibleName(adapter)}</span>
      </p>
    );
  }

  return (
    // A labelled group of toggles rather than a tablist: there is no tabpanel,
    // no roving tabindex, and no arrow-key contract here, so announcing these
    // as tabs would promise a keyboard model the control does not implement.
    <fieldset
      className={cn(
        "m-0 flex min-w-0 justify-end gap-1 border-0 p-0",
        className,
      )}
      data-testid="performance-gpu-selector"
    >
      <legend className="sr-only">{t("pages.performance.gpuSelector")}</legend>
      {adapters.map((adapter) => {
        const isSelected = adapter.id === selectedId;
        return (
          <button
            key={adapter.id}
            type="button"
            aria-pressed={isSelected}
            // The visible label is shortened to fit the card, so the full
            // name is what assistive technology announces — carrying the
            // ordinal when the platform reports the same name twice, since
            // the raw name alone would not say which card this is.
            aria-label={accessibleName(adapter)}
            title={accessibleName(adapter)}
            onClick={() => onSelect(adapter.id)}
            className={cn(
              "min-w-0 max-w-28 truncate rounded-md border px-1.5 py-0.5 text-[11px] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50",
              isSelected
                ? "border-primary bg-primary text-primary-foreground"
                : "border-border/70 bg-transparent text-muted-foreground hover:bg-muted hover:text-foreground",
            )}
          >
            {adapter.label}
          </button>
        );
      })}
    </fieldset>
  );
};
