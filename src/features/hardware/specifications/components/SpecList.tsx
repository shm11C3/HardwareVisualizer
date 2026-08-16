import { minOpacity } from "@/consts/style";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";

export type SpecListRow = { label: string; value: string | number };

/**
 * Hairline key-value rows for static facts. Values keep tabular figures so
 * numeric columns align without a table layout. Content dims with a reduced
 * background-image opacity exactly like the classic InfoTable, so both
 * screens honor the same transparency setting.
 */
export const SpecList = ({ rows }: { rows: SpecListRow[] }) => {
  const { settings } = useSettingsAtom();

  return (
    <div
      className="grid gap-x-12 md:grid-cols-2"
      style={{
        opacity:
          settings.selectedBackgroundImg != null
            ? Math.max(
                (1 - settings.backgroundImgOpacity / 100) ** 2,
                minOpacity,
              )
            : 1,
      }}
    >
      {rows.map((row) => (
        <div
          key={row.label}
          className="flex items-baseline justify-between gap-6 border-border/60 border-b py-1.5 text-sm"
        >
          <span className="shrink-0 text-muted-foreground">{row.label}</span>
          {/* min-w-0 lets break-words apply: a flex item's automatic minimum
              size is min-content, so a long serial would otherwise widen the
              row instead of wrapping. */}
          <span className="min-w-0 break-words text-right font-mono tabular-nums">
            {row.value}
          </span>
        </div>
      ))}
    </div>
  );
};
