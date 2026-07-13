import { Suspense, useCallback, useEffect, useState } from "react";
import {
  ChartTemplate,
  CpuUsages,
  Dashboard,
  Insights,
  Settings,
} from "./lazyScreens";
import "./index.css";
import type { CSSProperties, ErrorInfo, JSX } from "react";
import { ErrorBoundary } from "react-error-boundary";
import ErrorFallback from "@/components/ErrorFallback";
import { RootErrorFallback } from "@/components/RootErrorFallback";
import { CloseToTrayFirstRunDialog } from "@/components/shared/CloseToTrayFirstRunDialog";
import { ExternalComponentGuidanceDialog } from "@/components/shared/ExternalComponentGuidanceDialog";
import { NavigationRestructureNotice } from "@/components/shared/NavigationRestructureNotice";
import { useHardwareEventListener } from "@/features/hardware/hooks/useHardwareEventListener";
import { useSelectedGpuPersistence } from "@/features/hardware/hooks/useSelectedGpuPersistence";
import { useSelectedStorageDevicePersistence } from "@/features/hardware/hooks/useSelectedStorageDevicePersistence";
import { useErrorModalListener } from "@/hooks/useTauriEventListener";
import { ScreenTemplate } from "./components/shared/ScreenTemplate";
import { SideMenu } from "./features/menu/SideMenu";
import { useSettingsAtom } from "./features/settings/hooks/useSettingsAtom";
import { useBackgroundImage } from "./hooks/useBgImage";
import { useColorTheme } from "./hooks/useColorTheme";
import { useDocumentVisibilityClass } from "./hooks/useDocumentVisibilityClass";
import type { SelectedDisplayType } from "./types/ui";
import "@/lib/i18n";
import {
  ChartLineIcon,
  CpuIcon,
  GaugeIcon,
  GearIcon,
  SquaresFourIcon,
} from "@phosphor-icons/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { platform } from "@tauri-apps/plugin-os";
import { useAtom } from "jotai";
import { useTranslation } from "react-i18next";
import { clearTauriStore } from "@/lib/tauriStore";
import { cn } from "@/lib/utils";
import { FullScreenButton } from "./components/ui/FullScreenButton";
import { FullscreenExitButton } from "./components/ui/FullScreenExit";
import { useHardwareInfoAtom } from "./features/hardware/hooks/useHardwareInfoAtom";
import { displayTargetAtom } from "./features/menu/hooks/useMenu";
import { TrayWidgetFlyout } from "./features/tray/TrayWidgetFlyout";
import { AppUpdate } from "./features/updater/AppUpdate";
import { useFullScreenMode } from "./hooks/useFullScreenMode";
import { useKeydown } from "./hooks/useInputListener";
import { useTauriStore } from "./hooks/useTauriStore";
import { useTitleIconVisualSelector } from "./hooks/useTitleIconVisualSelector";

const onRootError = (error: unknown, info: ErrorInfo) => {
  console.error(
    "root.error.message",
    error instanceof Error ? error.message : String(error),
  );
  console.error(
    "root.info.componentStack:",
    info.componentStack ?? "No stack trace available",
  );
};

const onError = (error: unknown, info: ErrorInfo) => {
  console.error(
    "error.message",
    error instanceof Error ? error.message : String(error),
  );
  console.error(
    "info.componentStack:",
    info.componentStack ?? "No stack trace available",
  );
};

/** Outer wrapper to catch initialization errors */
export const App = () => {
  if (getCurrentWindowLabel() === "tray-widget-flyout") {
    return (
      <ErrorBoundary
        FallbackComponent={RootErrorFallback}
        onError={onRootError}
      >
        <TrayWidgetFlyout />
      </ErrorBoundary>
    );
  }

  return (
    <ErrorBoundary FallbackComponent={RootErrorFallback} onError={onRootError}>
      <AppContent />
    </ErrorBoundary>
  );
};

const getCurrentWindowLabel = () => {
  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
};

