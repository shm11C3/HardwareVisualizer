import { useEffect } from "react";
import { commands } from "@/rspc/bindings";
import { useTauriDialog } from "./useTauriDialog";

export const useKeydown = ({
  isDecorated,
  setDecorated,
}: {
  isDecorated: boolean;
  setDecorated: (newValue: boolean) => Promise<void>;
}) => {
  const { error } = useTauriDialog();

  useEffect(() => {
    const handleDecoration = async () => {
      try {
        await commands.setDecoration(!isDecorated);
        await setDecorated(!isDecorated);
      } catch (e) {
        error(e as string);
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
  }, [isDecorated, setDecorated, error]);
};
