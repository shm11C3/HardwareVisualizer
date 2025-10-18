import type { LibreHardwareMonitorImportSettings } from "@/rspc/bindings";

const defaultLHMSettings: Omit<LibreHardwareMonitorImportSettings, "enabled"> =
  {
    host: "localhost",
    port: 8085,
    useHttps: false,
    refreshInterval: 5,
    timeout: 3000,
    basicAuthUsername: null,
    basicAuthPassword: null,
  };

export const getDefaultLHMSettings = (
  enabled: boolean,
  options: {
    currentSettings?: LibreHardwareMonitorImportSettings | null;
  },
): LibreHardwareMonitorImportSettings => {
  const { currentSettings } = options;

  return {
    ...defaultLHMSettings,
    ...currentSettings,
    enabled,
  };
};