/** Main app content with reset-capable error boundary */
const AppContent = () => {
  const { settings, loadSettings } = useSettingsAtom();
  useColorTheme(settings.theme);
  const { backgroundImage: nextImage, initBackgroundImage } =
    useBackgroundImage();
  const { t, i18n } = useTranslation();
  const [isDecorated, setDecorated] = useTauriStore("window_decorated", true);

  const [currentImage, setCurrentImage] = useState(nextImage);
  const [opacity, setOpacity] = useState(1);
  const [settingsLoaded, setSettingsLoaded] = useState(false);

  useErrorModalListener();
  useDocumentVisibilityClass();
  useHardwareEventListener();
  useSelectedGpuPersistence();
  useSelectedStorageDevicePersistence();
  const { hardwareInfo } = useHardwareInfoAtom();

  useEffect(() => {
    i18n.changeLanguage(settings.language);
    document.documentElement.lang = settings.language;
  }, [settings.language, i18n]);

  useEffect(() => {
    document.documentElement.classList.toggle(
      "user-select-none",
      !settings.textSelectable,
    );
  }, [settings.textSelectable]);

  useEffect(() => {
    setOpacity(0);
    const fadeOutTimeout = setTimeout(() => {
      setCurrentImage(nextImage);
      setOpacity(1);
    }, 500);

    return () => clearTimeout(fadeOutTimeout);
  }, [nextImage]);

  useEffect(() => {
    let isCancelled = false;

    const initialize = async () => {
      const [didLoadSettings, backgroundResult] = await Promise.allSettled([
        loadSettings(),
        initBackgroundImage(),
      ]);

      if (!isCancelled) {
        setSettingsLoaded(
          didLoadSettings.status === "fulfilled" && didLoadSettings.value,
        );
      }

      if (backgroundResult.status === "rejected") {
        throw backgroundResult.reason;
      }
    };

    initialize();

    return () => {
      isCancelled = true;
    };
  }, [loadSettings, initBackgroundImage]);

  const [displayTarget] = useAtom(displayTargetAtom);

  const { visibleTypes } = useTitleIconVisualSelector();

  useKeydown({ isDecorated: Boolean(isDecorated), setDecorated });
  const { isFullScreen, toggleFullScreen } = useFullScreenMode();
  const isMacOS = platform() === "macos";
  const windowOpacityRatio = settings.transparentUi
    ? settings.windowOpacity / 100
    : 1;
  const transparentSurfaceOpacity = Math.min(
    70,
    Math.max(34, settings.windowOpacity * 0.64),
  );

  const handleReset = useCallback(async () => {
    try {
      await clearTauriStore();
      await loadSettings();
      await initBackgroundImage();
    } catch (error) {
      console.error("Failed to reset app state:", error);
    }
  }, [initBackgroundImage, loadSettings]);

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
    performance: (
      <ScreenTemplate
        icon={
          visibleTypes.includes("dashboard") ? (
            <GaugeIcon size={32} />
          ) : undefined
        }
        title={
          visibleTypes.includes("dashboard")
            ? t("navigation.performance")
            : undefined
        }
        enabledBurnInShift
      >
        <Dashboard />
      </ScreenTemplate>
    ),
    usage: <ChartTemplate isFullScreen={Boolean(isFullScreen)} />,
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
    <ErrorBoundary
      FallbackComponent={ErrorFallback}
      onError={onError}
      onReset={handleReset}
    >
      <div
        className={cn(
          "min-h-screen bg-background bg-cover text-foreground duration-300 ease-in-out",
          settings.transparentUi && "transparent-ui",
          settings.transparentUi && isMacOS && "native-vibrancy",
        )}
        style={
          {
            "--window-opacity-percent": `${settings.windowOpacity}%`,
            "--transparent-surface-opacity-percent": `${transparentSurfaceOpacity}%`,
            "--transparent-backdrop-blur-px": `${settings.glassBlur}px`,
          } as CSSProperties
        }
      >
        <div
          className="transparent-theme-backdrop fixed inset-0 bg-background bg-cover transition-opacity duration-300"
          style={{
            backgroundImage: "var(--background-gradient)",
            opacity: windowOpacityRatio,
          }}
        />
        <div
          className="fixed inset-0 bg-center bg-cover transition-opacity duration-500"
          style={{
            backgroundImage: `url(${currentImage})`,
            backgroundAttachment: "fixed",
            backgroundSize: "cover",
            opacity: currentImage
              ? opacity *
                (settings.backgroundImgOpacity / 100) *
                windowOpacityRatio
              : 0,
          }}
        />
        {settings.transparentUi && (
          <div className="transparent-app-backdrop pointer-events-none fixed inset-0" />
        )}
        <div className="relative z-10">
          <SideMenu
            isFullScreen={isFullScreen || false}
            navigationLayout={settings.navigationLayout}
            settingsLoaded={settingsLoaded}
          />
          <Suspense>
            {displayTarget ? (
              displayTargets[displayTarget]
            ) : (
              <div className="min-h-screen text-foreground" />
            )}
          </Suspense>
          <AppUpdate />
          <NavigationRestructureNotice settingsLoaded={settingsLoaded} />
          <CloseToTrayFirstRunDialog
            closeToTrayChoiceMade={settings.closeToTrayChoiceMade}
            settingsLoaded={settingsLoaded}
          />
          <ExternalComponentGuidanceDialog
            displayTarget={displayTarget}
            settingsLoaded={settingsLoaded}
          />
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
