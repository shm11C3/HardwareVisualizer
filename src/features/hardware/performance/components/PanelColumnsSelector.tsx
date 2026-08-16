import { ColumnsIcon, RowsIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  type PerformancePanelColumns,
  performancePanelColumnOptions,
} from "../types/performanceLayout";

const columnIcons = {
  1: <RowsIcon />,
  2: <ColumnsIcon />,
} satisfies Record<PerformancePanelColumns, React.ReactNode>;

/**
 * Panel column count for the Panels view. Two is an upper bound: the grid
 * still collapses to one column when the window cannot hold two panels.
 */
export const PanelColumnsSelector = ({
  columns,
  onColumnsChange,
}: {
  columns: PerformancePanelColumns;
  onColumnsChange: (columns: PerformancePanelColumns) => void;
}) => {
  const { t } = useTranslation();

  return (
    <Tabs
      value={String(columns)}
      onValueChange={(value) =>
        onColumnsChange(Number(value) as PerformancePanelColumns)
      }
      className="min-w-0"
    >
      <TabsList
        className="h-auto justify-start"
        aria-label={t("pages.performance.panelColumns")}
      >
        {performancePanelColumnOptions.map((candidate) => (
          <TabsTrigger
            key={candidate}
            value={String(candidate)}
            className="min-h-9 px-3"
            aria-label={t(`pages.performance.columns.${candidate}`)}
          >
            {columnIcons[candidate]}
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  );
};
