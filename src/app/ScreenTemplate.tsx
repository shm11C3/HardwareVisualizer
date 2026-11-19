import type { JSX } from "react";
import { BurnInShift } from "@/components/BurnInShift";
import { cn } from "@/lib/utils";

interface ScreenTemplateProps {
  title?: string | undefined;
  icon?: JSX.Element | undefined;
  enabledBurnInShift?: boolean | undefined;
  /** The closed side rail is not rendered in full screen, so its gutter goes. */
  isFullScreen?: boolean | undefined;
  /** Right-aligned header controls; rendered even when the title is hidden. */
  actions?: React.ReactNode;
  children: React.ReactNode;
}

export const ScreenTemplate: React.FC<ScreenTemplateProps> = ({
  title,
  icon,
  enabledBurnInShift = false,
  isFullScreen = false,
  actions,
  children,
}) => {
  return (
    <BurnInShift enabled={enabledBurnInShift}>
      <div
        className={cn(
          "mx-auto w-full pt-12 pr-4 2xl:w-3/4 2xl:px-4",
          isFullScreen ? "pl-4" : "pl-16",
        )}
      >
        <div className="flex flex-wrap items-center gap-2">
          {icon != null && icon}
          {title && (
            <h2 className="py-3 pl-2 text-3xl font-bold text-foreground">
              {title}
            </h2>
          )}
          {actions != null && (
            <div className="ml-auto flex items-center gap-2">{actions}</div>
          )}
        </div>
        {children}
      </div>
    </BurnInShift>
  );
};
