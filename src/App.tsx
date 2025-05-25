import { useEffect, useState } from "react";
import Dashboard from "./features/hardware/dashboard/Dashboard";
import ChartTemplate from "./features/hardware/usage/Usage";
import "./index.css";
import {
  useHardwareUpdater,
  useUsageUpdater,
} from "@/features/hardware/hooks/useHardwareData";
import { useErrorModalListener } from "@/hooks/useTauriEventListener";
import type { ErrorInfo, JSX } from "react";
import { ErrorBoundary } from "react-error-boundary";
import ErrorFallback from "./components/ErrorFallback";
import ScreenTemplate from "./components/shared/ScreenTemplate";
import { SideMenu } from "./features/menu/SideMenu";
import Settings from "./features/settings/Settings";
import { useSettingsAtom } from "./features/settings/hooks/useSettingsAtom";
import { useBackgroundImage } from "./hooks/useBgImage";
import { useDarkMode } from "./hooks/useDarkMode";
import type { SelectedDisplayType } from "./types/ui";
import "@/lib/i18n";
import { ChartLine, CpuIcon, Gear } from "@phosphor-icons/react";
import { useAtom } from "jotai";
import { useTranslation } from "react-i18next";
import { FullscreenExitButton } from "./components/ui/FullScreenExit";
import { useHardwareInfoAtom } from "./features/hardware/hooks/useHardwareInfoAtom";
import { Insights } from "./features/hardware/insights/Insights";
import { CpuUsages } from "./features/hardware/usage/cpu/CpuUsage";
import { displayTargetAtom } from "./features/menu/hooks/useMenu";
import { useKeydown } from "./hooks/useInputListener";
import { useTauriStore } from "./hooks/useTauriStore";

const onError = (error: Error, info: ErrorInfo) => {
  console.error("error.message", error.message);
  console.error(
    "info.componentStack:",
    info.componentStack ?? "No stack trace available",
  );
};

const Page = () => {
  const { settings, loadSettings } = useSettingsAtom();
  const { toggle } = useDarkMode();
  const { backgroundImage: nextImage, initBackgroundImage } =
    useBackgroundImage();
  const { t, i18n } = useTranslation();
  const [isDecorated, setDecorated] = useTauriStore("window_decorated", false);

  const [currentImage, setCurrentImage] = useState(nextImage);
  const [opacity, setOpacity] = useState(1);

  useErrorModalListener();
  useUsageUpdater("cpu");
  useUsageUpdater("memory");
  useUsageUpdater("gpu");
  useUsageUpdater("processors");
  useHardwareUpdater("gpu", "temp");
  useHardwareUpdater("gpu", "fan");
  const { hardwareInfo } = useHardwareInfoAtom();

  useEffect(() => {
    i18n.changeLanguage(settings.language);
  }, [settings.language, i18n]);

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

  useEffect(() => {
    loadSettings();
    initBackgroundImage();
  }, [loadSettings, initBackgroundImage]);

  const [displayTarget] = useAtom(displayTargetAtom);

  useKeydown({ isDecorated: Boolean(isDecorated), setDecorated });

  const displayTargets: Record<SelectedDisplayType, JSX.Element> = {
    dashboard: (
      <ScreenTemplate>
        <Dashboard />
      </ScreenTemplate>
    ),
    usage: <ChartTemplate />,
    cpuDetail: (
      <ScreenTemplate
        icon={<CpuIcon size={32} />}
        title={hardwareInfo.cpu?.name || "CPU"}
      >
        <CpuUsages />
      </ScreenTemplate>
    ),
    insights: (
      <ScreenTemplate
        icon={<ChartLine size={32} />}
        title={t("pages.insights.name")}
      >
        <Insights />
      </ScreenTemplate>
    ),
    settings: (
      <ScreenTemplate
        icon={<Gear size={32} />}
        title={t("pages.settings.name")}
      >
        <Settings />
      </ScreenTemplate>
    ),
  };

  return (
    <ErrorBoundary FallbackComponent={ErrorFallback} onError={onError}>
      <div className="min-h-screen bg-cover bg-zinc-200 text-gray-900 duration-300 ease-in-out dark:bg-gray-900 dark:text-white">
        <div
          className="fixed inset-0 bg-center bg-cover transition-opacity duration-500"
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
          {displayTarget ? (
            displayTargets[displayTarget]
          ) : (
            // biome-ignore lint/style/useSelfClosingElements: <explanation>
            <div className="min-h-screen bg-cover bg-zinc-200 text-gray-900 dark:bg-gray-900 dark:text-white"></div>
          )}
        </div>
      </div>
      <FullscreenExitButton
        isDecorated={Boolean(isDecorated)}
        setDecorated={setDecorated}
      />
    </ErrorBoundary>
  );
};

export default Page;
