import { GraphSizeSlider } from "./GraphSizeSlider";
import { GraphStyleToggle } from "./GraphStyleToggle";
import { LineChartTypeSelector } from "./LineChartTypeSelector";

export const GraphStyleSettings = () => {
  return (
    <>
      <GraphStyleToggle type="lineGraphBorder" />
      <GraphStyleToggle type="lineGraphFill" />
      <GraphStyleToggle type="lineGraphMix" />
      <GraphStyleToggle type="lineGraphShowLegend" />
      <GraphStyleToggle type="lineGraphShowScale" />
      <GraphStyleToggle type="lineGraphShowTooltip" />
      <GraphSizeSlider />
      <LineChartTypeSelector />
    </>
  );
};
