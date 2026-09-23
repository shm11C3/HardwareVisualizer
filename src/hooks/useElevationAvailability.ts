import { platform } from "@tauri-apps/plugin-os";
import { useEffect, useState } from "react";
import { commands, type ElevationAvailability } from "@/rspc/bindings";

/**
 * Whether this installation can run itself as administrator (#2216).
 * `null` while loading. Windows only; other platforms report "unsupported".
 */
export const useElevationAvailability = (): ElevationAvailability | null => {
  const [availability, setAvailability] =
    useState<ElevationAvailability | null>(null);

  useEffect(() => {
    if (platform() !== "windows") {
      setAvailability("unsupported");
      return;
    }

    let isCancelled = false;
    commands
      .getElevationAvailability()
      .then((value) => {
        if (!isCancelled) setAvailability(value);
      })
      .catch((err) => {
        console.error("Failed to read elevation availability:", err);
        // Fail closed like Core: without an answer, do not offer elevation.
        if (!isCancelled) setAvailability("unprotectedLocation");
      });

    return () => {
      isCancelled = true;
    };
  }, []);

  return availability;
};
