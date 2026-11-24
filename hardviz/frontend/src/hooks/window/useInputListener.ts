import { useEffect } from "react";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

export const useKeydown = ({
  isDecorated,
  setDecorated,
}: {
  isDecorated: boolean;
  setDecorated: (newValue: boolean) => Promise<void>;
}) => {
  useEffect(() => {
    const handleDecoration = async () => {
      try {
        // F11 has no control of its own on screen to report under, so a
        // failure is log-only (the fullscreen buttons report their own).
        const result = await commands.setDecoration(!isDecorated);
        if (isError(result)) {
          console.error("Failed to toggle window decoration:", result.error);
          return;
        }
        await setDecorated(!isDecorated);
      } catch (e) {
        console.error("Failed to toggle window decoration:", e);
      }
    };

    const handleKeyDown = async (event: KeyboardEvent) => {
      if (event.key === "F11") handleDecoration();
    };

    window.addEventListener("keydown", handleKeyDown, { passive: true });
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [isDecorated, setDecorated]);
};
