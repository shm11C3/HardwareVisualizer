import type { JSX } from "react";

export type LegendItem = {
  label: string;
  icon: JSX.Element;
};

export const CustomLegend = ({
  item,
}: {
  item: LegendItem;
}) => {
  return (
    <div className="mx-6">
      <div className="flex cursor-default items-center">
        {item.icon}
        <span className="ml-2">{item.label}</span>
      </div>
    </div>
  );
};
