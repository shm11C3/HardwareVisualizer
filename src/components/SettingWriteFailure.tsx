import { useTranslation } from "react-i18next";
import { ActionFailure } from "@/components/ActionFailure";
import { useSettingWriteFailure } from "@/hooks/settings/useSettingWriteFailure";
import type { ClientSettings } from "@/rspc/bindings";

/**
 * The failure line for an Application Preference row. Renders nothing until
 * the last write of one of `keys` failed, then says the control was restored.
 */
export const SettingWriteFailure = ({
  keys,
  className = "mt-2",
}: {
  keys: keyof ClientSettings | readonly (keyof ClientSettings)[];
  className?: string;
}) => {
  const { t } = useTranslation();
  const failed = useSettingWriteFailure(keys);

  if (!failed) return null;

  return (
    <ActionFailure
      message={t("shared.settingWriteFailed")}
      className={className}
    />
  );
};
