import { platform } from "@tauri-apps/plugin-os";
import { useEffect, useState } from "react";
import { commands, type ElevationAvailability } from "@/rspc/bindings";
import { isError } from "@/types/result";

/**
 * `unknown` means the backend could not be asked, which is not evidence about
 * the install folder; callers treat it like any other non-`available` state.
 */
export type ElevationAvailabilityState = ElevationAvailability | "unknown";

/**
 * Whether this installation can run itself as administrator (#2216).
 * `null` while loading. Windows only; other platforms report "unsupported".
 * Elevation actions should be offered only for `available`.
 */
export const useElevationAvailability =
  (): ElevationAvailabilityState | null => {
    const [availability, setAvailability] =
      useState<ElevationAvailabilityState | null>(null);

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
          if (!isCancelled) setAvailability("unknown");
        });

      return () => {
        isCancelled = true;
      };
    }, []);

    return availability;
  };

/** Whether the running process is already elevated. `null` while loading or unknown. */
export const useProcessElevated = (): boolean | null => {
  const [elevated, setElevated] = useState<boolean | null>(null);

  useEffect(() => {
    if (platform() !== "windows") {
      setElevated(false);
      return;
    }

    let isCancelled = false;
    commands
      .isProcessElevated()
      .then((result) => {
        if (isCancelled) return;
        if (isError(result)) {
          console.error("Failed to read process elevation:", result.error);
          return;
        }
        setElevated(result.data);
      })
      .catch((err) => {
        console.error("Failed to read process elevation:", err);
      });

    return () => {
      isCancelled = true;
    };
  }, []);

  return elevated;
};

/** The explanation to show when elevation is not available, if any. */
export const elevationUnavailableReasonKey = (
  availability: ElevationAvailabilityState | null,
) => {
  switch (availability) {
    case "unprotectedLocation":
      return "elevationUnavailable.reason" as const;
    case "unknown":
    case "unsupported":
      return "elevationUnavailable.unknown" as const;
    default:
      return null;
  }
};
