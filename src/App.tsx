import { Suspense, useEffect, useState } from "react";
import { Dashboard, ChartTemplate, CpuUsages, Insights, Settings } from "./lazyScreens";
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
import { ensureLanguage } from "@/lib/i18n";
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
import { useFullScreenMode } from "./hooks/useFullScreenMode";
import { useKeydown } from "./hooks/useInputListener";
import { useTauriStore } from "./hooks/useTauriStore";

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
  const { t } = useTranslation();
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
    ensureLanguage(settings.language as "en" | "ja");
  }, [settings.language]);

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
  const { isFullScreen, toggleFullScreen } = useFullScreenMode();

  const displayTargets: Record<SelectedDisplayType, JSX.Element> = {
    dashboard: (
      <ScreenTemplate
        icon={<SquaresFourIcon size={32} />}
        title={t("pages.dashboard.name")}
        enabledBurnInShift
      >
        <Dashboard />
      </ScreenTemplate>
    ),
    usage: <ChartTemplate />,
    cpuDetail: (
      <ScreenTemplate
        icon={<CpuIcon size={32} />}
        title={hardwareInfo.cpu?.name || "CPU"}
        enabledBurnInShift
      >
        <CpuUsages />
      </ScreenTemplate>
    ),
    insights: (
      <ScreenTemplate
        icon={<ChartLineIcon size={32} />}
        title={t("pages.insights.name")}
        enabledBurnInShift
      >
        <Insights />
      </ScreenTemplate>
    ),
    settings: (
      <ScreenTemplate
        icon={<GearIcon size={32} />}
        title={t("pages.settings.name")}
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
          <Suspense
            fallback={
              <></>
            }
          >
            {displayTarget ? (
              displayTargets[displayTarget]
            ) : (
              <div
                className="min-h-screen bg-background bg-cover text-foreground"
                style={{ backgroundImage: "var(--background-gradient)" }}
              />
            )}
          </Suspense>
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
