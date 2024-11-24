import { commands } from "@/rspc/bindings";
import { useEffect } from "react";
import { useTauriDialog } from "./useTauriDialog";
import { useTauriStore } from "./useTauriStore";

export const useKeydown = () => {
  const { error } = useTauriDialog();
  const [isDecorated, setDecorated] = useTauriStore("window_decorated", false);

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
