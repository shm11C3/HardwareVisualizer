import { type ReactNode, useRef } from "react";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { useBurnInShift } from "@/hooks/useBurnInShift";
import { cn } from "@/lib/utils";

export const BurnInShift = ({
  enabled,
  children,
}: {
  enabled: boolean;
  children: ReactNode;
}) => {
  const shiftRef = useRef<HTMLDivElement>(null);
  const { settings } = useSettingsAtom();
  useBurnInShift(shiftRef, enabled, settings.burnInShiftOptions ?? undefined);

  const isDriftEnabled =
    enabled && settings.burnInShift && settings.burnInShiftMode === "drift";

  return (
    <div className="burnin-root">
      <div
        ref={shiftRef}
        className={cn("burnin-shift", isDriftEnabled && "burnin-drift-x")}
      >
        <div className={cn(isDriftEnabled && "burnin-drift-y")}>{children}</div>
      </div>
    </div>
  );
};
