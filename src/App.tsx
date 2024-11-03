import { useEffect, useState } from "react";
import Dashboard from "./template/Dashboard";
import ChartTemplate from "./template/Usage";
import "./index.css";
import { useHardwareUpdater, useUsageUpdater } from "@/hooks/useHardwareData";
import { useErrorModalListener } from "@/hooks/useTauriEventListener";
import type { ErrorInfo } from "react";
import { ErrorBoundary } from "react-error-boundary";
import { useSettingsAtom } from "./atom/useSettingsAtom";
import ErrorFallback from "./components/ErrorFallback";
import { useBackgroundImage } from "./hooks/useBgImage";
import { useDarkMode } from "./hooks/useDarkMode";
import ScreenTemplate from "./template/ScreenTemplate";
import Settings from "./template/Settings";
import SideMenu from "./template/SideMenu";
import type { SelectedDisplayType } from "./types/ui";

const onError = (error: Error, info: ErrorInfo) => {
  console.error("error.message", error.message);
  console.error(
    "info.componentStack:",
    info.componentStack ?? "No stack trace available",
  );
};

const Page = () => {
  const { settings } = useSettingsAtom();
  const { toggle } = useDarkMode();
  const { backgroundImage: nextImage } = useBackgroundImage();

  const [currentImage, setCurrentImage] = useState(nextImage);
  const [opacity, setOpacity] = useState(1);

  useErrorModalListener();
  useUsageUpdater("cpu");
  useUsageUpdater("memory");
  useUsageUpdater("gpu");
  useHardwareUpdater("gpu", "temp");
  useHardwareUpdater("gpu", "fan");

  useEffect(() => {
    if (settings.theme) {
      toggle(settings.theme === "dark");
    }
  }, [settings.theme, toggle]);

  useEffect(() => {
    setOpacity(0);
    const fadeOutTimeout = setTimeout(() => {
      setCurrentImage(nextImage);
      setOpacity(1);
    }, 500);

    return () => clearTimeout(fadeOutTimeout);
  }, [nextImage]);

  const displayTargets: Record<SelectedDisplayType, JSX.Element> = {
    dashboard: (
      <ScreenTemplate>
        <Dashboard />
      </ScreenTemplate>
    ),
    usage: <ChartTemplate />,
    settings: (
      <ScreenTemplate title="Settings">
        <Settings />
      </ScreenTemplate>
    ),
  };

  return (
    <ErrorBoundary FallbackComponent={ErrorFallback} onError={onError}>
      <div className="bg-zinc-200 dark:bg-gray-900 text-gray-900 dark:text-white min-h-screen bg-cover ease-in-out duration-300">
        <div
          className="fixed inset-0 bg-cover bg-center transition-opacity duration-500"
          style={{
            backgroundImage: `url(${currentImage})`,
            backgroundAttachment: "fixed",
            backgroundSize: "cover",
            opacity: currentImage
              ? opacity * (settings.backgroundImgOpacity / 100)
              : 0,
          }}
        />
        <div className="relative z-10">
          <SideMenu />
          {displayTargets[settings.state.display]}
        </div>
      </div>
    </ErrorBoundary>
  );
};

export default Page;
