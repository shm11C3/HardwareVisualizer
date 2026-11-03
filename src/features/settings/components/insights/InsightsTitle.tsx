import { Info } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { TypographyP } from "@/components/ui/typography";

export const InsightsTitle = () => {
  const { t } = useTranslation();
  return (
    <div className="flex items-center space-x-4 pt-3 pb-1">
      <h3 className="text-2xl font-bold">
        {t("pages.settings.insights.name")}
      </h3>
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Info />
          </TooltipTrigger>
          <TooltipContent>
            <TypographyP className="text-sm whitespace-pre-wrap">
              {t("pages.settings.insights.about.description")}
            </TypographyP>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    </div>
  );
};
