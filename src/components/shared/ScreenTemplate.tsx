import type { JSX } from "react";
import { cn } from "@/lib/utils";
import { BurnInShift } from "./BurnInShift";

interface ScreenTemplateProps {
  title?: string | undefined;
  icon?: JSX.Element | undefined;
  enabledBurnInShift?: boolean | undefined;
  /** The closed side rail is not rendered in full screen, so its gutter goes. */
  isFullScreen?: boolean | undefined;
  children: React.ReactNode;
}

export const ScreenTemplate: React.FC<ScreenTemplateProps> = ({
  title,
  icon,
  enabledBurnInShift = false,
  isFullScreen = false,
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
        <div className="flex items-center">
          {icon != null && icon}
          {title && (
            <h2 className="py-3 pl-2 font-bold text-3xl text-foreground">
              {title}
            </h2>
          )}
        </div>
        {children}
      </div>
    </BurnInShift>
  );
};
