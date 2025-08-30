import { Fullscreen, Shrink } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { Tooltip, TooltipContent, TooltipTrigger } from "./tooltip";

export interface FullScreenButtonProps {
  isFullScreen: boolean;
  onToggleFullScreen?: () => void;
}

export const FullScreenButton = ({
  isFullScreen,
  onToggleFullScreen,
}: FullScreenButtonProps) => {
  const { t } = useTranslation();
  const [hovered, setHovered] = useState(false);

  if (isFullScreen) {
    return (
      <section
        className="fixed top-0 right-0 z-50 flex h-20 w-20 items-start justify-end"
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        aria-label="Fullscreen controls"
      >
        <div
          className={cn(
            "mt-2 mr-2 transition-opacity duration-300",
            hovered ? "opacity-100" : "pointer-events-none opacity-0",
          )}
        >
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                className="cursor-pointer rounded-full bg-black/30 p-2 text-white transition-colors hover:bg-black/50"
                onClick={onToggleFullScreen}
                aria-label={t("shared.fullscreen.exit")}
              >
                <Shrink size={24} />
              </button>
            </TooltipTrigger>
            <TooltipContent>{t("shared.fullscreen.exit")}</TooltipContent>
          </Tooltip>
        </div>
      </section>
    );
  }

  return (
    <div className="fixed top-0 right-0 z-50 mt-2 mr-2">
      <Tooltip>
        <TooltipTrigger asChild>
          <button
            type="button"
            className="cursor-pointer rounded-full bg-black/30 p-2 text-white transition-colors hover:bg-black/50"
            onClick={onToggleFullScreen}
            aria-label={t("shared.fullscreen.enter")}
          >
            <Fullscreen size={24} />
          </button>
        </TooltipTrigger>
        <TooltipContent>{t("shared.fullscreen.enter")}</TooltipContent>
      </Tooltip>
    </div>
  );
};
