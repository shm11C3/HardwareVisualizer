import { useTauriDialog } from "@/hooks/useTauriDialog";
import { cn } from "@/lib/utils";
import { commands } from "@/rspc/bindings";
import { XIcon } from "@phosphor-icons/react";
import { useState } from "react";

export const FullscreenExitButton = ({
  isDecorated,
  setDecorated,
}: {
  isDecorated: boolean;
  setDecorated: (newValue: boolean) => Promise<void>;
}) => {
  const [hovered, setHovered] = useState(false);
  const { error } = useTauriDialog();

  const handleDecoration = async () => {
    try {
      await commands.setDecoration(true);
      await setDecorated(true);
    } catch (e) {
      error(e as string);
      console.error("Failed to toggle window decoration:", e);
    }
  };

  return !isDecorated ? (
    <div
      className="-translate-x-1/2 fixed top-0 left-1/2 z-10 flex h-20 w-1/4 items-start justify-center"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <div
        className={cn(
          "mt-2 transition-opacity duration-300",
          hovered ? "opacity-100" : "pointer-events-none opacity-0",
        )}
      >
        <button
          type="button"
          className="rounded-full bg-black/30 p-2 text-white hover:bg-black/50"
          onClick={handleDecoration}
          aria-label="Exit Fullscreen"
        >
          <XIcon size={24} />
        </button>
      </div>
    </div>
  ) : (
    <></>
  );
};
