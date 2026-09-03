import type { SensorSupport } from "@/rspc/bindings";

export type RoutedSensorCapability = "unknown" | "present" | "absent";
export type SensorNotice = "unsupported" | "notCollected";
export type SensorNoticeScope = "both" | "power" | "fan";

/**
 * Explain a missing lane without deriving hardware support from historical
 * absence. Only Core's explicit hardware-support fact licenses "unsupported".
 */
export const resolveSensorNotice = (
  capability: RoutedSensorCapability,
  support: SensorSupport,
): SensorNotice | null => {
  if (capability !== "absent") {
    return null;
  }

  switch (support) {
    case "unsupported":
      return "unsupported";
    case "supported":
      return "notCollected";
    case "unknown":
      return null;
  }
};

export type SensorNoticeGroup = {
  notice: SensorNotice;
  scope: SensorNoticeScope;
};

/** Combine matching power/fan states while preserving mixed causes. */
export const groupSensorNotices = (
  power: SensorNotice | null,
  fan: SensorNotice | null,
): SensorNoticeGroup[] => {
  if (power == null && fan == null) {
    return [];
  }
  if (power != null && power === fan) {
    return [{ notice: power, scope: "both" }];
  }

  return [
    ...(power == null ? [] : [{ notice: power, scope: "power" as const }]),
    ...(fan == null ? [] : [{ notice: fan, scope: "fan" as const }]),
  ];
};
