import { Progress } from "@/components/ui/progress";
import { formatBytesBigint } from "@/lib/formatter";

type Props = {
  percent: number; // 0-100
  transferredBytes?: bigint;
  totalBytes?: bigint | null;
};

export function UpdateTopBar({ percent, transferredBytes, totalBytes }: Props) {
  const p = Math.max(0, Math.min(100, percent));

  const sizeText =
    transferredBytes != null && totalBytes != null
      ? `${formatBytesBigint(transferredBytes)} / ${formatBytesBigint(totalBytes)}`
      : "";

  return (
    <div className="fixed top-0 right-0 left-0 z-50 backdrop-blur">
      <div className="mx-auto flex max-w-5xl flex-col items-center justify-center gap-2 px-4 py-3">
        <div className="flex w-full items-center justify-between gap-3">
          <div className="truncate font-medium text-sm">
            Downloading update…
          </div>
          <div className="text-right text-muted-foreground text-xs tabular-nums">
            {Math.round(p)}%
          </div>
        </div>

        <div className="w-full">
          <Progress value={p} />
        </div>

        {sizeText && (
          <div className="w-full text-center text-muted-foreground text-xs">
            {sizeText}
          </div>
        )}
      </div>
    </div>
  );
}
