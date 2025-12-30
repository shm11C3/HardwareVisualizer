import { Suspense, useEffect, useState } from "react";
import {
  ChartTemplate,
  CpuUsages,
  Dashboard,
  Insights,
  Settings,
} from "./lazyScreens";
import "./index.css";
import type { ErrorInfo, JSX } from "react";
import { ErrorBoundary } from "react-error-boundary";
import {
  useHardwareUpdater,
  useUsageUpdater,
} from "@/features/hardware/hooks/useHardwareData";
import { useErrorModalListener } from "@/hooks/useTauriEventListener";
import ErrorFallback from "./components/ErrorFallback";
import { ScreenTemplate } from "./components/shared/ScreenTemplate";
import { SideMenu } from "./features/menu/SideMenu";
import { useSettingsAtom } from "./features/settings/hooks/useSettingsAtom";
import { useBackgroundImage } from "./hooks/useBgImage";
import { useColorTheme } from "./hooks/useColorTheme";
import type { SelectedDisplayType } from "./types/ui";
import "@/lib/i18n";
import {
  ChartLineIcon,
  CpuIcon,
  GearIcon,
  SquaresFourIcon,
} from "@phosphor-icons/react";
import { useAtom } from "jotai";
import { useTranslation } from "react-i18next";
import { FullScreenButton } from "./components/ui/FullScreenButton";
import { FullscreenExitButton } from "./components/ui/FullScreenExit";
import { useHardwareInfoAtom } from "./features/hardware/hooks/useHardwareInfoAtom";
import { displayTargetAtom } from "./features/menu/hooks/useMenu";
import { AppUpdate } from "./features/updater/ConfirmUpdate";
import { useFullScreenMode } from "./hooks/useFullScreenMode";
import { useKeydown } from "./hooks/useInputListener";
import { useTauriStore } from "./hooks/useTauriStore";
import { useTitleIconVisualSelector } from "./hooks/useTitleIconVisualSelector";

const onError = (error: Error, info: ErrorInfo) => {
  console.error("error.message", error.message);
  console.error(
    "info.componentStack:",
    info.componentStack ?? "No stack trace available",
  );
};

export const App = () => {
  const { settings, loadSettings } = useSettingsAtom();
  useColorTheme(settings.theme);
  const { backgroundImage: nextImage, initBackgroundImage } =
    useBackgroundImage();
  const { t, i18n } = useTranslation();
  const [isDecorated, setDecorated] = useTauriStore("window_decorated", true);

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
    document.documentElement.lang = settings.language;
  }, [settings.language, i18n]);

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

  const { visibleTypes } = useTitleIconVisualSelector();

  useKeydown({ isDecorated: Boolean(isDecorated), setDecorated });
  const { isFullScreen, toggleFullScreen } = useFullScreenMode();

  const displayTargets: Record<SelectedDisplayType, JSX.Element> = {
    dashboard: (
      <ScreenTemplate
        icon={
          visibleTypes.includes("dashboard") ? (
            <SquaresFourIcon size={32} />
          ) : undefined
        }
        title={
          visibleTypes.includes("dashboard")
            ? t("pages.dashboard.name")
            : undefined
        }
        enabledBurnInShift
      >
        <Dashboard />
      </ScreenTemplate>
    ),
    usage: <ChartTemplate />,
    cpuDetail: (
      <ScreenTemplate
        icon={
          visibleTypes.includes("cpuDetail") ? <CpuIcon size={32} /> : undefined
        }
        title={
          visibleTypes.includes("cpuDetail")
            ? hardwareInfo.cpu?.name || "CPU"
            : undefined
        }
        enabledBurnInShift
      >
        <CpuUsages />
      </ScreenTemplate>
    ),
    insights: (
      <ScreenTemplate
        icon={
          visibleTypes.includes("insights") ? (
            <ChartLineIcon size={32} />
          ) : undefined
        }
        title={
          visibleTypes.includes("insights")
            ? t("pages.insights.name")
            : undefined
        }
        enabledBurnInShift
      >
        <Insights />
      </ScreenTemplate>
    ),
    settings: (
      <ScreenTemplate
        icon={
          visibleTypes.includes("settings") ? <GearIcon size={32} /> : undefined
        }
        title={
          visibleTypes.includes("settings")
            ? t("pages.settings.name")
            : undefined
        }
      >
        <Settings />
      </ScreenTemplate>
    ),
  };

  return (
    <ErrorBoundary FallbackComponent={ErrorFallback} onError={onError}>
      <div
        className="min-h-screen bg-background bg-cover text-foreground duration-300 ease-in-out"
        style={{ backgroundImage: "var(--background-gradient)" }}
      >
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
          <SideMenu isFullScreen={isFullScreen || false} />
          <Suspense>
            {displayTarget ? (
              displayTargets[displayTarget]
            ) : (
              <div
                className="min-h-screen bg-background bg-cover text-foreground"
                style={{ backgroundImage: "var(--background-gradient)" }}
              />
            )}
          </Suspense>
          <AppUpdate />
        </div>
      </div>
      <FullscreenExitButton
        isDecorated={Boolean(isDecorated)}
        setDecorated={setDecorated}
      />
      <FullScreenButton
        isFullScreen={isFullScreen || false}
        onToggleFullScreen={toggleFullScreen}
      />
    </ErrorBoundary>
  );
};
