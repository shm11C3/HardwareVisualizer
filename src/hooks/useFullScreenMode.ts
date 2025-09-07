import { commands } from "@/rspc/bindings";
import { useTauriStore } from "./useTauriStore";

export const useFullScreenMode = () => {
  const [isFullScreen, setIsFullScreen] = useTauriStore("isFullScreen", false);

  const toggleFullScreen = async () => {
    // If switching to fullscreen, remove window decorations
    await commands.setDecoration(Boolean(isFullScreen));
    setIsFullScreen(!isFullScreen);
  };

  return {
    isFullScreen,
    toggleFullScreen,
  };
};
