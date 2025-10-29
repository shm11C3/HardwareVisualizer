import type { JSX } from "react";
import { BurnInShift } from "./BurnInShift";

interface ScreenTemplateProps {
  title?: string;
  icon?: JSX.Element;
  enabledBurnInShift?: boolean;
  children: React.ReactNode;
}

export const ScreenTemplate: React.FC<ScreenTemplateProps> = ({
  title,
  icon,
  enabledBurnInShift = false,
  children,
}) => {
  return (
    <BurnInShift enabled={enabledBurnInShift}>
      <div className="mx-auto w-full pt-12 pr-4 pl-16 2xl:w-3/4 2xl:px-4">
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
