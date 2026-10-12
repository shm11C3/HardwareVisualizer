import { useTranslation } from "react-i18next";
import { ActionFailure } from "@/components/ActionFailure";
import { useSettingWriteFailure } from "@/hooks/settings/useSettingWriteFailure";
import type { SettingWriteAction } from "@/store/settings";

/**
 * The failure line for an Application Preference control. Renders nothing
 * until the last write of one of `actions` failed, then says the control was
 * restored. List exactly the action ids of the control it sits under.
 */
export const SettingWriteFailure = ({
  actions,
  className = "mt-2",
}: {
  actions: SettingWriteAction | readonly SettingWriteAction[];
  className?: string;
}) => {
  const { t } = useTranslation();
  const failed = useSettingWriteFailure(actions);

  if (!failed) return null;

  return (
    <ActionFailure
      message={t("shared.settingWriteFailed")}
      className={className}
    />
  );
};
