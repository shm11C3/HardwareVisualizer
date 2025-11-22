import { DataRetentionSettings } from "./DataRetentionSettings";
import { InsightsTitle } from "./InsightsTitle";
import { InsightsToggle } from "./InsightsToggle";

export const InsightsSettings = () => {
  return (
    <div className="items-start gap-x-12 gap-y-4 p-4 xl:grid xl:grid-cols-6">
      <div className="col-span-2">
        <InsightsTitle />
        <div className="p-4">
          <InsightsToggle />
          <DataRetentionSettings />
        </div>
      </div>
    </div>
  );
};
