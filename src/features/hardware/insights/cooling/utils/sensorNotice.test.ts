import { describe, expect, it } from "vitest";
import { groupSensorNotices, resolveSensorNotice } from "./sensorNotice";

describe("resolveSensorNotice", () => {
  it("reports hardware unsupported only from explicit support evidence", () => {
    expect(resolveSensorNotice("absent", "unsupported")).toBe("unsupported");
  });

  it("reports a supported sensor with no values in the routed period as not collected", () => {
    expect(resolveSensorNotice("absent", "supported")).toBe("notCollected");
  });

  it("does not infer a cause when hardware support is unknown", () => {
    expect(resolveSensorNotice("absent", "unknown")).toBeNull();
  });

  it.each(["present", "unknown"] as const)(
    "shows no missing-data notice when routed capability is %s",
    (capability) => {
      expect(resolveSensorNotice(capability, "unsupported")).toBeNull();
      expect(resolveSensorNotice(capability, "supported")).toBeNull();
      expect(resolveSensorNotice(capability, "unknown")).toBeNull();
    },
  );
});

describe("groupSensorNotices", () => {
  it("combines matching power and fan causes", () => {
    expect(groupSensorNotices("unsupported", "unsupported")).toEqual([
      { notice: "unsupported", scope: "both" },
    ]);
  });

  it("keeps mixed causes separate", () => {
    expect(groupSensorNotices("unsupported", "notCollected")).toEqual([
      { notice: "unsupported", scope: "power" },
      { notice: "notCollected", scope: "fan" },
    ]);
  });
});
